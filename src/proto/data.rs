use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::engine::game::{Context, self};

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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GameResult {
    pub south_score: i32,
    pub north_score: i32,
}

impl Display for GameResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
               "GameResult: (south: {}, north: {})",
               self.south_score,
               self.north_score)?;
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

impl Into<crate::engine::board::Board> for Board {
    fn into(self) -> crate::engine::board::Board {
        let (h, w) = (self.cells.len(), self.cells[0].len());
        let mut cells = Vec::with_capacity(h as usize);
        for y in 0..h {
            let mut row = Vec::with_capacity(w as usize);
            for x in 0..w {
                row.push(self.cells[y][x].into());
            }
            cells.push(row);
        }
        crate::engine::board::Board::new(
            self.name.into(),
            cells
        )
    }
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

impl Into<crate::engine::board::BoardCell> for BoardCell {
    fn into(self) -> crate::engine::board::BoardCell {
        match self {
            BoardCell::None => crate::engine::board::BoardCell::None,
            BoardCell::Wall => crate::engine::board::BoardCell::Wall,
            BoardCell::InkSouth => crate::engine::board::BoardCell::Ink(crate::engine::game::PlayerId::Player),
            BoardCell::SpecialSouth => crate::engine::board::BoardCell::Special(crate::engine::game::PlayerId::Player),
            BoardCell::InkNorth => crate::engine::board::BoardCell::Ink(crate::engine::game::PlayerId::Opponent),
            BoardCell::SpecialNorth => crate::engine::board::BoardCell::Special(crate::engine::game::PlayerId::Opponent),
        }
    }
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

impl Into<game::PlayerId> for PlayerId {
    fn into(self) -> game::PlayerId {
        match self {
            PlayerId::Sourth => game::PlayerId::Player,
            PlayerId::North => game::PlayerId::Opponent,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Action {
    Pass(CardId),
    Put(CardId, CardPosition),
    Special(CardId, CardPosition),
}

impl From<crate::engine::game::Action> for Action {
    fn from(a: crate::engine::game::Action) -> Self {
        match a {
            crate::engine::game::Action::Pass(c) => Action::Pass(c.get_id()),
            crate::engine::game::Action::Put(c, pos) => Action::Put(c.get_id(), pos.into()),
            crate::engine::game::Action::Special(c, pos) => Action::Special(c.get_id(), pos.into()),
        }
    }
}

impl Action {
    pub fn convert(self, context: &Context) -> crate::engine::game::Action {
        match self {
            Action::Pass(cid) => crate::engine::game::Action::Pass(context.get_card(cid)),
            Action::Put(cid, pos) => crate::engine::game::Action::Put(context.get_card(cid), pos.into()),
            Action::Special(cid, pos) => crate::engine::game::Action::Special(context.get_card(cid), pos.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct CardPosition {
    pub x: i32,
    pub y: i32,
    pub rotation: Rotation,
}

impl Into<crate::engine::card::CardPosition> for CardPosition {
    fn into(self) -> crate::engine::card::CardPosition {
        crate::engine::card::CardPosition {
            x: self.x,
            y: self.y,
            rotation: self.rotation.into(),
        }
    }
}

impl From<crate::engine::card::CardPosition> for CardPosition {
    fn from(a: crate::engine::card::CardPosition) -> Self {
        CardPosition {
            x: a.x,
            y: a.y,
            rotation: a.rotation.into(),
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i8)]
pub enum Rotation {
    Up,
    Right,
    Down,
    Left,
}

impl Into<crate::engine::game::Rotation> for Rotation {
    fn into(self) -> crate::engine::game::Rotation {
        match self {
            Rotation::Up => crate::engine::game::Rotation::Up,
            Rotation::Right => crate::engine::game::Rotation::Right,
            Rotation::Down => crate::engine::game::Rotation::Down,
            Rotation::Left => crate::engine::game::Rotation::Left,
        }
    }
}

impl From<crate::engine::game::Rotation> for Rotation {
    fn from(r: crate::engine::game::Rotation) -> Self {
        match r {
            crate::engine::game::Rotation::Up => Rotation::Up,
            crate::engine::game::Rotation::Right =>Rotation::Right,
            crate::engine::game::Rotation::Down => Rotation::Down,
            crate::engine::game::Rotation::Left => Rotation::Left,
        }
    }
}
