use std::{collections::HashMap, path::PathBuf};

use clap::{Args, ValueHint};
use itertools::Itertools;
use log::*;
use rand::seq::IteratorRandom;
use rand_mt::Mt64;

use crate::{
    engine::{
        board::Board,
        card::{self, Card},
        game,
    },
    players::{random::RandomPlayer, Player},
    runner,
};

#[derive(Args)]
pub struct TrainDeckArgs {
    #[clap(long, short = 'e', value_parser, default_value_t = 1)]
    max_epoch: u32,

    /// How many battles should be held for each epoch.
    /// Note that specified amount of battles happen for each deck variations so
    /// `C(variations, 2) * battles_per_epoch` battle simulations happen for each epoch.
    #[clap(long, short = 'b', value_parser, default_value_t = 1)]
    battles_per_epoch: usize,

    /// How many deck variations should be made for each epoch.
    #[clap(long, short, value_parser, default_value_t = 4)]
    variations_count: u32,

    /// a path to a deck file which describes the list of cards you already have.
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
    )]
    inventory_path: PathBuf,

    /// a path to a deck file where we start training from
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
    )]
    checkpoint_deck_path: Option<PathBuf>,
}

struct TrainDeck<'a> {
    rng: Mt64,
    args: TrainDeckArgs,
    inventory_cards: Vec<&'a Card>,
    player: RandomPlayer,
    opponent: RandomPlayer,
}

impl<'a> TrainDeck<'a> {
    fn new(args: TrainDeckArgs, inventory_cards: Vec<&'a Card>) -> TrainDeck<'a> {
        let mut rng = Mt64::new(42);
        let p_seed = rng.next_u64();
        let o_seed = rng.next_u64();
        TrainDeck {
            rng,
            args,
            inventory_cards,
            player: RandomPlayer::new(p_seed),
            opponent: RandomPlayer::new(o_seed),
        }
    }

    fn create_initial_variations(&mut self) -> Vec<Vec<&'a Card>> {
        let mut variations: Vec<Vec<&Card>> = vec![];
        for _ in 0..self.args.variations_count {
            let mut deck: Vec<&Card> = self
                .inventory_cards
                .iter()
                .map(|r| *r)
                .choose_multiple(&mut self.rng, game::DECK_SIZE);
            deck.sort();
            variations.push(deck);
        }
        variations
    }

    fn run_battles(
        &mut self,
        board: &Board,
        player_deck: &[&'a Card],
        opponent_deck: &[&'a Card],
    ) -> (i32, i32, i32) {
        let mut player_won_cnt = 0;
        let mut opponent_won_cnt = 0;
        let mut draw_cnt = 0;
        (0..self.args.battles_per_epoch).for_each(|_| {
            let (p, o) = runner::run(
                board,
                player_deck,
                opponent_deck,
                &mut self.player,
                &mut self.opponent,
                &mut self.rng,
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
        });
        (player_won_cnt, opponent_won_cnt, draw_cnt)
    }

    fn run_league(&mut self, board: &Board, variations: &[Vec<&'a Card>]) {
        // key: variation_index
        // value: won count
        let mut won_cnts: HashMap<usize, i32> = HashMap::new();
        (0..variations.len()).combinations(2).for_each(|pair| {
            let p_deck_index = pair[0];
            let o_deck_index = pair[1];
            debug!(
                "Start running battles: {} v.s. {}",
                p_deck_index, o_deck_index
            );
            let (p, o, _draw) =
                self.run_battles(board, &variations[p_deck_index], &variations[o_deck_index]);
            *won_cnts.entry(p_deck_index).or_insert(0) += p;
            *won_cnts.entry(o_deck_index).or_insert(0) += o;
        });

        debug!("League result: {:?}", won_cnts);
    }

    fn run(&mut self, _all_cards: &HashMap<u32, Card>, board: &Board) {
        let variations = self.create_initial_variations();
        debug!("Initial variations:");
        variations
            .iter()
            .enumerate()
            .for_each(|(i, v)| info!("  {}: {}", i, Card::format_cards(&v)));

        let max_epoch = self.args.max_epoch;
        for n in 0..max_epoch {
            self.run_league(board, &variations);
            if n % 100 == 0 {
                println!(" #{}", n);
            }
        }
    }
}

pub fn train_deck(all_cards: &HashMap<u32, Card>, board: &Board, args: TrainDeckArgs) {
    let inventory_cards =
        card::card_ids_to_card_refs(all_cards, &card::load_deck(&args.inventory_path));
    TrainDeck::new(args, inventory_cards).run(all_cards, board);
}
