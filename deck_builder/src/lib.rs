use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::PathBuf,
};

use clap::{ValueHint, Parser};
use log::*;
use more_asserts::assert_le;
use players::PlayerType;
use rand::{prelude::Distribution, seq::IteratorRandom, Rng};
use rand_distr::{WeightedAliasIndex, WeightedIndex};
use rand_mt::Mt64;

use engine::{
    Card,
    Context, Board,
};

use players::Player;

#[derive(Parser)]
pub struct DeckBuilderArgs {
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

    #[clap(long, value_parser, default_value = "random")]
    player: PlayerType,

    #[clap(long, value_parser, default_value = "random")]
    opponent: PlayerType,

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
struct Report<'b> {
    deck: &'b [Card],
    win_cnt: u32,
}

impl<'b> Report<'b> {
    fn get_weight(&self) -> u32 {
        self.win_cnt
    }
}

impl<'b> Display for Report<'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "deck: {}, win: {}",
            engine::format_cards(self.deck),
            self.win_cnt
        )
    }
}

struct DeckBuilder<'a> {
    rng: Mt64,
    context: &'a Context,
    board: Board,
    args: DeckBuilderArgs,
    inventory_cards: HashMap<u32, Card>,
}

impl<'c> DeckBuilder<'c> {
    fn new(
        context: &'c Context,
        board: Board,
        args: DeckBuilderArgs,
        inventory_cards: HashMap<u32, Card>,
    ) -> DeckBuilder<'c> {
        DeckBuilder {
            rng: Mt64::new(42),
            context,
            board,
            args,
            inventory_cards,
        }
    }

    fn run_battles(
        &mut self,
        battle_count: usize,
        player_deck: &[Card],
        opponent_deck: &[Card],
        player: &mut dyn Player,
        opponent: &mut dyn Player,
    ) -> (u32, u32, u32) {
        let mut player_won_cnt = 0;
        let mut opponent_won_cnt = 0;
        let mut draw_cnt = 0;

        for _i in 0..battle_count {
            let (p, o) = local::run(
                self.context,
                &self.board,
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
        population: &'b [Vec<Card>],
        opponent_deck: &'b [Card],
        player: &mut dyn Player,
        opponent: &mut dyn Player,
    ) -> Vec<Report<'b>> {
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

    fn create_initial_population(&mut self) -> Vec<Vec<Card>> {
        let mut population: Vec<Vec<Card>> = vec![];
        for _ in 0..self.args.population_size {
            let mut deck: Vec<Card> = self
                .inventory_cards
                .values()
                .cloned()
                .choose_multiple(&mut self.rng, engine::DECK_SIZE);
            deck.sort();
            population.push(deck);
        }
        population
    }

    fn crossover<'b>(&mut self, a: &Report<'b>, b: &Report<'b>) -> Vec<Card> {
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
        let mut new_deck: Vec<Card> = vec![];
        (0..engine::DECK_SIZE).for_each(|_| {
            let dist = WeightedIndex::new(card_weights.iter().map(|e| e.1)).unwrap();
            let index: usize = dist.sample(&mut self.rng);
            let (selected_card_id, _weight) = card_weights.remove(index);
            new_deck.push(self.inventory_cards[&selected_card_id].clone());
        });
        new_deck
    }

    fn mutation(&mut self, deck: &mut [Card]) {
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
                let removing = &deck[i];
                let replacing_id: u32 = *pool.iter().choose(&mut self.rng).unwrap();

                pool.insert(removing.get_id());
                pool.remove(&replacing_id);
                debug!("swapping: from:{} to:{}", removing.get_id(), &replacing_id);

                deck[i] = self.inventory_cards[&replacing_id].clone();
                mutated = true;
            }
        });
        engine::sort_by_id(deck);
        if mutated {
            debug!("Mutated");
            debug!("    {}", engine::format_cards(deck));
        }
    }

    fn create_next_generation<'b>(&mut self, reports: &mut [Report<'b>]) -> Vec<Vec<Card>> {
        assert_eq!(self.args.population_size, reports.len());

        reports.sort_by(|a, b| b.win_cnt.cmp(&a.win_cnt));
        if log_enabled!(log::Level::Debug) {
            debug!("League result:");
            reports.iter().for_each(|r| {
                debug!("  win: {}: {}", r.win_cnt, engine::format_cards(r.deck));
            });
        }

        let mut next_gen: Vec<Vec<Card>> = vec![];

        // Choose top elites as is.
        (0..self.args.elite_count).for_each(|i| {
            let mut deck = reports[i].deck.to_vec();
            engine::sort_by_id(&mut deck);
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
                engine::format_cards(reports[a_index].deck),
            );
            debug!(
                "    #{}: {}",
                reports[b_index].win_cnt,
                engine::format_cards(reports[b_index].deck),
            );
            let mut deck = self.crossover(&reports[a_index], &reports[b_index]);
            engine::sort_by_id(&mut deck);
            debug!("Crossover result:");
            debug!("    {}", engine::format_cards(&deck));
            self.mutation(&mut deck);

            next_gen.push(deck);
        }

        assert_eq!(self.args.population_size, next_gen.len());
        next_gen
    }

    fn run(&mut self, player: &mut dyn Player, opponent: &mut dyn Player) {
        assert_le!(
            self.args.elite_count,
            self.args.population_size,
            "elite-count must be smaller than population-size"
        );

        let validation_deck = self.context.get_cards(
            &engine::load_deck(&self.args.validation_deck_path));

        let loaded_evaluation_deck: Vec<Card> = if let Some(eval_deck_path) =
            &self.args.evaluation_deck_path
        {
            self.context.get_cards(&engine::load_deck(eval_deck_path))
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
                .for_each(|(i, v)| info!("  {}: {}", i, engine::format_cards(v)));

            let evaluation_deck: &Vec<Card> = if self.args.evaluation_deck_path.is_none() {
                info!(
                    "Opponent uses the best deck: {}",
                    engine::format_cards(&population[0])
                );
                &population[0]
            } else {
                info!(
                    "Opponent uses the loaded deck: {}",
                    engine::format_cards(&loaded_evaluation_deck)
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
            info!("Board: {}", self.board.get_name());

            let next_generation = self.create_next_generation(&mut reports);
            population = next_generation;
        }
    }
}

pub fn train_deck<'p, 'c: 'p>(
    args: DeckBuilderArgs,
) {
    let all_cards = engine::load_cards(&args.card_dir);
    let board = engine::load_board(&args.board_path);

    let context = Context {
        all_cards,
        enabled_step_execution: false,
    };

    // Use fixed seed for reproducible results.
    let mut rng = Mt64::new(0x42);

    let mut player = args.player.create_player(&context, rng.next_u64());
    let mut opponent = args.opponent.create_player(&context, rng.next_u64());

    let ids = engine::load_deck(&args.inventory_path);
    let card_map = ids
        .iter()
        .map(|id| (*id, context.get_card(*id)))
        .collect();
    DeckBuilder::new(&context, board.clone(), args, card_map).run(&mut *player, &mut *opponent);
}
