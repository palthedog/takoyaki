pub mod mcts;
pub mod random;
pub mod utils;

use crate::engine::{
    card::Card,
    game::{Action, Context, PlayerId},
    state::State,
};

/// The base class for all player implementations.
pub trait Player<'c> {
    fn init_game(&mut self, player_id: PlayerId, context: &'c Context, deck: Vec<&'c Card>);

    /// It will be called once before the first action.
    fn need_redeal_hands(&mut self, dealed_cards: &[&'c Card]) -> bool;

    fn get_action<'a>(&mut self, state: &State, hands: &[&'c Card]) -> Action<'c>;
}
