use super::game::Action;
use super::game::State;

pub fn is_valid_action(state: &State, action: Action) -> bool {
    if let Action::Pass(_) = action {
        return true;
    }

    true
}

// fn get_card_cells(card: &Card, action: &Action) -> Iter<

pub fn update(state: &mut State, player_action: Action, opponent_action: Action) -> bool {
    true
}
