extern crate env_logger;
extern crate log;

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueHint};
use log::*;
use rand::seq::SliceRandom;
use rand_mt::Mt64;

use takoyaki::{
    engine::{
        board,
        card::{self, Card},
        game::{self, Context},
    },
    players::{mcts::MctsPlayer, random::RandomPlayer},
    runner,
    train::{self, deck::TrainDeckArgs},
};

#[derive(Parser)]
struct Cli {
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

    // sub commands
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// play games with random hands/decks.
    #[clap(arg_required_else_help = true)]
    Rand {
        #[clap(long, short = 'c', value_parser, default_value_t = 1)]
        play_cnt: u32,

        /// List of cards which the player can choose for their deck
        #[clap(
            short,
            long,
            value_parser,
            value_hint=ValueHint::FilePath,
        )]
        player_deck_path: Option<PathBuf>,

        /// List of cards which the opponnt can choose for their deck
        #[clap(
            short,
            long,
            value_parser,
            value_hint=ValueHint::FilePath,
        )]
        opponent_deck_path: Option<PathBuf>,
    },

    TrainDeck(TrainDeckArgs),
}

fn run_rand(
    context: &Context,
    play_cnt: u32,
    player_deck_path: &Option<PathBuf>,
    opponent_deck_path: &Option<PathBuf>,
) {
    // Use fixed seed for reproducible results.
    let mut rng = Mt64::new(0x42);

    //let mut player = RandomPlayer::new(rng.next_u64());
    let mut player = MctsPlayer::new(rng.next_u64());
    let mut opponent = RandomPlayer::new(rng.next_u64());
    //let mut opponent = MctsPlayer::new(rng.next_u64());

    let mut player_inventory_cards: Vec<&Card> = match player_deck_path {
        Some(path) => card::card_ids_to_card_refs(&context.all_cards, &card::load_deck(path)),
        None => context.all_cards.values().collect(),
    };
    let mut opponent_inventory_cards: Vec<&Card> = match opponent_deck_path {
        Some(path) => card::card_ids_to_card_refs(&context.all_cards, &card::load_deck(path)),
        None => context.all_cards.values().collect(),
    };

    let mut player_won_cnt = 0;
    let mut opponent_won_cnt = 0;
    let mut draw_cnt = 0;
    for n in 0..play_cnt {
        let (player_deck, _) = player_inventory_cards.partial_shuffle(&mut rng, game::DECK_SIZE);
        let (opponent_deck, _) =
            opponent_inventory_cards.partial_shuffle(&mut rng, game::DECK_SIZE);

        let (p, o) = runner::run(
            context,
            &player_deck,
            &opponent_deck,
            &mut player,
            &mut opponent,
            &mut rng,
        );
        match p.cmp(&o) {
            std::cmp::Ordering::Less => {
                debug!("Opponent win!");
                opponent_won_cnt += 1;
            }
            std::cmp::Ordering::Equal => {
                debug!("Draw");
                draw_cnt += 1;
            }
            std::cmp::Ordering::Greater => {
                debug!("Player win!");
                player_won_cnt += 1;
            }
        }
        info!("Battle #{}. {} v.s. {} ", n, p, o);
        if n % 1 == 0 || context.enabled_step_execution {
            info!("Battle #{}", n);
            print_rate(player_won_cnt, opponent_won_cnt, draw_cnt);
        }
    }

    info!("\n* All battles have finished");
    info!(
        "Used decks: p: {:?}, o: {:?}",
        player_deck_path, opponent_deck_path
    );
    info!("Board: {}", &context.board.get_name());
    print_rate(player_won_cnt, opponent_won_cnt, draw_cnt);
}

fn print_rate(p_cnt: usize, o_cnt: usize, draw_cnt: usize) {
    let total: f32 = (p_cnt + o_cnt + draw_cnt) as f32;
    let player_won_ratio: f32 = p_cnt as f32 / total;
    let opponent_won_ratio: f32 = o_cnt as f32 / total;
    info!("Player won cnt: {} ({:.3})", p_cnt, player_won_ratio);
    info!("Opponent won cnt: {} ({:.3})", o_cnt, opponent_won_ratio);
    info!("Draw cnt: {}", draw_cnt);
}

fn main() {
    // Initialize env_logger with a default log level of INFO.
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = Cli::parse();

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

    match args.command {
        Commands::Rand {
            play_cnt,
            player_deck_path,
            opponent_deck_path,
        } => {
            run_rand(&context, play_cnt, &player_deck_path, &opponent_deck_path);
        }
        Commands::TrainDeck(args) => train::deck::train_deck(&context, args),
    }
}
