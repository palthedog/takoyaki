use log::info;
use rand::{rngs::ThreadRng, seq::SliceRandom, thread_rng, Rng};

use crate::engine::{
    board::Board,
    card::Card,
    game::{self, Action, PlayerId},
    state::{PlayerState, State},
};

use super::{utils, Player};

pub struct RandomPlayer {
    player_id: PlayerId,
    rng: ThreadRng,
}

impl RandomPlayer {
    pub fn new() -> Self {
        RandomPlayer {
            player_id: PlayerId::Player,
            rng: thread_rng(),
        }
    }
}

impl Default for RandomPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Player for RandomPlayer {
    fn init_game(&mut self, player_id: PlayerId, _board: &Board) {
        self.player_id = player_id;
    }

    fn get_deck<'a>(&mut self, available_cards: &[&'a Card]) -> Vec<&'a Card> {
        // TODO: Provide a way to select cards.
        let mut v = available_cards.to_vec();
        let (deck, _) = v.partial_shuffle(&mut self.rng, game::DECK_SIZE);
        deck.to_vec()
    }

    fn need_redeal_hands(&mut self, _dealed_cards: &[&Card]) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn get_action<'a>(&mut self, state: &State, player_state: &'a PlayerState) -> Action<'a> {
        let mut actions_buffer: Vec<Action> = vec![];
        utils::list_valid_actions(
            state,
            player_state.get_hands(),
            self.player_id,
            &mut actions_buffer,
        );
        info!("Got {} valid actions", actions_buffer.len());
        let index = self.rng.gen_range(0..actions_buffer.len());
        let action: Action<'a> = actions_buffer.remove(index);
        action
    }
}
