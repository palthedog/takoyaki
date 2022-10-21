use std::path::PathBuf;

use clap::{Args, ValueHint};
use log::*;
use rand::seq::SliceRandom;
use rand_mt::Mt64;

use crate::{
    engine::{
        card::{self, Card},
        game::{self, Context},
    },
    players::Player,
    runner,
};

#[derive(Args)]
pub struct PlayArgs {
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
}

pub fn run_rand<'c>(
    context: &'c Context,
    player: &mut dyn Player<'c>,
    opponent: &mut dyn Player<'c>,
    args: PlayArgs,
) {
    let play_cnt: u32 = args.play_cnt;
    let player_deck_path: Option<PathBuf> = args.player_deck_path;
    let opponent_deck_path: Option<PathBuf> = args.opponent_deck_path;

    // Use fixed seed for reproducible results.
    let mut rng = Mt64::new(0x42);

    let mut player_inventory_cards: Vec<&Card> = match &player_deck_path {
        Some(path) => card::card_ids_to_card_refs(&context.all_cards, &card::load_deck(path)),
        None => context.all_cards.values().collect(),
    };
    let mut opponent_inventory_cards: Vec<&Card> = match &opponent_deck_path {
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
