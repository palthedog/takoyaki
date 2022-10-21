// Protocol definition in serde.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    FlexBuffer,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Manmenmi {
    format: Format,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Message {
    // The first message sent from the client.
    // It must be serialized as JSON format.
    Manmenmi(Manmenmi),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let message = Message::Manmenmi(Manmenmi {
            format: Format::Json,
            name: String::from("Ika"),
        });
        let serialized = serde_json::to_string(&message).unwrap();
        assert_eq!(r#"{"Manmenmi":{"format":"Json","name":"Ika"}}"#, serialized);
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        assert_eq!(message, deserialized);
    }
}
