use std::path::PathBuf;

use clap::{
    self,
    Parser,
    ValueHint,
};
use log::*;

use engine::{
    self,
    Board,
    Card,
    Context,
};
use players::{
    Player,
    PlayerType,
};
use rand::seq::SliceRandom;
use rand_mt::Mt64;

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

    #[clap(long, short = 'c', value_parser, default_value_t = 1)]
    play_cnt: u32,

    /// List of cards which the player can choose for their deck. See data/decks/starter for an example.
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
    )]
    player_deck_path: PathBuf,

    /// List of cards which the opponnt can choose for their deck. See data/decks/starter for an example.
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
    )]
    opponent_deck_path: PathBuf,
}

fn main() {
    // Initialize env_logger with a default log level of INFO.
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = AppArgs::parse();

    let all_cards = engine::load_cards(&args.card_dir);
    let board = engine::load_board(&args.board_path);

    let context = Context {
        all_cards,
        enabled_step_execution: args.step_execution,
    };

    // Use fixed seed for reproducible results.
    let mut rng = Mt64::new(0x42);

    let mut player = args.player.create_player(&context, rng.next_u64());
    let mut opponent = args.opponent.create_player(&context, rng.next_u64());

    run_battles(&context, &board, &mut *player, &mut *opponent, args);
}

pub fn run_battles(
    context: &Context,
    board: &Board,
    player: &mut dyn Player,
    opponent: &mut dyn Player,
    args: AppArgs,
) {
    let play_cnt: u32 = args.play_cnt;
    let player_deck_path: PathBuf = args.player_deck_path;
    let opponent_deck_path: PathBuf = args.opponent_deck_path;

    // Use fixed seed for reproducible results.
    let mut rng = Mt64::new(0x42);

    let mut player_inventory_cards: Vec<Card> =
        context.get_cards(&engine::load_deck(&player_deck_path));
    let mut opponent_inventory_cards: Vec<Card> =
        context.get_cards(&engine::load_deck(&opponent_deck_path));

    let mut player_won_cnt = 0;
    let mut opponent_won_cnt = 0;
    let mut draw_cnt = 0;
    for n in 0..play_cnt {
        let (player_deck, _) = player_inventory_cards.partial_shuffle(&mut rng, engine::DECK_SIZE);
        let (opponent_deck, _) =
            opponent_inventory_cards.partial_shuffle(&mut rng, engine::DECK_SIZE);

        let (p, o) = local::run(
            context,
            board,
            player_deck,
            opponent_deck,
            player,
            opponent,
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
        print_rate(player_won_cnt, opponent_won_cnt, draw_cnt);
    }

    info!("\n* All battles have finished");
    info!(
        "Used decks: p: {:?}, o: {:?}",
        &player_deck_path, &opponent_deck_path
    );
    info!("Board: {}", board.get_name());
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
