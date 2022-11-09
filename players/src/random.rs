use std::time::Duration;

use rand::Rng;
use rand_mt::Mt64;

use engine::{
    Action,
    Board,
    Card,
    Context,
    PlayerId,
    State,
};

use crate::{
    utils::choose_random_action,
    Player,
};

pub struct RandomPlayer {
    player_id: PlayerId,
    name: String,
    rng: Mt64,
}

impl RandomPlayer {
    pub fn new(name: String, seed: u64) -> Self {
        RandomPlayer {
            player_id: PlayerId::South,
            name,
            rng: Mt64::new(seed),
        }
    }
}

impl Player for RandomPlayer {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn init_game(
        &mut self,
        player_id: PlayerId,
        _context: &Context,
        _board: &Board,
        _deck: Vec<Card>,
    ) {
        self.player_id = player_id;
    }

    fn need_redeal_hands(&mut self, _dealed_cards: &[Card], _time_limit: &Duration) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn get_action(&mut self, state: &State, hands: &[Card], _time_limit: &Duration) -> Action {
        choose_random_action(state, hands, self.player_id, &mut self.rng)
    }
}
