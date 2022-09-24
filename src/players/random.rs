use rand::{rngs::ThreadRng, thread_rng, Rng};

use crate::engine::{
    board::Board,
    card::Card,
    game::Action,
    state::{PlayerState, State},
};

use super::Player;

pub struct RandomPlayer {
    rng: ThreadRng,
}

impl RandomPlayer {
    pub fn new() -> Self {
        RandomPlayer { rng: thread_rng() }
    }
}

impl Default for RandomPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Player for RandomPlayer {
    fn set_board(&mut self, _board: &Board) {}

    fn get_deck<'a>(&mut self, available_cards: &[&'a Card]) -> Vec<&'a Card> {
        available_cards[0..15].to_vec()
    }

    fn need_redeal_hands(&mut self, _dealed_cards: &[&Card]) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn get_action<'a>(&mut self, _state: &State, player_state: &'a PlayerState) -> Action<'a> {
        let action = Action::Pass(player_state.get_hands()[0]);
        // todo!("Choose a random action from valid options.");
        action
    }
}
