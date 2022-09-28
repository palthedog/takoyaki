use std::{collections::HashMap, fmt::Display, path::PathBuf};

use clap::{Args, ValueHint};
use itertools::Itertools;
use log::*;
use more_asserts::assert_le;
use rand::{prelude::Distribution, seq::IteratorRandom};
use rand_distr::WeightedAliasIndex;
use rand_mt::Mt64;

use crate::{
    engine::{
        board::Board,
        card::{self, Card},
        game,
    },
    players::random::RandomPlayer,
    runner,
};

#[derive(Args)]
pub struct TrainDeckArgs {
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

    #[clap(long, short = 'g', value_parser, default_value_t = 1)]
    max_generation: u32,

    /// How many battles should be held for each epoch.
    /// Note that specified amount of battles happen for each deck variations so
    /// `C(variations, 2) * battles_per_epoch` battle simulations happen for each epoch.
    #[clap(long, short = 'b', value_parser, default_value_t = 1)]
    battles_per_epoch: usize,

    /// How many deck variations should be made for each epoch.
    #[clap(long, short = 'p', value_parser, default_value_t = 10)]
    population_size: usize,

    /// Top `elite_count` genes are always inherited to the next generation.
    /// Rest of the population is filled by crossover/mutation.
    #[clap(long, short, value_parser, default_value_t = 3)]
    elite_count: usize,

    #[clap(long, short, value_parser, default_value_t = 0.01)]
    mutation_rate: f32,
}

#[derive(Debug)]
struct Report<'a, 'b> {
    deck: &'b [&'a Card],
    win_cnt: u32,
}

impl<'a, 'b> Report<'a, 'b> {
    fn get_weight(&self) -> u32 {
        self.win_cnt
    }
}

impl<'a, 'b> Display for Report<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "deck: {}, win: {}",
            Card::format_cards(self.deck),
            self.win_cnt
        )
    }
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

    fn run_battles(
        &mut self,
        board: &Board,
        player_deck: &[&'a Card],
        opponent_deck: &[&'a Card],
    ) -> (u32, u32, u32) {
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

    fn run_league<'b>(
        &mut self,
        board: &Board,
        population: &'b [Vec<&'a Card>],
    ) -> Vec<Report<'a, 'b>> {
        assert_eq!(self.args.population_size, population.len());

        // key: variation_index
        // value: won count
        let mut won_cnts: HashMap<usize, u32> = HashMap::new();
        (0..population.len()).combinations(2).for_each(|pair| {
            let p_deck_index = pair[0];
            let o_deck_index = pair[1];
            debug!(
                "Start running battles: {} v.s. {}",
                p_deck_index, o_deck_index
            );
            let (p, o, _draw) =
                self.run_battles(board, &population[p_deck_index], &population[o_deck_index]);
            *won_cnts.entry(p_deck_index).or_insert(0) += p;
            *won_cnts.entry(o_deck_index).or_insert(0) += o;
        });

        won_cnts
            .iter()
            .map(|(index, cnt)| Report {
                deck: &population[*index],
                win_cnt: *cnt,
            })
            .collect()
    }

    fn create_initial_population(&mut self) -> Vec<Vec<&'a Card>> {
        let mut population: Vec<Vec<&Card>> = vec![];
        for _ in 0..self.args.population_size {
            let mut deck: Vec<&Card> = self
                .inventory_cards
                .iter()
                .map(|r| *r)
                .choose_multiple(&mut self.rng, game::DECK_SIZE);
            deck.sort();
            population.push(deck);
        }
        population
    }

    fn crossover<'b>(&mut self, a: &Report<'a, 'b>, b: &Report<'a, 'b>) -> Vec<&'a Card> {
        // key: card
        // value: weight
        let mut cards: HashMap<u32, (&Card, u32)> = HashMap::new();
        a.deck.iter().for_each(|card| {
            cards.insert(card.get_id(), (card, a.get_weight()));
        });
        b.deck.iter().for_each(|card| {
            let e = cards.entry(card.get_id()).or_insert((card, 0));
            e.1 += b.get_weight();
        });

        debug!("Weighted cards: # of cards: {}", cards.len());
        cards.values().for_each(|(c, w)| {
            debug!("    w: {}: {}", w, c.fmt_short());
        });

        let entries: Vec<(&u32, &(&Card, u32))> = cards.iter().collect();
        let dist = WeightedAliasIndex::new(entries.iter().map(|e| *e.0).collect()).unwrap();
        dist.sample_iter(&mut self.rng)
            .take(game::DECK_SIZE)
            .map(|index| entries[index].1 .0)
            .collect()
    }

    fn create_next_generation<'b>(&mut self, reports: &mut [Report<'a, 'b>]) -> Vec<Vec<&'a Card>> {
        assert_eq!(self.args.population_size, reports.len());

        reports.sort_by(|a, b| b.win_cnt.cmp(&a.win_cnt));
        info!("League result:");
        reports.iter().for_each(|r| {
            info!("  win: {}: {}", r.win_cnt, Card::format_cards(r.deck));
        });

        let mut next_gen: Vec<Vec<&'a Card>> = vec![];

        // Choose top elites as is.
        (0..self.args.elite_count).for_each(|i| {
            next_gen.push(reports[i].deck.to_vec());
        });

        // let weights = WeightedIndex::new(reports.iter().map(|r| r.get_weight())).unwrap();
        // We use WeightedAliasIndex instead of WeightedIndex becaues we'll take 2*N genes here.
        // Initialization cost + taking costs would be:
        //   WeightedIndex: N * O(logN) => O(NlogN)
        //   WeightedAliasIndex: O(N) + N * O(1) => O(N)
        let weights =
            WeightedAliasIndex::new(reports.iter().map(|r| r.get_weight()).collect()).unwrap();
        while next_gen.len() < self.args.population_size {
            let a_index = weights.sample(&mut self.rng);
            let b_index = weights.sample(&mut self.rng);
            debug!("Crossover");
            debug!(
                "    #{}: {}",
                reports[a_index].win_cnt,
                Card::format_cards(reports[a_index].deck),
            );
            debug!(
                "    #{}: {}",
                reports[b_index].win_cnt,
                Card::format_cards(reports[b_index].deck),
            );
            let deck = self.crossover(&reports[a_index], &reports[b_index]);
            debug!("Crossover result:");
            debug!("    {}", Card::format_cards(&deck));

            // TODO: mutation

            next_gen.push(deck);
        }

        assert_eq!(self.args.population_size, next_gen.len());
        next_gen
    }

    fn run(&mut self, _all_cards: &HashMap<u32, Card>, board: &Board) {
        assert_le!(
            self.args.elite_count,
            self.args.population_size,
            "elite-count must be smaller than population-size"
        );

        let mut population = self.create_initial_population();
        let max_epoch = self.args.max_generation;
        let battles_count = self.args.battles_per_epoch
            * self.args.population_size
            * (self.args.population_size - 1)
            / 2;
        for n in 0..max_epoch {
            info!("# Generation {}", n);
            population
                .iter()
                .enumerate()
                .for_each(|(i, v)| info!("  {}: {}", i, Card::format_cards(&v)));
            info!("Running  {} battles...", battles_count);
            let mut reports = self.run_league(board, &population);
            let next_generation = self.create_next_generation(&mut reports);
            population = next_generation;
        }
    }
}

pub fn train_deck(all_cards: &HashMap<u32, Card>, board: &Board, args: TrainDeckArgs) {
    let inventory_cards =
        card::card_ids_to_card_refs(all_cards, &card::load_deck(&args.inventory_path));
    TrainDeck::new(args, inventory_cards).run(all_cards, board);
}
