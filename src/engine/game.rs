use super::{board::Board, card::Card};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone)]
pub struct State {
    board: Board,
    turn: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rotation {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardPosition {
    top_left: Position,
    rotation: Rotation,
    special: bool,
}

pub enum PlayerType {
    Player,
    Opponent,
}

#[derive(Debug, Clone)]
pub enum Action {
    Pass(Card),
    Put(Card, CardPosition),
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    special_count: u32,
    action_history: Vec<Action>,
}
