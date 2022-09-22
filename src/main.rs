extern crate env_logger;
extern crate log;

use takoyaki::engine::{
    board::{self, Board},
    card::{self, Card},
    game::{self, Action},
    player::Player,
    state::PlayerState,
};

use clap::Parser;
use log::*;
use rand::{rngs::ThreadRng, seq::SliceRandom, thread_rng, Rng};

#[derive(Parser, Debug)]
#[clap(about)]
struct Args {
    #[clap(long, value_parser, default_value_t = String::from("data/cards"))]
    card_dir: String,

    #[clap(long, value_parser, default_value_t = String::from("data/boards"))]
    board_dir: String,
}

pub fn deal_hands<'a>(
    rng: &mut impl Rng,
    all_cards: &[&'a Card],
    player: &'a mut impl Player,
) -> PlayerState<'a> {
    let mut deck_cards = player.get_deck(all_cards);

    let (mut dealed, mut remaining) = deck_cards.partial_shuffle(rng, game::HAND_SIZE);

    if player.need_redeal_hands(dealed) {
        (dealed, remaining) = deck_cards.partial_shuffle(rng, game::HAND_SIZE);
    }

    PlayerState::new(dealed, remaining)
}

fn run<'a>(
    board: &'a Board,
    all_cards: &[&'a Card],
    player: &'a mut impl Player,
    opponent: &'a mut impl Player,
) {
    let mut rng = thread_rng();

    player.set_board(board);
    opponent.set_board(board);

    // TODO: Support deck with more than 15 cards.
    // For now, get_deck must return 15 cards to respect the rule.
    // However, we need a way to support "pseudo" deck/hand with
    // all possible cards so that we can playout a game to implement
    // MCTS.
    // Maybe we can just put all cards in `hands`?

    let player_state = deal_hands(&mut rng, all_cards, player);
    let opponent_state = deal_hands(&mut rng, all_cards, opponent);
    info!("Player states initialized");
    // TODO: Implement shorter display format for PlayerState
    info!("player: {}\nopponent: {}", player_state, opponent_state);

    todo!("Implement main loop.");
}

// TODO: Move to a different file/module.
struct RandomPlayer {
    rng: ThreadRng,
}

impl RandomPlayer {
    pub fn new() -> RandomPlayer {
        RandomPlayer { rng: thread_rng() }
    }
}

impl Player for RandomPlayer {
    fn set_board(&mut self, _board: &Board) {}

    fn get_deck<'a>(&mut self, available_cards: &[&'a Card]) -> Vec<&'a Card> {
        available_cards[0..15].to_vec()
    }

    fn need_redeal_hands(&mut self, _dealed_cards: &[&Card]) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn get_action<'a>(&'a mut self, player_state: &'a PlayerState) -> Action {
        Action::Pass(player_state.get_hands()[0])
    }
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    let all_cards = card::load_cards(&args.card_dir);
    let all_card_refs: Vec<&Card> = all_cards.iter().map(|c| c).collect();

    all_cards.iter().for_each(|c| info!("{}", c));

    let all_boards = board::load_boards(&args.board_dir);
    all_boards.iter().for_each(|c| info!("{}", c));

    let mut player = RandomPlayer::new();
    let mut opponent = RandomPlayer::new();

    run(&all_boards[0], &all_card_refs, &mut player, &mut opponent);
}
