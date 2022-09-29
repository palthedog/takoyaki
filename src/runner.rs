use std::io::{stdin, Read};

use log::*;
pub use rand::{seq::SliceRandom, Rng};

use crate::{
    engine::{
        card::Card,
        game::{self, Context, PlayerId},
        state::{self, PlayerState, State},
    },
    players::*,
};

pub fn deal_hands<'a>(
    rng: &mut impl Rng,
    all_cards: &[&'a Card],
    player: &mut impl Player,
) -> PlayerState<'a> {
    let mut deck_cards = player.get_deck(all_cards);
    debug!(
        "Deck: {:#?}",
        deck_cards
            .iter()
            .map(|card| card.get_name())
            .collect::<Vec<&str>>()
    );

    deck_cards.shuffle(rng);

    if player.need_redeal_hands(&deck_cards[0..game::HAND_SIZE]) {
        deck_cards.shuffle(rng);
    }

    PlayerState::new(
        &deck_cards[0..game::HAND_SIZE],
        &deck_cards[game::HAND_SIZE..],
    )
}

pub fn run<'a, 'c: 'a>(
    context: &Context,
    player_inventory_cards: &[&'c Card],
    opponent_inventory_cards: &[&'c Card],
    player: &'a mut impl Player,
    opponent: &'a mut impl Player,
    rng: &mut impl Rng,
) -> (i32, i32) {
    player.init_game(PlayerId::Player, &context.board);
    opponent.init_game(PlayerId::Opponent, &context.board);

    let mut player_state = deal_hands(rng, player_inventory_cards, player);
    let mut opponent_state = deal_hands(rng, opponent_inventory_cards, opponent);
    debug!("Player states initialized");
    debug!("player: {}\nopponent: {}", player_state, opponent_state);
    let mut state = State::new(context.board.clone(), 0, 0, 0);
    for turn in 0..game::TURN_COUNT {
        debug!("Starting Turn {}", turn + 1);
        let player_action = player.get_action(&state, &player_state);
        let opponent_action = opponent.get_action(&state, &opponent_state);

        debug!("Player action: {}", player_action);
        debug!("Opponent action: {}", opponent_action);
        if context.enabled_step_execution {
            println!("Player action: {}", player_action);
            println!("{}", player_action.get_consumed_card());
            println!("Opponent action: {}", opponent_action);
            println!("{}", opponent_action.get_consumed_card());
        }

        state::update_state(&mut state, &player_action, &opponent_action);

        debug!("Updating player/opponent state");
        let mut tmp = player_state.clone();
        state::update_player_state(&mut tmp, &player_action);
        player_state = tmp;

        let mut tmp = opponent_state.clone();
        state::update_player_state(&mut tmp, &opponent_action);
        opponent_state = tmp;

        debug!("State is updated ->: {}", state);
        debug!("Player state: {}", player_state);
        debug!("Opponent state: {}", opponent_state);
        if context.enabled_step_execution {
            println!("{}", state);
            println!("Turn {} has finished. Press enter key to continue", {
                turn + 1
            });
            stdin().read(&mut [0]).unwrap();
        }
    }

    state.board.get_scores()
}
