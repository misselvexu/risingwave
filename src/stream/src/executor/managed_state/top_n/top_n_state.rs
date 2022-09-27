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

use futures::{pin_mut, StreamExt};
use risingwave_common::array::Row;
use risingwave_common::util::epoch::EpochPair;
use risingwave_common::util::ordered::*;
use risingwave_storage::table::streaming_table::state_table::StateTable;
use risingwave_storage::StateStore;

use crate::executor::error::StreamExecutorResult;
use crate::executor::managed_state::iter_state_table;
use crate::executor::top_n::TopNCache;

pub struct ManagedTopNState<S: StateStore> {
    /// Relational table.
    pub(crate) state_table: StateTable<S>,
    /// For deserializing `OrderedRow`.
    ordered_row_deserializer: OrderedRowDeserializer,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TopNStateRow {
    pub ordered_key: OrderedRow,
    pub row: Row,
}

impl TopNStateRow {
    pub fn new(ordered_key: OrderedRow, row: Row) -> Self {
        Self { ordered_key, row }
    }
}

impl<S: StateStore> ManagedTopNState<S> {
    pub fn new(
        state_table: StateTable<S>,
        ordered_row_deserializer: OrderedRowDeserializer,
    ) -> Self {
        Self {
            state_table,
            ordered_row_deserializer,
        }
    }

    pub fn insert(&mut self, _key: OrderedRow, value: Row) {
        self.state_table.insert(value);
    }

    pub fn delete(&mut self, _key: &OrderedRow, value: Row) {
        self.state_table.delete(value);
    }

    fn get_topn_row(&self, row: Row) -> TopNStateRow {
        let mut datums = Vec::with_capacity(self.state_table.pk_indices().len());
        for pk_index in self.state_table.pk_indices() {
            datums.push(row[*pk_index].clone());
        }
        let pk = Row::new(datums);
        let pk_ordered = OrderedRow::new(pk, self.ordered_row_deserializer.get_order_types());
        TopNStateRow::new(pk_ordered, row)
    }

    /// This function will return the rows in the range of [`offset`, `offset` + `limit`),
    /// if `pk_prefix` is None, it will can rows from the relational table from the very beginning,
    /// else it will scan rows from the relational table begin with specific `pk_prefix`.
    /// if `num_limit` is None, it will scan with no limit.
    /// this function only used in tests
    #[cfg(test)]
    pub async fn find_range(
        &self,
        pk_prefix: Option<&Row>,
        offset: usize,
        num_limit: Option<usize>,
    ) -> StreamExecutorResult<Vec<TopNStateRow>> {
        let state_table_iter = iter_state_table(&self.state_table, pk_prefix).await?;
        pin_mut!(state_table_iter);

        // here we don't expect users to have large OFFSET.
        let (mut rows, mut stream) = if let Some(limits) = num_limit {
            (
                Vec::with_capacity(limits.min(1024)),
                state_table_iter.skip(offset).take(limits),
            )
        } else {
            (
                Vec::with_capacity(1024),
                state_table_iter.skip(offset).take(1024),
            )
        };
        while let Some(item) = stream.next().await {
            rows.push(self.get_topn_row(item?.into_owned()));
        }
        Ok(rows)
    }

