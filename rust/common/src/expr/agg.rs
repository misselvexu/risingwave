use crate::error::{ErrorCode, Result, RwError};
use risingwave_pb::expr::agg_call::Type;
use std::convert::TryFrom;

/// Kind of aggregation function
#[derive(Debug, Clone)]
pub enum AggKind {
    Min,
    Max,
    Sum,
    Count,
    RowCount,
    Avg,
    StringAgg,
}

impl TryFrom<Type> for AggKind {
    type Error = RwError;

    fn try_from(prost: Type) -> Result<Self> {
        match prost {
            Type::Min => Ok(AggKind::Min),
            Type::Max => Ok(AggKind::Max),
            Type::Sum => Ok(AggKind::Sum),
            Type::Avg => Ok(AggKind::Avg),
            Type::Count => Ok(AggKind::Count),
            Type::StringAgg => Ok(AggKind::StringAgg),
            _ => Err(ErrorCode::InternalError("Unrecognized agg.".into()).into()),
        }
    }
}
