use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    fmt::Display,
    path::PathBuf,
};

use clap::{Args, ValueHint};
use log::*;
use more_asserts::assert_le;
use rand::{prelude::Distribution, seq::IteratorRandom, Rng};
use rand_distr::{WeightedAliasIndex, WeightedIndex};
use rand_mt::Mt64;

use crate::{
    engine::{
        card::{self, Card},
        game::{self, Context},
    },
    players::{random::RandomPlayer, Player, PlayerType},
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

    /// a path to a deck file used by the opponent for evaluation.
    /// if not specified, best deck from the previous generation is used.
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
    )]
    evaluation_deck_path: Option<PathBuf>,

    /// a path to a deck file used by the opponent for validation.
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
        default_value = "data/decks/starter"
    )]
    validation_deck_path: PathBuf,

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
    #[clap(long, value_parser, default_value_t = 3)]
    elite_count: usize,

    #[clap(long, short, value_parser, default_value_t = 0.01)]
    mutation_rate: f64,
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
    context: &'a Context,
    args: TrainDeckArgs,
    inventory_cards: HashMap<u32, &'a Card>,
}

impl<'c> TrainDeck<'c> {
    fn new(
        context: &'c Context,
        args: TrainDeckArgs,
        inventory_cards: HashMap<u32, &'c Card>,
    ) -> TrainDeck<'c> {
        TrainDeck {
            rng: Mt64::new(42),
            context,
            args,
            inventory_cards,
        }
    }

    fn run_battles(
        &mut self,
        battle_count: usize,
        player_deck: &[&'c Card],
        opponent_deck: &[&'c Card],
        player: &mut dyn Player<'c>,
        opponent: &mut dyn Player<'c>,
    ) -> (u32, u32, u32) {
        let mut player_won_cnt = 0;
        let mut opponent_won_cnt = 0;
        let mut draw_cnt = 0;

        for i in 0..battle_count {
            let (p, o) = runner::run(
                self.context,
                player_deck,
                opponent_deck,
                player,
                opponent,
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
        }
        (player_won_cnt, opponent_won_cnt, draw_cnt)
    }

    fn evaluate_population<'b>(
        &mut self,
        population: &'b [Vec<&'c Card>],
        opponent_deck: &'b [&'c Card],
        player: &mut dyn Player<'c>,
        opponent: &mut dyn Player<'c>,
    ) -> Vec<Report<'c, 'b>> {
        // key: variation_index
        // value: won count
        let mut won_cnts: HashMap<usize, u32> = HashMap::new();
        (0..population.len()).for_each(|p_deck_index| {
            let player_deck = &population[p_deck_index];
            let (win, _lose, _draw) = self.run_battles(
                self.args.battles_per_epoch,
                player_deck,
                opponent_deck,
                player,
                opponent,
            );
            *won_cnts.entry(p_deck_index).or_insert(0) += win;
        });
        won_cnts
            .iter()
            .map(|(index, cnt)| Report {
                deck: &population[*index],
                win_cnt: *cnt,
            })
            .collect()
    }

    fn create_initial_population(&mut self) -> Vec<Vec<&'c Card>> {
        let mut population: Vec<Vec<&Card>> = vec![];
        for _ in 0..self.args.population_size {
            let mut deck: Vec<&Card> = self
                .inventory_cards
                .values()
                .copied()
                .choose_multiple(&mut self.rng, game::DECK_SIZE);
            deck.sort();
            population.push(deck);
        }
        population
    }

    fn crossover<'b>(&mut self, a: &Report<'c, 'b>, b: &Report<'c, 'b>) -> Vec<&'c Card> {
        // key: card id
        // value: weight
        let mut card_weights: HashMap<u32, u32> = HashMap::new();
        a.deck.iter().for_each(|card| {
            card_weights.insert(card.get_id(), a.get_weight());
        });
        b.deck.iter().for_each(|card| {
            let e = card_weights.entry(card.get_id()).or_insert(0);
            *e += b.get_weight();
        });

        if log_enabled!(log::Level::Debug) {
            debug!("Weighted cards: # of cards: {}", card_weights.len());
            card_weights.iter().for_each(|(id, w)| {
                debug!("    w: {}: {}", w, id);
            });
        }

        let mut card_weights: Vec<(u32, u32)> =
            card_weights.iter().map(|(k, v)| (*k, *v)).collect();
        let mut new_deck: Vec<&Card> = vec![];
        (0..game::DECK_SIZE).for_each(|_| {
            let dist = WeightedIndex::new(card_weights.iter().map(|e| e.1)).unwrap();
            let index: usize = dist.sample(&mut self.rng);
            let (selected_card_id, _weight) = card_weights.remove(index);
            new_deck.push(self.inventory_cards[&selected_card_id]);
        });
        new_deck
    }

    fn mutation(&mut self, deck: &mut [&'c Card]) {
        let mut pool: HashSet<u32> = HashSet::new();
        self.inventory_cards.keys().for_each(|card_id| {
            pool.insert(*card_id);
        });

        deck.iter().for_each(|card| {
            pool.remove(&card.get_id());
        });

        debug!("Pool: {:?}", pool);

        let mut mutated = false;
        (0..deck.len()).for_each(|i| {
            if self.rng.gen_bool(self.args.mutation_rate) {
                let removing = deck[i];
                let replacing_id: u32 = *pool.iter().choose(&mut self.rng).unwrap();

                pool.insert(removing.get_id());
                pool.remove(&replacing_id);
                debug!("swapping: from:{} to:{}", removing.get_id(), replacing_id);

                deck[i] = self.inventory_cards[&replacing_id];
                mutated = true;
            }
        });
        Card::sort_by_id(deck);
        if mutated {
            debug!("Mutated");
            debug!("    {}", Card::format_cards(deck));
        }
    }

    fn create_next_generation<'b>(&mut self, reports: &mut [Report<'c, 'b>]) -> Vec<Vec<&'c Card>> {
        assert_eq!(self.args.population_size, reports.len());

        reports.sort_by(|a, b| b.win_cnt.cmp(&a.win_cnt));
        if log_enabled!(log::Level::Debug) {
            debug!("League result:");
            reports.iter().for_each(|r| {
                debug!("  win: {}: {}", r.win_cnt, Card::format_cards(r.deck));
            });
        }

        let mut next_gen: Vec<Vec<&'c Card>> = vec![];

        // Choose top elites as is.
        (0..self.args.elite_count).for_each(|i| {
            let mut deck = reports[i].deck.to_vec();
            Card::sort_by_id(&mut deck);
            next_gen.push(deck);
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
            let mut deck = self.crossover(&reports[a_index], &reports[b_index]);
            Card::sort_by_id(&mut deck);
            debug!("Crossover result:");
            debug!("    {}", Card::format_cards(&deck));
            self.mutation(&mut deck);

            next_gen.push(deck);
        }

        assert_eq!(self.args.population_size, next_gen.len());
        next_gen
    }

    fn run(&mut self, player: &mut dyn Player<'c>, opponent: &mut dyn Player<'c>) {
        assert_le!(
            self.args.elite_count,
            self.args.population_size,
            "elite-count must be smaller than population-size"
        );

        let validation_deck = card::card_ids_to_card_refs(
            &self.context.all_cards,
            &card::load_deck(&self.args.validation_deck_path),
        );

        let loaded_evaluation_deck: Vec<&Card> = if let Some(eval_deck_path) =
            &self.args.evaluation_deck_path
        {
            card::card_ids_to_card_refs(&self.context.all_cards, &card::load_deck(eval_deck_path))
        } else {
            // it's not used.
            vec![]
        };

        let mut population = self.create_initial_population();
        let max_epoch = self.args.max_generation;
        let battles_count = self.args.battles_per_epoch * self.args.population_size;
        for n in 0..max_epoch {
            info!("# Generation {}", n);
            info!("Best {}", self.args.elite_count);
            population
                .iter()
                .enumerate()
                .take(self.args.elite_count)
                .for_each(|(i, v)| info!("  {}: {}", i, Card::format_cards(v)));

            let evaluation_deck: &Vec<&Card> = if self.args.evaluation_deck_path.is_none() {
                info!(
                    "Opponent uses the best deck: {}",
                    Card::format_cards(&population[0])
                );
                &population[0]
            } else {
                info!(
                    "Opponent uses the loaded deck: {}",
                    Card::format_cards(&loaded_evaluation_deck)
                );
                &loaded_evaluation_deck
            };

            info!("Running  {} battles...", battles_count);
            let mut reports =
                self.evaluate_population(&population, evaluation_deck, player, opponent);

            // Validation
            info!("Validating...");
            let best_deck = &reports
                .iter()
                .max_by(|a, b| a.win_cnt.cmp(&b.win_cnt))
                .unwrap()
                .deck;
            let (w, l, d) = self.run_battles(1000, best_deck, &validation_deck, player, opponent);
            info!("Validation: Win rate: {:.3}", w as f64 / (w + l + d) as f64);
            info!("Board: {}", self.context.board.get_name());

            let next_generation = self.create_next_generation(&mut reports);
            population = next_generation;
        }
    }
}

pub fn train_deck<'p, 'c: 'p>(
    context: &'c Context,
    player: &mut dyn Player<'c>,
    opponent: &mut dyn Player<'c>,
    args: TrainDeckArgs,
) {
    let inventory_cards =
        card::card_ids_to_card_map(&context.all_cards, &card::load_deck(&args.inventory_path));
    TrainDeck::new(context, args, inventory_cards).run(player, opponent);
}
