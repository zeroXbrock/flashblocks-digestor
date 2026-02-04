use serde::Serialize;

use crate::{envelope::StreamEnvelope, error::StreamError};

/// A trait to stream data to connected clients.
///
/// The trait is not generic - instead, `send` is a generic method that accepts
/// any serializable type along with a type label. This allows a single stream
/// to handle multiple different data types.
pub trait DataStream {
    /// Send data with an explicit type label.
    /// The data will be wrapped in a `StreamEnvelope` with the given type name.
    fn send<T: Serialize>(&self, data_type: &str, data: &T) -> Result<(), StreamError>;

    /// Send data using its Rust type name as the label.
    /// Convenience method that extracts the type name automatically.
    fn send_auto<T: Serialize>(&self, data: &T) -> Result<(), StreamError> {
        let full_name = std::any::type_name::<T>();
        let type_name = full_name.rsplit("::").next().unwrap_or(full_name);
        self.send(type_name, data)
    }

    /// Send a pre-constructed envelope directly.
    fn send_envelope<T: Serialize>(&self, envelope: &StreamEnvelope<T>) -> Result<(), StreamError>;
}
