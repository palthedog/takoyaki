use serde::{
    Deserialize,
    Serialize,
};

// Do NOT import types from crate::engine to prvent changes in engine/ affects the wire format.
use super::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum TakoyakiRequest {
    /// The first message sent from the client.
    /// It must be serialized as a newline delimited JSON format
    /// (i.e. the json message must be serialized in a single line and `'\n'` follows the message)
    /// Example:
    /// ```
    /// r#"{"Manmenmi":{"preferred_format":"Json","name":"Ika"}}\n"#;
    /// ```
    Manmenmi(ManmenmiRequest),

    JoinGame(JoinGameRequest),

    AcceptHands(AcceptHandsRequest),

    SelectAction(SelectActionRequest),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum TakoyakiResponse {
    // Only this response can be returned from the server for any type of request.
    Error(ErrorResponse),

    Manmenmi(ManmenmiResponse),

    JoinGame(JoinGameResponse),

    AcceptHands(AcceptHandsResponse),

    SelectAction(SelectActionResponse),
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
    pub preferred_format: WireFormat,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ManmenmiResponse {
    pub available_games: Vec<GameInfo>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JoinGameRequest {
    pub game_id: GameId,
    pub deck: Vec<CardId>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JoinGameResponse {
    pub player_id: PlayerId,
    pub initial_hands: Vec<CardId>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct AcceptHandsRequest {
    pub accept: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct AcceptHandsResponse {
    pub hands: Vec<CardId>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SelectActionRequest {
    pub action: Action,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SelectActionResponse {
    pub opponent_action: Action,
    pub hands: Vec<CardId>,

    pub game_result: Option<Scores>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let message = TakoyakiRequest::Manmenmi(ManmenmiRequest {
            preferred_format: WireFormat::Json,
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
