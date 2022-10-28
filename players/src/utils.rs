use log::*;

use engine::{
    Action,
    Card,
    CardPosition,
    PlayerId,
    Rotation,
    State,
};

pub fn append_valid_actions(
    state: &State,
    cards: &[Card],
    player_id: PlayerId,
    actions: &mut Vec<Action>,
) {
    let (width, height) = state.board.get_size();
    for card in cards {
        actions.push(Action::Pass(card.clone()));
        for rotation in Rotation::VALUES {
            let card_width = card.calculate_width(rotation);
            let card_height = card.calculate_height(rotation);
            for y in 1..height - card_height {
                for x in 1..width - card_width {
                    let pos = CardPosition { x, y, rotation };

                    // Normal
                    let action = Action::Put(card.clone(), pos);
                    if engine::is_valid_action(state, player_id, &action) {
                        actions.push(action);
                    }

                    // Special
                    let action = Action::Special(card.clone(), pos);
                    if engine::is_valid_action(state, player_id, &action) {
                        actions.push(action);
                    }
                }
            }
        }
    }
    debug!("Found {} valid actions", actions.len());
    trace!("Found actions:\n{:?}", actions);
}

// Get list of Card references from card IDs and a Card list
pub fn ids_to_deck<'a>(ids: &[u32], all_cards: &[&'a Card]) -> Vec<&'a Card> {
    ids.iter()
        .map(|id| {
            *all_cards
                .iter()
                .find(|card| card.get_id() == *id)
                .unwrap_or_else(|| panic!("Couldn't find a card with id: {}", id))
        })
        .collect()
}
