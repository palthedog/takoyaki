use log::*;
pub use rand::{seq::SliceRandom, Rng};

use crate::{
    engine::{
        board::Board,
        card::Card,
        game::{self, PlayerId},
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
    board: &'c Board,
    player_inventory_cards: &[&'c Card],
    opponent_inventory_cards: &[&'c Card],
    player: &'a mut impl Player,
    opponent: &'a mut impl Player,
    rng: &mut impl Rng,
) -> (i32, i32) {
    player.init_game(PlayerId::Player, board);
    opponent.init_game(PlayerId::Opponent, board);

    // TODO: Support deck with more than 15 cards.
    // For now, get_deck must return 15 cards to respect the rule.
    // However, we need a way to support "pseudo" deck/hand with
    // all possible cards so that we can playout a game to implement
    // MCTS.
    // Maybe we can just put all cards in `hands`?

    let mut player_state = deal_hands(rng, player_inventory_cards, player);
    let mut opponent_state = deal_hands(rng, opponent_inventory_cards, opponent);
    debug!("Player states initialized");
    debug!("player: {}\nopponent: {}", player_state, opponent_state);
    let mut state = State::new(board.clone(), 0, 0, 0);
    for turn in 0..game::TURN_COUNT {
        debug!("Starting Turn {}", turn + 1);
        let player_action = player.get_action(&state, &player_state);
        let opponent_action = opponent.get_action(&state, &opponent_state);

        debug!("Player action: {}", player_action);
        debug!("Opponent action: {}", opponent_action);

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
    }

    state.board.get_scores()
}
