use std::{collections::HashMap, fmt::Display};

use super::card::{Card, CardPosition};

pub const HAND_SIZE: usize = 4;
pub const DECK_SIZE: usize = 15;

pub const TURN_COUNT: i32 = 12;

#[derive(Clone, Debug)]
pub struct Context {
    pub all_cards: HashMap<u32, Card>,
    pub enabled_step_execution: bool,
}

impl Context {
    pub fn get_card(&self, card_id: u32) -> Card {
        self.all_cards.get(&card_id).unwrap_or_else(||{
            panic!("Unknown card ID: {}", card_id);
        }).clone()
    }

    pub fn get_cards(&self, ids: &[u32]) -> Vec<Card> {
        ids.iter().map(|id| self.get_card(*id)).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerId {
    Player,
    Opponent,
}

impl PlayerId {
    pub fn another(&self) -> PlayerId {
        match self {
            PlayerId::Player => PlayerId::Opponent,
            PlayerId::Opponent => PlayerId::Player,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            PlayerId::Player => 0,
            PlayerId::Opponent => 1,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Pass(Card),
    Put(Card, CardPosition),
}

impl Action {
    pub fn get_consumed_card(&self) -> &Card {
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

impl Display for Action {
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
