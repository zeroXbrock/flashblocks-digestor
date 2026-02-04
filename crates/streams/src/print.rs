use serde::Serialize;

use crate::{envelope::StreamEnvelope, error::StreamError, r#trait::DataStream};

pub struct PrintStream;

impl DataStream for PrintStream {
    fn send<T: Serialize>(&self, data_type: &str, data: &T) -> Result<(), StreamError> {
        let envelope = StreamEnvelope::new(data_type, data);
        self.send_envelope(&envelope)
    }

    fn send_envelope<T: Serialize>(&self, envelope: &StreamEnvelope<T>) -> Result<(), StreamError> {
        match serde_json::to_string(&envelope) {
            Ok(json) => {
                println!("{}", json);
                Ok(())
            }
            Err(e) => Err(StreamError::SendError(format!(
                "Failed to serialize data: {}",
                e
            ))),
        }
    }
}
