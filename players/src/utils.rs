use itertools::Itertools;
use log::*;

use engine::{
    Action,
    Card,
    CardPosition,
    PlayerId,
    Rotation,
    State,
};
use rand::{
    seq::SliceRandom,
    Rng,
};

#[derive(PartialEq, Eq)]
enum ActionType {
    Pass,
    Put,
    Special,
}

impl ActionType {
    const VALUES: &[ActionType] = &[ActionType::Pass, ActionType::Put, ActionType::Special];
}

// Note that it's randomness isn't great. Like, it has 25% of chance to choose Pass.
pub fn choose_random_action(
    state: &State,
    cards: &[Card],
    player_id: PlayerId,
    rng: &mut impl Rng,
) -> Action {
    let mut cards = cards.iter().collect_vec();
    let mut acts = ActionType::VALUES.iter().collect_vec();
    let mut rots = Rotation::VALUES.iter().collect_vec();
    let mut y_range = state.board.get_y_range().iter().collect_vec();
    let mut x_range = state.board.get_x_range().iter().collect_vec();
    cards.shuffle(rng);
    acts.shuffle(rng);
    rots.shuffle(rng);
    y_range.shuffle(rng);
    x_range.shuffle(rng);

    for act in acts {
        for card in cards.iter() {
            if *act == ActionType::Pass {
                return Action::Pass((*card).clone());
            }
            for rotation in rots.iter() {
                for y in y_range.iter() {
                    for x in x_range.iter() {
                        let pos = CardPosition {
                            x: **x,
                            y: **y,
                            rotation: **rotation,
                        };
                        let action = match act {
                            ActionType::Put => Action::Put((*card).clone(), pos),
                            ActionType::Special => Action::Special((*card).clone(), pos),
                            ActionType::Pass => unimplemented!(),
                        };
                        if engine::is_valid_action(state, player_id, &action) {
                            return action;
                        }
                    }
                }
            }
        }
    }
    unimplemented!();
}

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
                    let pos = CardPosition {
                        x,
                        y,
                        rotation,
                    };

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
