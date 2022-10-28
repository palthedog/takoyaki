use log::*;
use rand::Rng;
use rand_mt::Mt64;

use engine::{
    Action,
    Card,
    Context,
    PlayerId,
    State,
};

use super::{
    utils,
    Player,
};

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

impl Player for RandomPlayer {
    fn get_name(&self) -> &str {
        "rand"
    }

    fn init_game(&mut self, player_id: PlayerId, _context: &Context, _deck: Vec<Card>) {
        self.player_id = player_id;
    }

    fn need_redeal_hands(&mut self, _dealed_cards: &[Card]) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn get_action(&mut self, state: &State, hands: &[Card]) -> Action {
        let mut actions_buffer: Vec<Action> = vec![];
        utils::append_valid_actions(state, hands, self.player_id, &mut actions_buffer);
        debug!("Got {} valid actions", actions_buffer.len());
        let index = self.rng.gen_range(0..actions_buffer.len());
        actions_buffer.remove(index)
    }
}
