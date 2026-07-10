use serde::Serialize;

use crate::{AxisError, AxisResult};

pub fn canonical_bytes<T: Serialize>(value: &T) -> AxisResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| AxisError::Serialization(error.to_string()))
}
