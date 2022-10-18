use itertools::Itertools;
use log::*;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_mt::Mt64;

use crate::{
    engine::{
        board::Board,
        card::Card,
        game::{self, Action, Context, PlayerId},
        state::{PlayerState, State},
    },
    train::deck,
};

use super::{utils, Player};

pub struct MctsPlayer {
    player_id: PlayerId,
    rng: Mt64,
}

impl MctsPlayer {
    pub fn new(seed: u64) -> Self {
        MctsPlayer {
            player_id: PlayerId::Player,
            rng: Mt64::new(seed),
        }
    }
}

impl Player for MctsPlayer {
    fn init_game(&mut self, player_id: PlayerId, _board: &Board) {
        self.player_id = player_id;
    }

    fn get_deck<'a>(&mut self, inventory_cards: &[&'a Card]) -> Vec<&'a Card> {
        if inventory_cards.len() == game::DECK_SIZE {
            return inventory_cards.to_vec();
        }
        let mut v = inventory_cards.to_vec();
        let (deck, _) = v.partial_shuffle(&mut self.rng, game::DECK_SIZE);
        deck.to_vec()
    }

    fn need_redeal_hands(&mut self, _dealed_cards: &[&Card]) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn get_action<'a>(&mut self, state: &State, player_state: &'a PlayerState) -> Action<'a> {
        let mut actions_buffer: Vec<Action> = vec![];
        utils::append_valid_actions(
            state,
            player_state.get_hands(),
            self.player_id,
            &mut actions_buffer,
        );
        debug!("Got {} valid actions", actions_buffer.len());
        let index = self.rng.gen_range(0..actions_buffer.len());
        actions_buffer.remove(index)
    }
}

#[derive(Clone, Debug)]
struct Statistic {
    win_cnt: u32,
    lose_cnt: u32,
    draw_cnt: u32,
}

impl Default for Statistic {
    fn default() -> Self {
        Self {
            win_cnt: 0,
            lose_cnt: 0,
            draw_cnt: 0,
        }
    }
}

#[derive(Debug)]
enum ChanceAction<'c> {
    DealInitialHand(PlayerId, [&'c Card; game::HAND_SIZE]),
    DealCard(PlayerId, &'c Card),
}

#[derive(Debug)]
enum NodeAction<'c> {
    PlayerAction(Action<'c>),
    ChanceAction(ChanceAction<'c>),
}

impl<'c> From<ChanceAction<'c>> for NodeAction<'c> {
    fn from(c: ChanceAction<'c>) -> Self {
        NodeAction::ChanceAction(c)
    }
}

impl<'c> From<Action<'c>> for NodeAction<'c> {
    fn from(a: Action<'c>) -> Self {
        NodeAction::PlayerAction(a)
    }
}

struct DealingState<'c> {
    deck: Vec<&'c Card>,
    hands: Vec<&'c Card>,

    confirmed: bool, // it  must be true while playing a game.
}

impl<'c> DealingState<'c> {
    fn new_init(deck: &[&'c Card]) -> Self {
        DealingState {
            deck: deck.to_vec(),
            hands: vec![],
            confirmed: false,
        }
    }
}

struct Preparing<'c> {
    board: Board,
    player: DealingState<'c>,
    opponent: DealingState<'c>,
}

struct Running<'c> {
    pub state: State,

    // The game is simultaneous game so it doesn't make sense for sure.
    // However this Player implements a naive MCTS so that we can compare strength with other implementations.
    pub next_player: PlayerId,

    pub player_state: PlayerState<'c>,

    /// The player doesn't know the opponent's state of course.
    pub determined_opponent_state: PlayerState<'c>,
}

impl<'c> Running<'c> {
    fn new(
        state: State,
        next_player: PlayerId,
        player_state: PlayerState<'c>,
        determined_opponent_state: PlayerState<'c>,
    ) -> Self {
        Self {
            state,
            next_player,
            player_state,
            determined_opponent_state,
        }
    }
}

