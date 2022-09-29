use std::{collections::HashMap, fmt::Display};

use super::{
    board::Board,
    card::{Card, CardPosition},
};

pub const HAND_SIZE: usize = 4;
pub const DECK_SIZE: usize = 15;

pub const TURN_COUNT: i32 = 12;

pub struct Context {
    pub board: Board,
    pub all_cards: HashMap<u32, Card>,
    pub enabled_step_execution: bool,
}

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

impl Rotation {
    pub const VALUES: [Self; 4] = [Self::Up, Self::Right, Self::Down, Self::Left];
}

impl Display for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PlayerType {
    Player,
    Opponent,
}

#[derive(Debug, Clone, Copy)]
pub enum Action<'a> {
    Pass(&'a Card),
    Put(&'a Card, CardPosition),
}

impl<'a> Action<'a> {
    pub fn get_consumed_card(&self) -> &'a Card {
        match self {
            Action::Pass(c) => c,
            Action::Put(c, _) => c,
        }
    }

    pub fn get_card_card_position(&self) -> (&Card, &CardPosition) {
        match self {
            Action::Pass(_) => panic!("Tried to get CardPosition from Action::Pass"),
            Action::Put(c, card_position) => (c, card_position),
        }
    }

    pub fn is_pass(&self) -> bool {
        matches!(self, Action::Pass(_))
    }

    pub fn is_put(&self) -> bool {
        matches!(self, Action::Put(_, _))
    }
}

impl<'a> Display for Action<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Pass(card) => {
                write!(f, "Pass({})", card.get_name())?;
            }
            Action::Put(card, card_position) => {
                if card_position.special {
                    write!(f, "Special!({}) @ {}", card.get_name(), card_position)?;
                } else {
                    write!(f, "Put({}) @ {}", card.get_name(), card_position)?;
                }
            }
        }
        Ok(())
    }
}