    pub async fn fill_high_cache<const WITH_TIES: bool>(
        &self,
        pk_prefix: Option<&Row>,
        topn_cache: &mut TopNCache<WITH_TIES>,
        start_key: &OrderedRow,
        cache_size_limit: usize,
    ) -> StreamExecutorResult<()> {
        let cache = &mut topn_cache.high;
        let state_table_iter = iter_state_table(&self.state_table, pk_prefix).await?;
        pin_mut!(state_table_iter);
        while let Some(item) = state_table_iter.next().await {
            // Note(bugen): should first compare with start key before constructing TopNStateRow.
            let topn_row = self.get_topn_row(item?.into_owned());
            if topn_row.ordered_key <= *start_key {
                continue;
            }
            cache.insert(topn_row.ordered_key, topn_row.row);
            if cache.len() == cache_size_limit {
                break;
            }
        }
        if WITH_TIES && topn_cache.is_high_cache_full() {
            let high_last_sort_key = topn_cache
                .high
                .last_key_value()
                .unwrap()
                .0
                .prefix(topn_cache.order_by_len);
            while let Some(item) = state_table_iter.next().await {
                let topn_row = self.get_topn_row(item?.into_owned());
                if topn_row.ordered_key.prefix(topn_cache.order_by_len) == high_last_sort_key {
                    topn_cache.high.insert(topn_row.ordered_key, topn_row.row);
                } else {
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn init_topn_cache<const WITH_TIES: bool>(
        &self,
        pk_prefix: Option<&Row>,
        topn_cache: &mut TopNCache<WITH_TIES>,
    ) -> StreamExecutorResult<()> {
        assert!(topn_cache.low.is_empty());
        assert!(topn_cache.middle.is_empty());
        assert!(topn_cache.high.is_empty());

        let state_table_iter = iter_state_table(&self.state_table, pk_prefix).await?;
        pin_mut!(state_table_iter);
        if topn_cache.offset > 0 {
            while let Some(item) = state_table_iter.next().await {
                let topn_row = self.get_topn_row(item?.into_owned());
                topn_cache.low.insert(topn_row.ordered_key, topn_row.row);
                if topn_cache.low.len() == topn_cache.offset {
                    break;
                }
            }
        }

        assert!(topn_cache.limit > 0, "topn cache limit should always > 0");
        while let Some(item) = state_table_iter.next().await {
            let topn_row = self.get_topn_row(item?.into_owned());
            topn_cache.middle.insert(topn_row.ordered_key, topn_row.row);
            if topn_cache.middle.len() == topn_cache.limit {
                break;
            }
        }
        if WITH_TIES && topn_cache.is_middle_cache_full() {
            let middle_last_sort_key = topn_cache
                .middle
                .last_key_value()
                .unwrap()
                .0
                .prefix(topn_cache.order_by_len);
            while let Some(item) = state_table_iter.next().await {
                let topn_row = self.get_topn_row(item?.into_owned());
                if topn_row.ordered_key.prefix(topn_cache.order_by_len) == middle_last_sort_key {
                    topn_cache.middle.insert(topn_row.ordered_key, topn_row.row);
                } else {
                    topn_cache.high.insert(topn_row.ordered_key, topn_row.row);
                    break;
                }
            }
        }

        assert!(
            topn_cache.high_capacity > 0,
            "topn cache high_capacity should always > 0"
        );
        while !topn_cache.is_high_cache_full() && let Some(item) = state_table_iter.next().await {
            let topn_row = self.get_topn_row(item?.into_owned());
            topn_cache.high.insert(topn_row.ordered_key, topn_row.row);
        }
        if WITH_TIES && topn_cache.is_high_cache_full() {
            let high_last_sort_key = topn_cache
                .high
                .last_key_value()
                .unwrap()
                .0
                .prefix(topn_cache.order_by_len);
            while let Some(item) = state_table_iter.next().await {
                let topn_row = self.get_topn_row(item?.into_owned());
                if topn_row.ordered_key.prefix(topn_cache.order_by_len) == high_last_sort_key {
                    topn_cache.high.insert(topn_row.ordered_key, topn_row.row);
                } else {
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn flush(&mut self, epoch: EpochPair) -> StreamExecutorResult<()> {
        self.state_table.commit(epoch).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use risingwave_common::types::DataType;
    use risingwave_common::util::sort_util::OrderType;

    // use std::collections::BTreeMap;
    use super::*;
    use crate::executor::test_utils::top_n_executor::create_in_memory_state_table;
    use crate::row_nonnull;

    #[tokio::test]
    async fn test_managed_top_n_state() {
        let data_types = vec![DataType::Varchar, DataType::Int64];
        let order_types = vec![OrderType::Ascending, OrderType::Ascending];
        let state_table = {
            let mut tb = create_in_memory_state_table(
                &[DataType::Varchar, DataType::Int64],
                &[OrderType::Ascending, OrderType::Ascending],
                &[0, 1],
            );
            tb.init_epoch(EpochPair::new_test_epoch(1));
            tb
        };
        let mut managed_state = ManagedTopNState::new(
            state_table,
            OrderedRowDeserializer::new(data_types, order_types.clone()),
        );

        let row1 = row_nonnull!["abc".to_string(), 2i64];
        let row2 = row_nonnull!["abc".to_string(), 3i64];
        let row3 = row_nonnull!["abd".to_string(), 3i64];
        let row4 = row_nonnull!["ab".to_string(), 4i64];
        let rows = vec![row1, row2, row3, row4];
        let ordered_rows = rows
            .clone()
            .into_iter()
            .map(|row| OrderedRow::new(row, &order_types))
            .collect::<Vec<_>>();

        managed_state.insert(ordered_rows[3].clone(), rows[3].clone());

        // now ("ab", 4)
        let valid_rows = managed_state.find_range(None, 0, Some(1)).await.unwrap();

        assert_eq!(valid_rows.len(), 1);
        assert_eq!(valid_rows[0].ordered_key, ordered_rows[3].clone());

        managed_state.insert(ordered_rows[2].clone(), rows[2].clone());
        let valid_rows = managed_state.find_range(None, 1, Some(1)).await.unwrap();
        assert_eq!(valid_rows.len(), 1);
        assert_eq!(valid_rows[0].ordered_key, ordered_rows[2].clone());

        managed_state.insert(ordered_rows[1].clone(), rows[1].clone());

        let valid_rows = managed_state.find_range(None, 1, Some(2)).await.unwrap();
        assert_eq!(valid_rows.len(), 2);
        assert_eq!(
            valid_rows.first().unwrap().ordered_key,
            ordered_rows[1].clone()
        );
        assert_eq!(
            valid_rows.last().unwrap().ordered_key,
            ordered_rows[2].clone()
        );

        // delete ("abc", 3)
        managed_state.delete(&ordered_rows[1].clone(), rows[1].clone());

        // insert ("abc", 2)
        managed_state.insert(ordered_rows[0].clone(), rows[0].clone());

        let valid_rows = managed_state.find_range(None, 0, Some(3)).await.unwrap();

        assert_eq!(valid_rows.len(), 3);
        assert_eq!(valid_rows[0].ordered_key, ordered_rows[3].clone());
        assert_eq!(valid_rows[1].ordered_key, ordered_rows[0].clone());
        assert_eq!(valid_rows[2].ordered_key, ordered_rows[2].clone());
    }

    #[tokio::test]
    async fn test_managed_top_n_state_fill_cache() {
        let data_types = vec![DataType::Varchar, DataType::Int64];
        let order_types = vec![OrderType::Ascending, OrderType::Ascending];
        let state_table = {
            let mut tb = create_in_memory_state_table(
                &[DataType::Varchar, DataType::Int64],
                &[OrderType::Ascending, OrderType::Ascending],
                &[0, 1],
            );
            tb.init_epoch(EpochPair::new_test_epoch(1));
            tb
        };
        let mut managed_state = ManagedTopNState::new(
            state_table,
            OrderedRowDeserializer::new(data_types, order_types.clone()),
        );

        let row1 = row_nonnull!["abc".to_string(), 2i64];
        let row2 = row_nonnull!["abc".to_string(), 3i64];
        let row3 = row_nonnull!["abd".to_string(), 3i64];
        let row4 = row_nonnull!["ab".to_string(), 4i64];
        let row5 = row_nonnull!["abcd".to_string(), 5i64];
        let rows = vec![row1, row2, row3, row4, row5];

        let mut cache = TopNCache::<false>::new(1, 1, 1);
        let ordered_rows = rows
            .clone()
            .into_iter()
            .map(|row| OrderedRow::new(row, &order_types))
            .collect::<Vec<_>>();

        managed_state.insert(ordered_rows[3].clone(), rows[3].clone());
        managed_state.insert(ordered_rows[1].clone(), rows[1].clone());
        managed_state.insert(ordered_rows[2].clone(), rows[2].clone());
        managed_state.insert(ordered_rows[4].clone(), rows[4].clone());

        managed_state
            .fill_high_cache(None, &mut cache, &ordered_rows[3], 2)
            .await
            .unwrap();
        assert_eq!(cache.high.len(), 2);
        assert_eq!(cache.high.first_key_value().unwrap().0, &ordered_rows[1]);
        assert_eq!(cache.high.last_key_value().unwrap().0, &ordered_rows[4]);
    }
}
