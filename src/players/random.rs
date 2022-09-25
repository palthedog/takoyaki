use log::info;
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

    deck_card_ids: Vec<u32>,
}

impl RandomPlayer {
    pub fn new(seed: u64, deck_card_ids: Vec<u32>) -> Self {
        RandomPlayer {
            player_id: PlayerId::Player,
            rng: Mt64::new(seed),
            deck_card_ids,
        }
    }

    pub fn new_with_random_deck(seed: u64) -> Self {
        let mut rng = Mt64::new(seed);
        let mut v: Vec<u32> = (1..=162).collect();
        let (deck_card_ids, _) = v.partial_shuffle(&mut rng, game::DECK_SIZE);
        RandomPlayer {
            player_id: PlayerId::Player,
            rng,
            deck_card_ids: deck_card_ids.to_vec(),
        }
    }
}

impl Player for RandomPlayer {
    fn init_game(&mut self, player_id: PlayerId, _board: &Board) {
        self.player_id = player_id;
    }

    fn get_deck<'a>(&mut self, available_cards: &[&'a Card]) -> Vec<&'a Card> {
        if !self.deck_card_ids.is_empty() {
            let cards = utils::ids_to_deck(&self.deck_card_ids, available_cards);
            assert_eq!(cards.len(), game::DECK_SIZE);
            return cards;
        }
        let mut v = available_cards.to_vec();
        let (deck, _) = v.partial_shuffle(&mut self.rng, game::DECK_SIZE);
        return deck.to_vec();
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
        actions_buffer.remove(index)
    }
}
