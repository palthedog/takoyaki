extern crate env_logger;
extern crate log;

use std::{path::PathBuf, sync::Arc};

use takoyaki::{players::random::RandomPlayer, client::Client, proto::{Format, GameInfo}, engine::card::Card};

use clap::{self, Parser, ValueHint};
use log::*;

use takoyaki::engine::{card, game::Context};

#[derive(Parser)]
pub struct AppArgs {
    /// a directory path where holds all card data. no need to specify for many cases.
    #[clap(long, value_parser, default_value = "data/cards")]
    card_dir: String,

    /// A file path which is a list of cards the player use for the game.
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
        default_value = "data/decks/starter"
    )]
    deck_path: PathBuf,

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
    let context = Arc::new(Context {
        all_cards,
        enabled_step_execution: false,
    });
    let deck_ids = card::load_deck(&args.deck_path);
    let deck: Arc<Vec<Card>> = Arc::new(context.get_cards(&deck_ids));

    let mut client: Client<RandomPlayer> = Client::new(
        context,
        Format::Flexbuffers,
        RandomPlayer::new(42),
        Box::new(move |games: &[GameInfo]| {
            let game_id = games[0].game_id;
            let deck = deck.clone();
            (game_id, deck.to_vec())
        })
    );

    info!("Joining a game");
    match client.join_game(&args.server) {
        Ok(result) => info!("{}", result),
        Err(e) => error!("Failed to join a game: {}", e),
    };
    info!("quiting...");
}
