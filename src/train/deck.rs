use std::{collections::HashMap, path::PathBuf};

use clap::{Args, ValueHint};
use log::*;
use rand::seq::IteratorRandom;
use rand_mt::Mt64;

use crate::engine::{
    board::Board,
    card::{self, Card},
    game,
};

#[derive(Args)]
pub struct TrainDeckArgs {
    #[clap(long, short = 'e', value_parser, default_value_t = 1)]
    max_epoch: u32,

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
}

impl<'a> TrainDeck<'a> {
    fn new(args: TrainDeckArgs, inventory_cards: Vec<&'a Card>) -> TrainDeck<'a> {
        TrainDeck {
            rng: Mt64::new(42),
            args,
            inventory_cards,
        }
    }

    fn create_initial_variations(&mut self) -> Vec<Vec<&Card>> {
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

    fn run(&mut self, _all_cards: &HashMap<u32, Card>, _board: &Board) {
        let variations = self.create_initial_variations();
        debug!("Initial variations:");
        variations
            .iter()
            .enumerate()
            .for_each(|(i, v)| info!("  {}: {}", i, Card::format_cards(&v)));

        for n in 0..self.args.max_epoch {
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
