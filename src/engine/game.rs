use std::fmt::Display;

use super::{
    board::Board,
    card::{Card, CardPosition},
};

#[derive(Debug, Clone)]
pub struct State {
    pub board: Board,
    pub turn: u32,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "turn: {}\n{}", self.turn, self.board)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rotation {
    Up,
    Right,
    Down,
    Left,
}

impl Display for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
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
    pub special_count: u32,
    pub action_history: Vec<Action>,
}
