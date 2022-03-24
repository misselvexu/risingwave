// Copyright 2022 Singularity Data
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::ops::RangeBounds;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use futures::Future;
use risingwave_common::error::Result;

use super::StateStoreMetrics;
use crate::storage_value::StorageValue;
use crate::{StateStore, StateStoreIter};

/// A state store wrapper for monitoring metrics.
#[derive(Clone)]
pub struct MonitoredStateStore<S> {
    inner: S,

    stats: Arc<StateStoreMetrics>,
}

impl<S> MonitoredStateStore<S> {
    pub fn new(inner: S, stats: Arc<StateStoreMetrics>) -> Self {
        Self { inner, stats }
    }
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

impl<S> MonitoredStateStore<S>
where
    S: StateStore,
{
    async fn monitored_iter<'a, I>(
        &self,
        iter: I,
    ) -> Result<<MonitoredStateStore<S> as StateStore>::Iter<'a>>
    where
        I: Future<Output = Result<S::Iter<'a>>>,
    {
        self.stats.iter_counts.inc();

        let timer = self.stats.iter_seek_latency.start_timer();
        let iter = iter.await?;
        timer.observe_duration();

        let monitored = MonitoredStateStoreIter {
            inner: iter,
            stats: self.stats.clone(),
        };
        Ok(monitored)
    }

    pub fn stats(&self) -> Arc<StateStoreMetrics> {
        self.stats.clone()
    }
}

#[async_trait]
impl<S> StateStore for MonitoredStateStore<S>
where
    S: StateStore,
{
    type Iter<'a> = MonitoredStateStoreIter<S::Iter<'a>>;

    async fn get(&self, key: &[u8], epoch: u64) -> Result<Option<StorageValue>> {
        self.stats.get_counts.inc();

        let timer = self.stats.get_duration.start_timer();
        let value = self.inner.get(key, epoch).await?;
        timer.observe_duration();

        self.stats.get_key_size.observe(key.len() as _);
        if let Some(value) = value.as_ref() {
            self.stats.get_value_size.observe(value.len() as _);
        }

        Ok(value)
    }

    async fn scan<R, B>(
        &self,
        key_range: R,
        limit: Option<usize>,
        epoch: u64,
    ) -> Result<Vec<(Bytes, StorageValue)>>
    where
        R: RangeBounds<B> + Send,
        B: AsRef<[u8]>,
    {
        self.stats.range_scan_counts.inc();

        let timer = self.stats.range_scan_duration.start_timer();
        let result = self.inner.scan(key_range, limit, epoch).await?;
        timer.observe_duration();

        self.stats
            .range_scan_size
            .observe(result.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() as _);

        Ok(result)
    }

    async fn reverse_scan<R, B>(
        &self,
        key_range: R,
        limit: Option<usize>,
        epoch: u64,
    ) -> Result<Vec<(Bytes, StorageValue)>>
    where
        R: RangeBounds<B> + Send,
        B: AsRef<[u8]>,
    {
        self.stats.range_reverse_scan_counts.inc();

        let timer = self.stats.range_reverse_scan_duration.start_timer();
        let result = self.inner.scan(key_range, limit, epoch).await?;
        timer.observe_duration();

        self.stats
            .range_reverse_scan_size
            .observe(result.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() as _);

        Ok(result)
    }

    async fn ingest_batch(
        &self,
        kv_pairs: Vec<(Bytes, Option<StorageValue>)>,
        epoch: u64,
    ) -> Result<()> {
        if kv_pairs.is_empty() {
            return Ok(());
        }

        self.stats.write_batch_counts.inc();
        self.stats
            .write_batch_tuple_counts
            .inc_by(kv_pairs.len() as _);

        let total_size = kv_pairs
            .iter()
            .map(|(k, v)| k.len() + v.as_ref().map(|v| v.len()).unwrap_or_default())
            .sum::<usize>();

        let timer = self.stats.write_batch_shared_buffer_time.start_timer();
        self.inner.ingest_batch(kv_pairs, epoch).await?;
        timer.observe_duration();

        self.stats.write_batch_size.observe(total_size as _);

        Ok(())
    }

    async fn iter<R, B>(&self, key_range: R, epoch: u64) -> Result<Self::Iter<'_>>
    where
        R: RangeBounds<B> + Send,
        B: AsRef<[u8]>,
    {
        self.monitored_iter(self.inner.iter(key_range, epoch)).await
    }

    async fn reverse_iter<R, B>(&self, key_range: R, epoch: u64) -> Result<Self::Iter<'_>>
    where
        R: RangeBounds<B> + Send,
        B: AsRef<[u8]>,
    {
        self.monitored_iter(self.inner.reverse_iter(key_range, epoch))
            .await
    }

    async fn wait_epoch(&self, epoch: u64) -> Result<()> {
        self.inner.wait_epoch(epoch).await
    }

    async fn sync(&self, epoch: Option<u64>) -> Result<()> {
        self.stats.write_shared_buffer_sync_counts.inc();
        let timer = self.stats.write_shared_buffer_sync_time.start_timer();
        self.inner.sync(epoch).await?;
        timer.observe_duration();
        Ok(())
    }

    fn monitored(self, _stats: Arc<StateStoreMetrics>) -> MonitoredStateStore<Self> {
        panic!("the state store is already monitored")
    }

    async fn replicate_batch(
        &self,
        kv_pairs: Vec<(Bytes, Option<StorageValue>)>,
        epoch: u64,
    ) -> Result<()> {
        self.inner.replicate_batch(kv_pairs, epoch).await
    }
}

/// A state store iterator wrapper for monitoring metrics.
pub struct MonitoredStateStoreIter<I> {
    inner: I,

    stats: Arc<StateStoreMetrics>,
}

#[async_trait]
impl<I> StateStoreIter for MonitoredStateStoreIter<I>
where
    I: StateStoreIter<Item = (Bytes, StorageValue)>,
{
    type Item = I::Item;

    async fn next(&mut self) -> Result<Option<Self::Item>> {
        let timer = self.stats.iter_next_latency.start_timer();
        let pair = self.inner.next().await?;
        timer.observe_duration();

        if let Some((key, value)) = pair.as_ref() {
            self.stats
                .iter_next_size
                .observe((key.len() + value.len()) as _);
        }

        Ok(pair)
    }
}
