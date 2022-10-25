extern crate env_logger;
extern crate log;

use takoyaki::players::random::RandomPlayer;
use std::path::PathBuf;

use clap::{self, Parser, Subcommand};
use log::*;
use rand_mt::Mt64;

use takoyaki::{
    engine::{board, card, game::Context},
};

#[derive(Parser)]
pub struct AppArgs {
    /// a directory path where holds all card data. no need to specify for many cases.
    #[clap(long, value_parser, default_value = "data/cards")]
    card_dir: String,

    #[clap(long, short, value_parser, default_value = "localhost:3333")]
    server: String,
}

fn main() {
    // Initialize env_logger with a default log level of INFO.
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = AppArgs::parse();

    let all_cards = card::load_cards(&args.card_dir);
    info!("yo");

/*
    let client: Client<RandomPlayer> = Client::new(
        RandomPlayer::new(42),
    );

    match client.connect(&args.server).await {

    }
*/
}
