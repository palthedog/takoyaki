extern crate env_logger;
extern crate log;

use std::collections::HashMap;
use std::path::PathBuf;

use rand_mt::Mt64;
use takoyaki::engine::game::{self, PlayerId};
use takoyaki::engine::{
    board::{self, Board},
    card::{self, Card},
    state::{self, PlayerState, State},
};
use takoyaki::players::random::RandomPlayer;

use takoyaki::players::*;

use clap::{Parser, Subcommand, ValueHint};
use log::*;
pub use rand::{seq::SliceRandom, Rng};
use takoyaki::players::utils::load_deck;

#[derive(Parser, Debug)]
struct Cli {
    #[clap(long, value_parser, default_value_t = String::from("data/cards"))]
    card_dir: String,

    /// a file path to a board file
    #[clap(long, value_parser, default_value = "data/boards/0")]
    board_path: PathBuf,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(arg_required_else_help = true)]
    Rand {
        #[clap(long, value_parser, default_value_t = 1)]
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

    TrainDeck {
        #[clap(long, value_parser, default_value_t = 1)]
        train_cnt: u32,
    },
}

pub fn deal_hands<'a>(
    rng: &mut impl Rng,
    all_cards: &[&'a Card],
    player: &mut impl Player,
) -> PlayerState<'a> {
    let mut deck_cards = player.get_deck(all_cards);
    info!(
        "Deck: {:#?}",
        deck_cards
            .iter()
            .map(|card| card.get_name())
            .collect::<Vec<&str>>()
    );

    deck_cards.shuffle(rng);

    if player.need_redeal_hands(&deck_cards[0..game::HAND_SIZE]) {
        deck_cards.shuffle(rng);
    }

    PlayerState::new(
        &deck_cards[0..game::HAND_SIZE],
        &deck_cards[game::HAND_SIZE..],
    )
}

fn run<'a, 'c: 'a>(
    board: &'c Board,
    player_inventory_cards: &[&'c Card],
    opponent_inventory_cards: &[&'c Card],
    player: &'a mut impl Player,
    opponent: &'a mut impl Player,
    rng: &mut impl Rng,
) -> (i32, i32) {
    player.init_game(PlayerId::Player, board);
    opponent.init_game(PlayerId::Opponent, board);

    // TODO: Support deck with more than 15 cards.
    // For now, get_deck must return 15 cards to respect the rule.
    // However, we need a way to support "pseudo" deck/hand with
    // all possible cards so that we can playout a game to implement
    // MCTS.
    // Maybe we can just put all cards in `hands`?

    let mut player_state = deal_hands(rng, player_inventory_cards, player);
    let mut opponent_state = deal_hands(rng, opponent_inventory_cards, opponent);
    info!("Player states initialized");
    info!("player: {}\nopponent: {}", player_state, opponent_state);
    let mut state = State::new(board.clone(), 0, 0, 0);
    for turn in 0..game::TURN_COUNT {
        info!("Starting Turn {}", turn + 1);
        let player_action = player.get_action(&state, &player_state);
        let opponent_action = opponent.get_action(&state, &opponent_state);

        info!("Player action: {}", player_action);
        info!("Opponent action: {}", opponent_action);

        state::update_state(&mut state, &player_action, &opponent_action);

        debug!("Updating player/opponent state");
        let mut tmp = player_state.clone();
        state::update_player_state(&mut tmp, &player_action);
        player_state = tmp;

        let mut tmp = opponent_state.clone();
        state::update_player_state(&mut tmp, &opponent_action);
        opponent_state = tmp;

        info!("State is updated ->: {}", state);
        info!("Player state: {}", player_state);
        info!("Opponent state: {}", opponent_state);
    }

    state.board.get_scores()
}

fn card_ids_to_card_refs<'a>(all_cards: &'a HashMap<u32, Card>, card_ids: &[u32]) -> Vec<&'a Card> {
    card_ids
        .iter()
        .map(|id| all_cards.get(id).unwrap())
        .collect()
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
        Some(path) => card_ids_to_card_refs(all_cards, &load_deck(path)),
        None => all_cards.values().collect(),
    };
    let opponent_inventory_cards: Vec<&Card> = match opponent_deck_path {
        Some(path) => card_ids_to_card_refs(all_cards, &load_deck(path)),
        None => all_cards.values().collect(),
    };

    let mut player_won_cnt = 0;
    let mut opponent_won_cnt = 0;
    let mut draw_cnt = 0;
    for n in 0..play_cnt {
        let (p, o) = run(
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
        Commands::TrainDeck { train_cnt: _ } => {}
    }
}
