extern crate env_logger;
extern crate log;

use std::path::PathBuf;

use clap::{self, Parser, Subcommand};
use log::*;

use rand_mt::Mt64;
use takoyaki::{
    engine::{board, card, game::Context},
    play::{self, PlayArgs},
    players::PlayerType,
    server::{self, ServerArgs},
    train::{self, deck::TrainDeckArgs},
};

#[derive(Parser)]
pub struct AppArgs {
    /// a directory path where holds all card data. no need to specify for many cases.
    #[clap(long, value_parser, default_value_t = String::from("data/cards"))]
    card_dir: String,

    /// a file path to a board file. the selected board is used for games/training.
    #[clap(
        long,
        short,
        value_parser,
        default_value = "data/boards/massugu_street"
    )]
    board_path: PathBuf,

    #[clap(long, short, value_parser, default_value_t = false)]
    step_execution: bool,

    #[clap(long, value_parser, default_value = "random")]
    player: PlayerType,

    #[clap(long, value_parser, default_value = "random")]
    opponent: PlayerType,

    // sub commands
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// play games
    #[clap(arg_required_else_help = true)]
    Play(PlayArgs),

    /// Find stronger deck
    #[clap(arg_required_else_help = true)]
    TrainDeck(TrainDeckArgs),

    /// run server
    #[clap(arg_required_else_help = true)]
    Server(ServerArgs),
}

fn main() {
    // Initialize env_logger with a default log level of INFO.
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = AppArgs::parse();

    let all_cards = card::load_cards(&args.card_dir);
    if log_enabled!(Level::Debug) {
        all_cards.values().for_each(|c| debug!("{}", c));
    }
    let board = board::load_board(&args.board_path);

    let context = Context {
        board,
        all_cards,
        enabled_step_execution: args.step_execution,
    };

    // Use fixed seed for reproducible results.
    let mut rng = Mt64::new(0x42);

    let mut player = args.player.create_player(&context, rng.next_u64());
    let mut opponent = args.opponent.create_player(&context, rng.next_u64());

    match args.command {
        Commands::Play(args) => play::run_rand(&context, &mut *player, &mut *opponent, args),
        Commands::TrainDeck(args) => {
            train::deck::train_deck(&context, &mut *player, &mut *opponent, args)
        }
        // TODO: Make it a different binary?
        Commands::Server(args) => server::run_server(&context, args),
    }
}
