use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidArgument,

    InvalidMessage,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Format {
    /// New line (`'\n'`) delimited JSON.
    Json,
    /// Size delimited binary.
    FlexBuffer,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Board {
    board_name: String,
    cells: Vec<Vec<BoardCell>>,
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