struct CardsState<'c> {
    // It must be sorted by card IDs.
    pub hands: Vec<&'c Card>,

    // It must be shuffled. Chance node deals a card from the top
    // of the deck.
    pub deck: Vec<&'c Card>,
}

impl<'c> CardsState<'c> {
    fn new(hands: Vec<&'c Card>, deck: Vec<&'c Card>) -> Self {
        CardsState { hands, deck }
    }
}

/// Game state which is visible from a player.
/// It includes presumed information (e.g. opponent's hand/deck)
struct Determinization<'c> {
    player_cards: CardsState<'c>,
    opponent_cards: CardsState<'c>,
}

impl<'c> Determinization<'c> {
    fn new(player_cards: CardsState<'c>, opponent_cards: CardsState<'c>) -> Self {
        Determinization {
            player_cards,
            opponent_cards,
        }
    }
}

struct Node<'c> {
    state: State,

    statistic: Statistic,
    visit_count: u32,

    legal_actions: Vec<NodeAction<'c>>,
}

impl<'c> Node<'c> {
    fn new(state: State, legal_actions: Vec<NodeAction<'c>>) -> Self {
        Self {
            state,
            statistic: Statistic::default(),
            visit_count: 0,
            legal_actions,
        }
    }
}

struct Traverser<'c> {
    context: &'c Context,
    player_initial_deck: Vec<&'c Card>,
    rng: Mt64,
}

impl<'c> Traverser<'c> {
    fn new(context: &'c Context, player_initial_deck: Vec<&'c Card>, seed: u64) -> Self {
        Self {
            context,
            player_initial_deck,
            rng: Mt64::new(seed),
        }
    }

    fn traverse(
        &mut self,
        state: &State,
        player_state: &PlayerState<'c>,
        opponent_deck: &[&'c Card],
    ) {
        const N: usize = 2;
        let root_node = self.create_root_node(state, player_state);
        let mut node = &root_node;
        for n in 0..N {
            let determinization = Determinization::new(
                self.determinize_player_deck(state, player_state),
                self.determinize_opponent_deck(state),
            );
            //let action = self.select_action(n, node);
        }
    }

    // fn playout(node: &Node, action: Action) {}

    // Choose an action based on UCB1.
    //fn select_action(&self, total_simulation_cnt: u32, node: &Node) -> NodeAction {
    //let actions = node.legal_actions;
    //}

    fn precalculate_valid_actions_for_root(
        &self,
        state: &State,
        player_state: &PlayerState<'c>,
    ) -> Vec<NodeAction<'c>> {
        let mut valid_actions = vec![];
        utils::append_valid_actions(
            state,
            player_state.get_hands(),
            PlayerId::Player,
            &mut valid_actions,
        );
        valid_actions
            .iter()
            .map(|a| NodeAction::PlayerAction(*a))
            .collect()
    }

    fn precalculate_valid_actions(
        &self,
        state: &State,
        cards_state: &CardsState<'c>,
        player_id: PlayerId,
    ) -> Vec<NodeAction<'c>> {
        let mut valid_actions = vec![];

        utils::append_valid_actions(state, &cards_state.hands, player_id, &mut valid_actions);

        if player_id == PlayerId::Opponent {
            // For the opponent player, list up actions also for cards in deck so that
            // future playouts (which may have different determinized hands) can refer the precalculated actions.
            utils::append_valid_actions(state, &cards_state.deck, player_id, &mut valid_actions);
        }

