use std::{
    fs::File,
    io::{BufRead, BufReader},
};

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

pub fn load_deck(deck_path: &str) -> Vec<u32> {
    let file = File::open(deck_path).unwrap_or_else(|_| panic!("Failed to open: {}", deck_path));
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
    lines
        .iter()
        .map(|line| {
            line.trim()
                .splitn(2, " ")
                .next()
                .unwrap()
                .parse::<u32>()
                .unwrap()
        })
        .collect()
}

// Get list of Card references from card IDs and a Card list
pub fn ids_to_deck<'a>(ids: &Vec<u32>, all_cards: &[&'a Card]) -> Vec<&'a Card> {
    ids.iter()
        .map(|id| {
            *all_cards
                .iter()
                .find(|card| card.get_id() == *id)
                .expect(&format!("Couldn't find a card with id: {}", id))
        })
        .collect()
}
