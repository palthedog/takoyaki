use std::collections::HashMap;

use log::debug;
use log::trace;

use super::board::BoardPosition;
use super::card::Card;
use super::card::CardCell;
use super::card::CardCellPosition;
use super::card::CardPosition;
use super::game::Action;
use super::game::State;

pub fn is_valid_action(state: &State, action: Action) -> bool {
    match action {
        Action::Pass(_) => true,
        Action::Put(card, pos) => check_action_put(state, &card, &pos),
    }
}

fn check_action_put(state: &State, card: &Card, position: &CardPosition) -> bool {
    let cells = card.get_cells(position.rotation);

    if has_conflict_with_wall(state, position, cells) {
        return false;
    }
    true
}

fn has_conflict_with_wall(
    state: &State,
    card_position: &CardPosition,
    cells: &HashMap<CardCellPosition, CardCell>,
) -> bool {
    for cell_pos in cells.keys() {
        let board_pos = BoardPosition {
            x: card_position.x + cell_pos.x,
            y: card_position.y + cell_pos.y,
        };
        if state.board.get_cell(board_pos).is_wall() {
            trace!("A cell has conflict with a wall at: {:?}", board_pos);
            return true;
        }
    }
    false
}

pub fn update(state: &mut State, player_action: Action, opponent_action: Action) -> bool {
    debug!(
        "Player action is valid? {}",
        is_valid_action(state, player_action)
    );
    debug!(
        "Opponent action is valid? {}",
        is_valid_action(state, opponent_action)
    );

    todo!()
}
