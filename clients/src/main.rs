use std::path::PathBuf;

use clap::{
    Args,
    Parser,
    Subcommand,
    ValueHint,
};
use log::{
    error,
    info,
};

use clients::{
    Client,
    GameResult,
};
use engine::{
    Card,
    Context,
};
use players::{
    mcts::MctsPlayer,
    random::RandomPlayer,
};
use proto::{
    GameInfo,
    WireFormat,
};

#[derive(Parser)]
pub struct ClientArgs {
    /// a directory path where holds all card data. no need to specify for many cases.
    #[clap(long, value_parser, default_value = "data/cards")]
    pub card_dir: String,

    /// A file path which is a list of cards the player use for the game.
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
        default_value = "data/decks/starter"
    )]
    pub deck_path: PathBuf,

    #[clap(long, short, value_parser, default_value = "localhost:3333")]
    pub server: String,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a player choose a random action
    Rand,

    /// Run Monte Carlo Tree Search client
    Mcts(MctsArgs),
}

#[derive(Args)]
struct MctsArgs {
    #[clap(long, short, value_parser)]
    iterations: usize,
}

pub fn init_common(args: &ClientArgs) -> (Context, Vec<Card>) {
    let all_cards = engine::load_cards(&args.card_dir);
    let context = Context {
        all_cards,
        enabled_step_execution: false,
    };
    let deck_ids = engine::load_deck(&args.deck_path);
    let deck: Vec<Card> = context.get_cards(&deck_ids);

    (context, deck)
}

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = ClientArgs::parse();

    let deck_name: String = args
        .deck_path
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap();
    let (context, deck) = init_common(&args);
    match args.command {
        Commands::Rand => run_rand(&args.server, context, format!("rand/{}", deck_name), deck),
        Commands::Mcts(m) => run_mcts(
            &args.server,
            context,
            format!("mcts-{}/{}", m.iterations, deck_name),
            deck,
            m,
        ),
    };
}

fn handle_result(game_result: Result<GameResult, String>) {
    match game_result {
        Err(e) => {
            error!("me: {}", e);
        }
        Ok(result) => {
            info!("{}", result);
        }
    };
}

fn run_rand(server: &str, context: Context, name: String, deck: Vec<Card>) {
    let mut client: Client<RandomPlayer> = Client::new(
        context,
        WireFormat::Flexbuffers,
        RandomPlayer::new(name, 42),
        Box::new(move |games: &[GameInfo]| {
            let game_id = games[0].game_id;
            (game_id, deck.to_vec())
        }),
    );

    let result = client.start(server);
    handle_result(result);
}

fn run_mcts(server: &str, context: Context, name: String, deck: Vec<Card>, mcts_args: MctsArgs) {
    let mut client: Client<MctsPlayer> = Client::new(
        context,
        WireFormat::Flexbuffers,
        MctsPlayer::new(name, 42, mcts_args.iterations),
        Box::new(move |games: &[GameInfo]| {
            let game_id = games[0].game_id;
            (game_id, deck.to_vec())
        }),
    );
    let result = client.start(server);
    handle_result(result);
}
