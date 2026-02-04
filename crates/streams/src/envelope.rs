use serde::Serialize;

/// A wrapper struct that adds type information to streamed data.
///
/// When serialized, this produces JSON like:
/// ```json
/// {
///     "type": "ParsedSwap",
///     "data": { ... }
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct StreamEnvelope<T: Serialize> {
    /// The type name of the data being sent
    #[serde(rename = "type")]
    pub data_type: String,
    /// The actual data payload
    pub data: T,
}

impl<T: Serialize> StreamEnvelope<T> {
    /// Create a new envelope with the given type name and data
    pub fn new(data_type: impl Into<String>, data: T) -> Self {
        Self {
            data_type: data_type.into(),
            data,
        }
    }

    /// Create a new envelope using the type's name from std::any::type_name
    /// Note: This uses the full type path, e.g. "my_crate::module::MyType"
    pub fn from_type_name(data: T) -> Self {
        let full_name = std::any::type_name::<T>();
        // Extract just the type name without the module path
        let type_name = full_name.rsplit("::").next().unwrap_or(full_name);
        Self {
            data_type: type_name.to_string(),
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestData {
        value: i32,
    }

    #[test]
    fn test_envelope_serialization() {
        let data = TestData { value: 42 };
        let envelope = StreamEnvelope::new("TestData", data);

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains(r#""type":"TestData""#));
        assert!(json.contains(r#""data":{"value":42}"#));
    }

    #[test]
    fn test_envelope_from_type_name() {
        let data = TestData { value: 42 };
        let envelope = StreamEnvelope::from_type_name(data);

        assert_eq!(envelope.data_type, "TestData");
    }
}
