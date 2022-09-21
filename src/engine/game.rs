use std::fmt::Display;

use super::{
    board::Board,
    card::{Card, CardPosition},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerId {
    Player,
    Opponent,
}

#[derive(Debug, Clone)]
pub struct State<'a> {
    pub board: Board,
    pub turn: u32,
    player_state: PlayerState<'a>,
    opponent_state: PlayerState<'a>,
}

impl<'a> State<'a> {
    pub fn new(
        board: Board,
        turn: u32,
        player_state: PlayerState<'a>,
        opponent_state: PlayerState<'a>,
    ) -> State<'a> {
        State {
            board,
            turn,
            player_state,
            opponent_state,
        }
    }

    pub fn get_player_state(&mut self, player_id: PlayerId) -> &mut PlayerState<'a> {
        match player_id {
            PlayerId::Player => &mut self.player_state,
            PlayerId::Opponent => &mut self.opponent_state,
        }
    }
}

impl<'a> Display for State<'a> {
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
pub enum Action<'a> {
    Pass(&'a Card),
    Put(&'a Card, CardPosition),
}

#[derive(Debug, Clone)]
pub struct PlayerState<'a> {
    special_count: u32,
    action_history: Vec<Action<'a>>,
    hands: Vec<&'a Card>,
    deck: Vec<&'a Card>,
}

impl<'a> PlayerState<'a> {
    pub fn new(deck: &[&'a Card]) -> PlayerState<'a> {
        PlayerState {
            special_count: 0,
            action_history: vec![],
            hands: vec![],
            deck: deck.to_vec(),
        }
    }

    #[cfg(test)]
    pub fn new_with_hand_for_testing(hand: &[&'a Card]) -> PlayerState<'a> {
        PlayerState {
            special_count: 0,
            action_history: vec![],
            hands: hand.to_vec(),
            deck: vec![],
        }
    }
}
