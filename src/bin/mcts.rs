extern crate env_logger;
extern crate log;

use takoyaki::{players::mcts::MctsPlayer, client::{Client, common::{ClientCommonArgs, self}}, proto::{Format, GameInfo}};

use clap::{self, Parser};
use log::*;

#[derive(Parser)]
pub struct AppArgs {
    #[clap(flatten)]
    common: ClientCommonArgs,

    #[clap(
        short,
        long,
        value_parser,
        default_value_t = 10
    )]
    iterations: usize,
}

fn main() {
    // Initialize env_logger with a default log level of INFO.
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = AppArgs::parse();
    let (context, deck) = common::init_common(&args.common);

    let mut client = Client::new(
        context,
        Format::Flexbuffers,
        MctsPlayer::new(42, args.iterations),
        Box::new(move |games: &[GameInfo]| {
            let game_id = games[0].game_id;
            (game_id, deck.to_vec())
        })
    );

    info!("Joining a game");
    match client.start(&args.common.server) {
        Ok(result) => info!("{}", result),
        Err(e) => error!("Failed to join a game: {}", e),
    };
}
