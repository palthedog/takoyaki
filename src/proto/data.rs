use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    Timeout,

    /// The server failed to parse the payload.
    MalformedPayload,

    /// The server doesn't want this request at this point.
    BadRequest,

    NetworkError,
    SerializationFailed,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Format {
    /// New line (`'\n'`) delimited JSON.
    Json,

    // Size delimited binary. Payload must look like
    // +-------------------------+----------------------------+
    // | size: u32 in big-endian | encoded_body: [u8; <size>] |
    // +-------------------------+----------------------------+
    Flexbuffers,
}

#[derive(Debug)]
pub struct GameResult {
    pub score: u32,
    pub opponent_score: u32,
}

impl Display for GameResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
               "GameResult: {}(score: {}, opponent: {})",
               match self.score.cmp(&self.opponent_score) {
                   std::cmp::Ordering::Less => "Lose",
                   std::cmp::Ordering::Equal => "Draw",
                   std::cmp::Ordering::Greater => "Win",
               },
               self.score,
               self.opponent_score)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Board {
    pub board_name: String,
    pub cells: Vec<Vec<BoardCell>>,
}

/// An enum reprecents each cell on a board.
/// We do NOT use enum with fields (e.g. Ink(PlayerId)) to keep the serialized data small.
#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Eq)]
#[repr(i8)]
pub enum BoardCell {
    None = 0,
    Wall = 9,

    InkSouth = 1,
    SpecialSouth = 2,

    InkNorth = -1,
    SpecialNorth = -2,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Eq)]
#[repr(i8)]
pub enum PlayerId {
    Sourth = 1,
    North = -1,
}
