pub mod mcts;
pub mod random;
pub mod utils;

use crate::engine::{
    board::Board,
    card::Card,
    game::{Action, PlayerId},
    state::{PlayerState, State},
};

/// The base class for all player implementations.
pub trait Player {
    fn init_game(&mut self, player_id: PlayerId, board: &Board);

    fn get_deck<'a>(&mut self, available_cards: &[&'a Card]) -> Vec<&'a Card>;

    /// It will be called once before the first action.
    fn need_redeal_hands(&mut self, dealed_cards: &[&Card]) -> bool;

    fn get_action<'a>(&mut self, state: &State, player_state: &'a PlayerState) -> Action<'a>;
}
