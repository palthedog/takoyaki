use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub type GameId = u32;
pub type CardId = u32;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    Timeout,

    /// The server failed to parse the payload.
    MalformedPayload,

    /// The server doesn't want this request at this point.
    BadRequest,

    NetworkError,
    SerializationFailure,
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
pub struct GameInfo {
    pub game_id: GameId,
    pub board: Board,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct PlayerState {
    pub hands: Vec<CardId>,
}

impl From<&crate::engine::state::PlayerCardState> for PlayerState {
    fn from(s: &crate::engine::state::PlayerCardState) -> Self {
        PlayerState {
            hands: crate::engine::card::to_ids(s.get_hands())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Board {
    pub name: String,
    pub cells: Vec<Vec<BoardCell>>,
}

impl From<&crate::engine::board::Board> for Board {
    fn from(b: &crate::engine::board::Board) -> Self {
        let (w, h) = b.get_size();
        let mut cells = Vec::with_capacity(h as usize);
        for y in 0..h {
            let mut row = Vec::with_capacity(w as usize);
            for x in 0..w {
                row.push(b.get_cell(crate::engine::board::BoardPosition{x, y}).into());
            }
            cells.push(row);
        }
        Board {
            name: b.get_name().into(),
            cells
        }
    }
}

/// An enum reprecents each cell on a board.
/// We do NOT use enum with fields (e.g. Ink(PlayerId)) to keep the serialized data small.
#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i8)]
pub enum BoardCell {
    None = 0,
    Wall = 9,

    InkSouth = 1,
    SpecialSouth = 2,

    InkNorth = -1,
    SpecialNorth = -2,
}

impl From<crate::engine::board::BoardCell> for BoardCell {
    fn from(c: crate::engine::board::BoardCell) -> Self {
        match c {
            crate::engine::board::BoardCell::None => BoardCell::None,
            crate::engine::board::BoardCell::Wall => BoardCell::Wall,
            crate::engine::board::BoardCell::Ink(crate::engine::game::PlayerId::Player) => BoardCell::InkSouth,
            crate::engine::board::BoardCell::Ink(crate::engine::game::PlayerId::Opponent) => BoardCell::InkNorth,
            crate::engine::board::BoardCell::Special(crate::engine::game::PlayerId::Player) => BoardCell::SpecialSouth,
            crate::engine::board::BoardCell::Special(crate::engine::game::PlayerId::Opponent) => BoardCell::SpecialNorth,
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i8)]
pub enum PlayerId {
    Sourth = 1,
    North = -1,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Action {
    Pass(CardId),
    Put(CardId),
    Special(CardId),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct CardPosition {
    pub x: u32,
    pub y: u32,
    pub rotation: Rotation,
}

/*
impl From<&crate::engine::card::CardPosition> for CardPosition {
    fn from(_: &crate::engine::card::CardPosition) -> Self {
        
    }
}
*/

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i8)]
pub enum Rotation {
    Up,
    Right,
    Down,
    Left,
}

/*
impl From<&crate::engine::card::Rotation> for CardRotation {
    fn from(r: &crate::engine::card::Rotation) -> Self {
        match r {
            crate::engine::game::Rotation::Up => Rotation::Up,
            crate::engine::game::Rotation::Right =>Rotation::Right, 
            crate::engine::game::Rotation::Down => Rotation::Down,
            crate::engine::game::Rotation::Left => Rotation::Left,
        }
    }
}
*/
