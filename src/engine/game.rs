use std::fmt::Display;

use super::card::{Card, CardPosition};

pub const HAND_SIZE: usize = 4;
pub const TURN_COUNT: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerId {
    Player,
    Opponent,
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
pub enum Action<'a> {
    Pass(&'a Card),
    Put(&'a Card, CardPosition),
}
