extern crate env_logger;
extern crate log;

use takoyaki::engine::{board, card};

use clap::Parser;
use log::*;

#[derive(Parser, Debug)]
#[clap(about)]
struct Args {
    #[clap(long, value_parser, default_value_t = String::from("data/cards"))]
    card_dir: String,

    #[clap(long, value_parser, default_value_t = String::from("data/boards"))]
    board_dir: String,
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    let all_cards = card::load_cards(&args.card_dir);

    all_cards.iter().for_each(|c| info!("{}", c));

    let all_boards = board::load_boards(&args.board_dir);
    all_boards.iter().for_each(|c| info!("{}", c));
}
