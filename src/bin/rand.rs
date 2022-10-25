extern crate env_logger;
extern crate log;

use takoyaki::{players::random::RandomPlayer, client::Client, proto::Format};

use clap::{self, Parser};
use log::*;

use takoyaki::engine::{card, game::Context};

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
    let context = Context {
        all_cards,
        enabled_step_execution: false,
    };

    let client: Client<RandomPlayer> = Client::new(
        &context,
        Format::Json,
        RandomPlayer::new(42),
    );

    info!("Joining a game");
    match client.join_game(&args.server) {
        Ok(result) => info!("{}", result),
        Err(e) => error!("Failed to join a game: {}", e),
    };
    info!("quiting...");
}