        valid_actions
            .iter()
            .map(|a| NodeAction::PlayerAction(*a))
            .collect()
    }

    fn filter_cards(cards: &mut Vec<&'c Card>, remove_card_ids: &[u32]) {
        let expected_len = cards.len() - remove_card_ids.len();
        remove_card_ids.iter().for_each(|r| {
            let remove_id = r;
            for i in 0..cards.len() {
                if cards[i].get_id() == *r {
                    cards.swap_remove(i);
                    break;
                }
            }
        });
        assert_eq!(expected_len, cards.len());
    }

    fn determinize_opponent_deck(&mut self, state: &State) -> CardsState<'c> {
        let mut all_cards = self.context.all_cards.values().collect_vec();
        Self::filter_cards(&mut all_cards, &state.opponent_consumed_cards);

        all_cards.shuffle(&mut self.rng);
        let (hands, deck) = all_cards.split_at(game::HAND_SIZE);

        CardsState::new(hands.to_vec(), deck.to_vec())
    }

    fn determinize_player_deck(
        &mut self,
        state: &State,
        player_state: &PlayerState<'c>,
    ) -> CardsState<'c> {
        let mut deck_cards = self.player_initial_deck.clone();

        let hand_ids: Vec<u32> = player_state
            .get_hands()
            .iter()
            .map(|c| c.get_id())
            .collect();
        Self::filter_cards(&mut deck_cards, &hand_ids);

        Self::filter_cards(&mut deck_cards, &state.player_consumed_cards);

        // Shuffle cards in deck since the player don't know the order of deck.
        deck_cards.shuffle(&mut self.rng);
        CardsState::new(player_state.get_hands().to_vec(), deck_cards)
    }

    /// Creates a node from an actual game state (visible from the player).
    /// It determines the opponent's state (i.e. hands and cards in their deck) so that
    /// the traverser can playout the game.
    /// Note that we should shuffle deck ONLY at the root node so
    fn create_root_node<'a>(&mut self, state: &State, player_state: &PlayerState<'c>) -> Node<'c> {
        Node::new(
            state.clone(),
            self.precalculate_valid_actions_for_root(state, player_state),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::engine::{
        board,
        state::{self, tests::new_test_card},
    };

    use super::*;

    fn new_test_all_cards(card_strs: &[&[&str]]) -> HashMap<u32, Card> {
        let mut tmp: HashMap<u32, Card> = HashMap::new();
        card_strs.iter().enumerate().for_each(|(i, s)| {
            const SPECIAL_COST: i32 = 10;
            tmp.insert(
                i as u32,
                state::tests::new_test_card_impl(s, i as u32, SPECIAL_COST),
            );
        });
        tmp
    }

    #[test]
    fn test_initial_node() {
        #[rustfmt::skip]
        let all_cards = new_test_all_cards(&[
            &[
                "===="
            ],
            &[
                "====="
            ],
            &[
                "======"
            ],
            &[
                "=",
                "==",
            ],
            &[
                "=",
                "=",
                "===",
            ],
        ]);
        #[rustfmt::skip]
        let board = board::load_board_from_lines(
            String::from("test_board"),
            &[
            "#####",
            "#.O##",
            "#..P#",
            "#####"
            ]);
        let context = Context {
            board,
            all_cards,
            enabled_step_execution: false,
        };
        const SEED: u64 = 42;
        let player_initial_deck = vec![
            context.card_ref(0),
            context.card_ref(1),
            context.card_ref(2),
            context.card_ref(3),
            context.card_ref(4),
        ];
        let player_hands = vec![
            context.card_ref(0),
            context.card_ref(1),
            context.card_ref(2),
            context.card_ref(3),
        ];
        let player_deck = vec![context.card_ref(4)];

        let mut traverser = Traverser::new(&context, player_initial_deck, SEED);

        let state = State::new(context.board.clone(), 0, 0, 0, vec![], vec![]);
        let player_state = PlayerState::new(&player_hands, &player_deck);

        let root_node = traverser.create_root_node(&state, &player_state);

        // 1: Put(3)
        // 4: Pass * HAND_SIZE
        assert_eq!(
            5,
            root_node.legal_actions.len(),
            "{:?}",
            root_node.legal_actions
        );
    }
}
