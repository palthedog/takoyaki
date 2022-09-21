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
use more_asserts::*;
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
    deck_cards: &mut Vec<&'a Card>,
    hand: &'a mut Vec<&'a Card>,
) {
    let deal_cnt = game::HAND_SIZE - hand.len();
    assert_lt!(deal_cnt, game::HAND_SIZE, "The hand is already dealed");

    let (dealed, _) = deck_cards.partial_shuffle(rng, game::HAND_SIZE);
    let dealed_cnt = dealed.len();
    assert_eq!(
        dealed_cnt, deal_cnt,
        "The number of dealed card({}) is different from what we requested({})",
        dealed_cnt, deal_cnt
    );
    hand.extend(dealed.iter());
    deck_cards.drain(0..dealed_cnt);

    assert_eq!(game::HAND_SIZE, hand.len());
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

    todo!("shuffle cards and deal them");
}

struct RandomPlayer {
    rng: ThreadRng,
}

impl RandomPlayer {
    pub fn new() -> RandomPlayer {
        RandomPlayer { rng: thread_rng() }
    }
}

impl Player for RandomPlayer {
    fn set_board(&mut self, board: &Board) {}

    fn get_deck<'a>(&mut self, available_cards: &[&'a Card]) -> Vec<&'a Card> {
        available_cards[0..15].to_vec()
    }

    fn need_redeal_hands(&mut self, player_state: &PlayerState) -> bool {
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
