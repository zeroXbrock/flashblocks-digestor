use serde::Serialize;

use crate::{error::StreamError, r#trait::DataStream};

pub struct PrintStream;

impl<T: Serialize> DataStream<T> for PrintStream {
    fn send_message(&self, data: &T) -> Result<(), StreamError> {
        match serde_json::to_string(&data) {
            Ok(json) => {
                println!("stream output: {}", json);
                Ok(())
            }
            Err(e) => Err(StreamError::SendError(format!(
                "Failed to serialize data: {}",
                e
            ))),
        }
    }
}
