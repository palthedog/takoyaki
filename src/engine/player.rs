use super::{board::Board, card::Card, game::Action, state::PlayerState};

pub trait Player {
    fn set_board(&mut self, board: &Board);

    fn get_deck<'a>(&mut self, available_cards: &[&'a Card]) -> Vec<&'a Card>;

    /// It will be called once before the first action.
    fn need_redeal_hands(&mut self, dealed_cards: &[&Card]) -> bool;

    fn get_action<'a>(&'a mut self, player_state: &'a PlayerState) -> Action;
}
