use std::io::stdin;

use log::*;
use rand::seq::SliceRandom;
use rand_mt::Mt64;

use engine::{
    Card,
    Context,
    PlayerId,
    PlayerCardState,
    State,
    Board,
};

use players::*;

pub fn deal_hands(
    rng: &mut Mt64,
    deck: &[Card],
    player: &mut dyn Player,
) -> PlayerCardState {
    let mut deck = deck.to_vec();
    debug!(
        "Deck: {:#?}",
        deck.iter()
            .map(|card| card.get_name())
            .collect::<Vec<&str>>()
    );

    deck.shuffle(rng);

    if player.need_redeal_hands(&deck[0..engine::HAND_SIZE]) {
        deck.shuffle(rng);
    }

    PlayerCardState::new(
        deck[0..engine::HAND_SIZE].to_vec(),
        deck[engine::HAND_SIZE..].to_vec(),
    )
}

pub fn run(
    context: &Context,
    board: &Board,
    player_deck: &[Card],
    opponent_deck: &[Card],
    player: &mut dyn Player,
    opponent: &mut dyn Player,
    rng: &mut Mt64,
) -> (i32, i32) {
    assert_eq!(engine::DECK_SIZE, player_deck.len());
    assert_eq!(engine::DECK_SIZE, opponent_deck.len());

    player.init_game(PlayerId::Player, context, player_deck.to_vec());
    opponent.init_game(PlayerId::Opponent, context, opponent_deck.to_vec());

    let mut player_state = deal_hands(rng, player_deck, player);
    let mut opponent_state = deal_hands(rng, opponent_deck, opponent);

    debug!("Player states initialized");
    debug!("player: {}\nopponent: {}", player_state, opponent_state);
    let mut state = State::new(board.clone(), 0, 0, 0, vec![], vec![]);
    for turn in 0..engine::TURN_COUNT {
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

        engine::update_state(&mut state, &player_action, &opponent_action);
        engine::update_player_state(&mut player_state, &player_action);
        engine::update_player_state(&mut opponent_state, &opponent_action);

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