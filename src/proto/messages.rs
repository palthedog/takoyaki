use serde::{Deserialize, Serialize};

// Do NOT import types from crate::engine to prvent changes in engine/ affect wire format.
use super::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum TakoyakiRequest {
    /// The first message sent from the client.
    /// It must be serialized as a newline delimited JSON format
    /// (i.e. the json message must be serialized in a singele line and `'\n'` follows the message)
    /// Example:
    /// ```
    /// {"Manmenmi":{"preferred_format":"Json","name":"Ika"}}"
    /// ```
    Manmenmi(ManmenmiRequest),

    SetDeck(SetDeckRequest),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum TakoyakiResponse {
    // Only this response can be returned from the server for any type of request.
    Error(ErrorResponse),

    Manmenmi(ManmenmiResponse),

    SetDeck(SetDeckResponse),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub message: String,
}

impl ErrorResponse {
    pub fn new_timeout() -> ErrorResponse {
        ErrorResponse {
            code: ErrorCode::Timeout,
            message: String::from("Timed out"),
        }
    }
}


#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ManmenmiRequest {
    pub preferred_format: Format,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ManmenmiResponse {
    pub board: Board,
}

type CardId = u32;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SetDeckRequest {
    pub deck: Vec<CardId>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SetDeckResponse {
    pub board: Board,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let message = TakoyakiRequest::Manmenmi(ManmenmiRequest {
            preferred_format: Format::Json,
            name: String::from("Ika"),
        });
        let serialized = serde_json::to_string(&message).unwrap();
        assert_eq!(
            r#"{"Manmenmi":{"preferred_format":"Json","name":"Ika"}}"#,
            serialized
        );
        let deserialized: TakoyakiRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_serialize_enum() {
        let message = TakoyakiResponse::Error(ErrorResponse {
            code: ErrorCode::MalformedPayload,
            message: "error...".into(),
        });
        let serialized = serde_json::to_string(&message).unwrap();
        assert_eq!(
            r#"{"Error":{"code":"MalformedPayload","message":"error..."}}"#,
            serialized
        );
    }
}
