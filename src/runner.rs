use std::io::stdin;

use log::*;
use rand::{seq::SliceRandom, Rng};
use rand_mt::Mt64;

use crate::{
    engine::{
        card::Card,
        game::{self, Action, Context, PlayerId},
        state::{self, PlayerCardState, State},
    },
    players::*,
};

pub fn deal_hands<'c>(
    rng: &mut Mt64,
    deck: &[&'c Card],
    player: &mut dyn Player<'c>,
) -> PlayerCardState<'c> {
    let mut deck = deck.to_vec();
    debug!(
        "Deck: {:#?}",
        deck.iter()
            .map(|card| card.get_name())
            .collect::<Vec<&str>>()
    );

    deck.shuffle(rng);

    if player.need_redeal_hands(&deck[0..game::HAND_SIZE]) {
        deck.shuffle(rng);
    }

    PlayerCardState::new(
        deck[0..game::HAND_SIZE].to_vec(),
        deck[game::HAND_SIZE..].to_vec(),
    )
}

pub fn run<'c>(
    context: &'c Context,
    player_deck: &[&'c Card],
    opponent_deck: &[&'c Card],
    player: &mut dyn Player<'c>,
    opponent: &mut dyn Player<'c>,
    rng: &mut Mt64,
) -> (i32, i32) {
    assert_eq!(game::DECK_SIZE, player_deck.len());
    assert_eq!(game::DECK_SIZE, opponent_deck.len());

    player.init_game(PlayerId::Player, &context, player_deck.to_vec());
    opponent.init_game(PlayerId::Opponent, &context, opponent_deck.to_vec());

    let mut player_state = deal_hands(rng, player_deck, player);
    let mut opponent_state = deal_hands(rng, opponent_deck, opponent);

    debug!("Player states initialized");
    debug!("player: {}\nopponent: {}", player_state, opponent_state);
    let mut state = State::new(context.board.clone(), 0, 0, 0, vec![], vec![]);
    for turn in 0..game::TURN_COUNT {
        debug!("Starting Turn {}", turn + 1);
        let player_action = player.get_action(&state, player_state.get_hands());
        let opponent_action = opponent.get_action(&state, opponent_state.get_hands());

        debug!("Original State: {}", state);
        debug!("Player state: {}", player_state);
        debug!("Opponent state: {}", opponent_state);
        debug!("Player action: {}", player_action);
        debug!("Opponent action: {}", opponent_action);
        if context.enabled_step_execution {
            println!("Player action: {}", player_action);
            println!("{}", player_action.get_consumed_card());
            println!("Opponent action: {}", opponent_action);
            println!("{}", opponent_action.get_consumed_card());
        }

        state::update_state(&mut state, &player_action, &opponent_action);
        state::update_player_state(&mut player_state, &player_action);
        state::update_player_state(&mut opponent_state, &opponent_action);

        debug!("State is updated ->: {}", state);
        debug!("Player state: {}", player_state);
        debug!("Opponent state: {}", opponent_state);
        if context.enabled_step_execution {
            println!("{}", state);
            println!("Turn {} has finished. Press enter key to continue", {
                turn + 1
            });
            stdin().read_line(&mut String::new()).unwrap();
        }
    }

    state.board.get_scores()
}
