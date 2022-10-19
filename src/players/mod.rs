pub mod mcts;
pub mod random;
pub mod utils;

use crate::engine::{
    board::Board,
    card::Card,
    game::{Action, PlayerId},
    state::State,
};

/// The base class for all player implementations.
pub trait Player<'c> {
    fn init_game(&mut self, player_id: PlayerId, board: &Board);

    /// It will be called once before the first action.
    fn need_redeal_hands(&mut self, dealed_cards: &[&'c Card]) -> bool;

    fn get_action<'a>(&mut self, state: &State, hands: &[&'c Card]) -> Action<'c>;
}
