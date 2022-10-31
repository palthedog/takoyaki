use std::time::Duration;

use engine::{
    Action,
    Card,
    Context,
    PlayerId,
    State,
};

use super::{
    mcts,
    random,
};

/// The base class for all player implementations.
pub trait Player {
    fn get_name(&self) -> &str;
    fn init_game(&mut self, player_id: PlayerId, context: &Context, deck: Vec<Card>);

    /// It will be called once before the first action.
    fn need_redeal_hands(&mut self, dealed_cards: &[Card]) -> bool;

    fn get_action(&mut self, state: &State, hands: &[Card], time_limit: &Duration) -> Action;
}

#[derive(Clone, Debug)]
pub enum PlayerType {
    // Manual
    Random,
    Mcts { iterations: usize },
}

const PLAYER_TYPE_VARIANTS: [PlayerType; 5] = [
    PlayerType::Random,
    PlayerType::Mcts { iterations: 10 },
    PlayerType::Mcts { iterations: 100 },
    PlayerType::Mcts { iterations: 300 },
    PlayerType::Mcts { iterations: 1000 },
];

impl clap::ArgEnum for PlayerType {
    fn value_variants<'a>() -> &'a [Self] {
        &PLAYER_TYPE_VARIANTS
    }

    fn to_possible_value<'a>(&self) -> Option<clap::PossibleValue<'a>> {
        let name = match self {
            PlayerType::Random => "random",
            PlayerType::Mcts { iterations: 10 } => "mcts-10",
            PlayerType::Mcts { iterations: 100 } => "mcts-100",
            PlayerType::Mcts { iterations: 300 } => "mcts-300",
            PlayerType::Mcts { iterations: 1000 } => "mcts-1000",
            _ => panic!(),
        };
        Some(clap::PossibleValue::new(name))
    }
}

impl PlayerType {
    pub fn create_player(&self, _context: &Context, seed: u64) -> Box<dyn Player> {
        match self {
            PlayerType::Random => Box::new(random::RandomPlayer::new("rand".into(), seed)),
            PlayerType::Mcts { iterations } => Box::new(mcts::MctsPlayer::new(
                format!("mcts-{}", iterations),
                seed,
                *iterations,
            )),
        }
    }
}
