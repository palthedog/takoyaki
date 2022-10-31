use std::fmt::Display;

use serde::{
    Deserialize,
    Serialize,
};
use serde_repr::{
    Deserialize_repr,
    Serialize_repr,
};

use engine;

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
pub enum WireFormat {
    /// New line (`'\n'`) delimited JSON.
    Json,

    // Size delimited binary. Payload must look like
    // +-------------------------+----------------------------+
    // | size: u32 in big-endian | encoded_body: [u8; <size>] |
    // +-------------------------+----------------------------+
    Flexbuffers,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Scores {
    pub south_score: u32,
    pub north_score: u32,
}

impl Display for Scores {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Scores: (south: {}, north: {})",
            self.south_score, self.north_score
        )?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum TimeControl {
    /// Players can spend as much as time they want.
    #[default]
    Infinite,

    /// Players can spend `time_limit_in_seconds` seconds for each action.
    /// If a player exceeds the time limit, the player loses.
    PerAction { time_limit_in_seconds: u32 },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GameInfo {
    pub game_id: GameId,
    pub time_control: TimeControl,
    pub board: Board,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct PlayerState {
    pub hands: Vec<CardId>,
}

impl From<&engine::PlayerCardState> for PlayerState {
    fn from(s: &engine::PlayerCardState) -> Self {
        PlayerState {
            hands: engine::to_ids(s.get_hands()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Board {
    pub name: String,
    pub cells: Vec<Vec<BoardCell>>,
}

impl From<Board> for engine::Board {
    fn from(val: Board) -> Self {
        let (h, w) = (val.cells.len(), val.cells[0].len());
        let mut cells = Vec::with_capacity(h as usize);
        for y in 0..h {
            let mut row = Vec::with_capacity(w as usize);
            for x in 0..w {
                row.push(val.cells[y][x].into());
            }
            cells.push(row);
        }
        engine::Board::new(val.name, cells)
    }
}

impl From<&engine::Board> for Board {
    fn from(b: &engine::Board) -> Self {
        let (w, h) = b.get_size();
        let mut cells = Vec::with_capacity(h as usize);
        for y in 0..h {
            let mut row = Vec::with_capacity(w as usize);
            for x in 0..w {
                row.push(b.get_cell(engine::BoardPosition { x, y }).into());
            }
            cells.push(row);
        }
        Board {
            name: b.get_name().into(),
            cells,
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

impl From<BoardCell> for engine::BoardCell {
    fn from(val: BoardCell) -> Self {
        match val {
            BoardCell::None => engine::BoardCell::None,
            BoardCell::Wall => engine::BoardCell::Wall,
            BoardCell::InkSouth => engine::BoardCell::Ink(engine::PlayerId::South),
            BoardCell::SpecialSouth => engine::BoardCell::Special(engine::PlayerId::South),
            BoardCell::InkNorth => engine::BoardCell::Ink(engine::PlayerId::North),
            BoardCell::SpecialNorth => engine::BoardCell::Special(engine::PlayerId::North),
        }
    }
}

impl From<engine::BoardCell> for BoardCell {
    fn from(c: engine::BoardCell) -> Self {
        match c {
            engine::BoardCell::None => BoardCell::None,
            engine::BoardCell::Wall => BoardCell::Wall,
            engine::BoardCell::Ink(engine::PlayerId::South) => BoardCell::InkSouth,
            engine::BoardCell::Ink(engine::PlayerId::North) => BoardCell::InkNorth,
            engine::BoardCell::Special(engine::PlayerId::South) => BoardCell::SpecialSouth,
            engine::BoardCell::Special(engine::PlayerId::North) => BoardCell::SpecialNorth,
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i8)]
pub enum PlayerId {
    Sourth = 1,
    North = -1,
}

impl From<PlayerId> for engine::PlayerId {
    fn from(val: PlayerId) -> Self {
        match val {
            PlayerId::Sourth => engine::PlayerId::South,
            PlayerId::North => engine::PlayerId::North,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Action {
    Pass(CardId),
    Put(CardId, CardPosition),
    Special(CardId, CardPosition),
}

impl From<engine::Action> for Action {
    fn from(a: engine::Action) -> Self {
        match a {
            engine::Action::Pass(c) => Action::Pass(c.get_id()),
            engine::Action::Put(c, pos) => Action::Put(c.get_id(), pos.into()),
            engine::Action::Special(c, pos) => Action::Special(c.get_id(), pos.into()),
        }
    }
}

impl Action {
    pub fn convert(self, context: &engine::Context) -> engine::Action {
        match self {
            Action::Pass(cid) => engine::Action::Pass(context.get_card(cid)),
            Action::Put(cid, pos) => engine::Action::Put(context.get_card(cid), pos.into()),
            Action::Special(cid, pos) => engine::Action::Special(context.get_card(cid), pos.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct CardPosition {
    pub x: i32,
    pub y: i32,
    pub rotation: Rotation,
}

impl From<CardPosition> for engine::CardPosition {
    fn from(val: CardPosition) -> Self {
        engine::CardPosition {
            x: val.x,
            y: val.y,
            rotation: val.rotation.into(),
        }
    }
}

impl From<engine::CardPosition> for CardPosition {
    fn from(a: engine::CardPosition) -> Self {
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

impl From<Rotation> for engine::Rotation {
    fn from(val: Rotation) -> Self {
        match val {
            Rotation::Up => engine::Rotation::Up,
            Rotation::Right => engine::Rotation::Right,
            Rotation::Down => engine::Rotation::Down,
            Rotation::Left => engine::Rotation::Left,
        }
    }
}

impl From<engine::Rotation> for Rotation {
    fn from(r: engine::Rotation) -> Self {
        match r {
            engine::Rotation::Up => Rotation::Up,
            engine::Rotation::Right => Rotation::Right,
            engine::Rotation::Down => Rotation::Down,
            engine::Rotation::Left => Rotation::Left,
        }
    }
}
