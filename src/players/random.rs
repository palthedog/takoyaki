use log::*;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_mt::Mt64;

use crate::engine::{
    board::Board,
    card::Card,
    game::{self, Action, PlayerId},
    state::{PlayerState, State},
};

use super::{utils, Player};

pub struct RandomPlayer {
    player_id: PlayerId,
    rng: Mt64,
}

impl RandomPlayer {
    pub fn new(seed: u64) -> Self {
        RandomPlayer {
            player_id: PlayerId::Player,
            rng: Mt64::new(seed),
        }
    }
}

impl<'c> Player<'c> for RandomPlayer {
    fn init_game(&mut self, player_id: PlayerId, _board: &Board) {
        self.player_id = player_id;
    }

    fn need_redeal_hands(&mut self, _dealed_cards: &[&'c Card]) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn get_action<'a>(&mut self, state: &State, player_state: &PlayerState<'c>) -> Action<'c> {
        let mut actions_buffer: Vec<Action> = vec![];
        utils::append_valid_actions(
            state,
            player_state.get_hands(),
            self.player_id,
            &mut actions_buffer,
        );
        debug!("Got {} valid actions", actions_buffer.len());
        let index = self.rng.gen_range(0..actions_buffer.len());
        actions_buffer.remove(index)
    }
}
