use std::io::stdin;

use log::*;
use rand::{seq::SliceRandom, Rng};

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
    deck: &[&'a Card],
    player: &mut impl Player,
) -> PlayerState<'a> {
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

    PlayerState::new(&deck[0..game::HAND_SIZE], &deck[game::HAND_SIZE..])
}

pub fn run<'a, 'c: 'a>(
    context: &Context,
    player_deck: &[&'c Card],
    opponent_deck: &[&'c Card],
    player: &'a mut impl Player,
    opponent: &'a mut impl Player,
    rng: &mut impl Rng,
) -> (i32, i32) {
    assert_eq!(game::DECK_SIZE, player_deck.len());
    assert_eq!(game::DECK_SIZE, opponent_deck.len());

    player.init_game(PlayerId::Player, &context.board);
    opponent.init_game(PlayerId::Opponent, &context.board);

    let mut player_state = deal_hands(rng, player_deck, player);
    let mut opponent_state = deal_hands(rng, opponent_deck, opponent);
    debug!("Player states initialized");
    debug!("player: {}\nopponent: {}", player_state, opponent_state);
    let mut state = State::new(context.board.clone(), 0, 0, 0, vec![], vec![]);
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
            stdin().read_line(&mut String::new()).unwrap();
        }
    }

    state.board.get_scores()
}
