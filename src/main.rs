extern crate env_logger;
extern crate log;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueHint};
use log::*;
use rand_mt::Mt64;

use takoyaki::{
    engine::{
        board::{self, Board},
        card::{self, Card},
    },
    players::random::RandomPlayer,
    runner,
    train::{self, deck::TrainDeckArgs},
};

#[derive(Parser)]
struct Cli {
    #[clap(long, value_parser, default_value_t = String::from("data/cards"))]
    card_dir: String,

    /// a file path to a board file
    #[clap(long, value_parser, default_value = "data/boards/massugu_street")]
    board_path: PathBuf,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    all_cards: &HashMap<u32, Card>,
    board: &Board,
    play_cnt: u32,
    player_deck_path: &Option<PathBuf>,
    opponent_deck_path: &Option<PathBuf>,
) {
    // Use fixed seed for reproducible results.
    let mut rng = Mt64::new(0x42);

    let mut player = RandomPlayer::new(rng.next_u64());
    let mut opponent = RandomPlayer::new(rng.next_u64());

    let player_inventory_cards: Vec<&Card> = match player_deck_path {
        Some(path) => card::card_ids_to_card_refs(all_cards, &card::load_deck(path)),
        None => all_cards.values().collect(),
    };
    let opponent_inventory_cards: Vec<&Card> = match opponent_deck_path {
        Some(path) => card::card_ids_to_card_refs(all_cards, &card::load_deck(path)),
        None => all_cards.values().collect(),
    };

    let mut player_won_cnt = 0;
    let mut opponent_won_cnt = 0;
    let mut draw_cnt = 0;
    for n in 0..play_cnt {
        let (p, o) = runner::run(
            board,
            &player_inventory_cards,
            &opponent_inventory_cards,
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
        if n % 100 == 0 {
            println!("Battle #{}", n);
            print_rate(player_won_cnt, opponent_won_cnt, draw_cnt);
        }
    }

    println!("\n* All battles have finished");
    print_rate(player_won_cnt, opponent_won_cnt, draw_cnt);
}

fn print_rate(p_cnt: usize, o_cnt: usize, draw_cnt: usize) {
    let total: f32 = (p_cnt + o_cnt + draw_cnt) as f32;
    let player_won_ratio: f32 = p_cnt as f32 / total;
    let opponent_won_ratio: f32 = o_cnt as f32 / total;
    println!("Player won cnt: {} ({})", p_cnt, player_won_ratio);
    println!("Opponent won cnt: {} ({})", o_cnt, opponent_won_ratio);
    println!("Draw cnt: {}", draw_cnt);
}

fn main() {
    env_logger::init();
    let args = Cli::parse();
    let all_cards = card::load_cards(&args.card_dir);
    all_cards.values().for_each(|c| debug!("{}", c));
    let board = board::load_board(&args.board_path);

    match args.command {
        Commands::Rand {
            play_cnt,
            player_deck_path,
            opponent_deck_path,
        } => {
            run_rand(
                &all_cards,
                &board,
                play_cnt,
                &player_deck_path,
                &opponent_deck_path,
            );
        }
        Commands::TrainDeck(args) => train::deck::train_deck(&all_cards, &board, args),
    }
}
