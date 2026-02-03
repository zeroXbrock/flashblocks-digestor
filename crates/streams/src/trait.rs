use serde::Serialize;

use crate::error::StreamError;

/// A trait to stream data of type T
pub trait DataStream<T: Serialize> {
    fn send_message(&self, data: &T) -> Result<(), StreamError>;
}
