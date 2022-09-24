use log::*;

use crate::engine::{
    card::{Card, CardPosition},
    game::{Action, PlayerId, Rotation},
    state::{self, State},
};

pub fn list_valid_actions<'a>(
    state: &State,
    cards: &[&'a Card],
    player_id: PlayerId,
    actions: &mut Vec<Action<'a>>,
) {
    actions.clear();

    let (width, height) = state.board.get_size();
    for card in cards {
        actions.push(Action::Pass(card));
        for y in 0..height {
            for x in 0..width {
                for rotation in Rotation::VALUES {
                    for special in [false, true] {
                        let pos = CardPosition {
                            x,
                            y,
                            rotation,
                            special,
                        };
                        let action = Action::Put(card, pos);
                        if state::is_valid_action(state, player_id, &action) {
                            actions.push(action);
                        }
                    }
                }
            }
        }
    }
    debug!("Found {} valid actions", actions.len());
    trace!("Found actions:\n{:?}", actions);
}
