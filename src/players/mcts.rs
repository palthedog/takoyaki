use itertools::Itertools;
use log::debug;
use more_asserts::*;
use once_cell::sync::OnceCell;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_mt::Mt64;

use std::cell::RefCell;

use crate::engine::{
    board::Board,
    card::Card,
    game::{self, Action, Context, PlayerId},
    state::{self, update_state, PlayerCardState, State},
};

use super::{
    utils::{self, append_valid_actions},
    Player,
};

pub struct MctsPlayer<'c> {
    player_id: PlayerId,
    traverser: Traverser<'c>,
    rng: Mt64,
}

impl<'c> MctsPlayer<'c> {
    pub fn new(context: &'c Context, deck: Vec<&'c Card>, seed: u64) -> Self {
        let mut rng = Mt64::new(seed);
        MctsPlayer {
            player_id: PlayerId::Player,
            traverser: Traverser::new(context, deck, rng.next_u64()),
            rng,
        }
    }
}

impl<'c> Player<'c> for MctsPlayer<'c> {
    fn init_game(&mut self, player_id: PlayerId, _board: &Board) {
        self.player_id = player_id;
    }

    fn need_redeal_hands(&mut self, _dealed_cards: &[&'c Card]) -> bool {
        self.rng.gen_bool(0.5)
    }

    fn get_action<'a>(&mut self, state: &State, hands: &[&'c Card]) -> Action<'c> {
        self.traverser.traverse(state, hands, 2);

        todo!("WIP");
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Statistic {
    total_cnt: i32,
    win_cnt: i32,
    lose_cnt: i32,
    draw_cnt: i32,
    value: i32,
}

impl Default for Statistic {
    fn default() -> Self {
        Self {
            total_cnt: 0,
            win_cnt: 0,
            lose_cnt: 0,
            draw_cnt: 0,
            value: 0,
        }
    }
}

impl Statistic {
    fn update_with(&mut self, (p, o): (i32, i32)) {
        self.total_cnt += 1;
        if p == o {
            self.draw_cnt += 1
        } else if p > o {
            self.win_cnt += 1
        } else {
            self.lose_cnt += 1
        }
        self.value += p - o;
    }

    fn get_expected_value(&self) -> f64 {
        self.value as f64 / self.total_cnt as f64
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ChanceAction<'c> {
    DealInitialHand(PlayerId, [&'c Card; game::HAND_SIZE]),
    DealCard(PlayerId, &'c Card),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum NodeAction<'c> {
    Root,
    PlayerAction(PlayerId, Action<'c>),
    ChanceAction(ChanceAction<'c>),
}

impl<'c> From<ChanceAction<'c>> for NodeAction<'c> {
    fn from(c: ChanceAction<'c>) -> Self {
        NodeAction::ChanceAction(c)
    }
}

/// Game state which is visible from a player.
/// It includes presumed information (e.g. opponent's hand/deck)
#[derive(Debug, PartialEq)]
struct Determinization<'c> {
    player_cards: PlayerCardState<'c>,
    opponent_cards: PlayerCardState<'c>,
}

impl<'c> Determinization<'c> {
    fn new(player_cards: PlayerCardState<'c>, opponent_cards: PlayerCardState<'c>) -> Self {
        Determinization {
            player_cards,
            opponent_cards,
        }
    }
}

#[derive(Debug, PartialEq)]
struct SimultaneousState<'c> {
    player_action: Option<Action<'c>>,
    opponent_action: Option<Action<'c>>,
}

impl<'c> SimultaneousState<'c> {
    fn new() -> Self {
        Self {
            player_action: None,
            opponent_action: None,
        }
    }

    fn player_action_filled(&self) -> bool {
        self.player_action.is_some()
    }

    fn with_action(&self, node_action: NodeAction<'c>) -> Self {
        match node_action {
            NodeAction::PlayerAction(PlayerId::Player, act) => {
                assert!(self.player_action.is_none());
                assert!(self.opponent_action.is_none());

                Self {
                    player_action: Some(act),
                    opponent_action: None,
                }
            }
            NodeAction::PlayerAction(PlayerId::Opponent, act) => {
                assert!(self.player_action.is_some());
                assert!(self.opponent_action.is_none());

                Self {
                    player_action: self.player_action,
                    opponent_action: Some(act),
                }
            }
            NodeAction::ChanceAction(_) => todo!(),
            NodeAction::Root => todo!(),
        }
    }

    fn both_actions_filled(&self) -> bool {
        self.player_action.is_some() && self.opponent_action.is_some()
    }
}

#[derive(Debug, PartialEq)]
struct Node<'c> {
    state: State,
    simultaneous_state: SimultaneousState<'c>,
    action: NodeAction<'c>,

    statistic: Statistic,

    child_nodes: Vec<Node<'c>>,
    legal_actions: OnceCell<Vec<NodeAction<'c>>>,
}

impl<'c> Node<'c> {
    fn new(
        state: State,
        simultaneous_state: SimultaneousState<'c>,
        action: NodeAction<'c>,
    ) -> Self {
        assert!(
            !simultaneous_state.both_actions_filled(),
            "If all actions are filled, we should create the new node with a updated State."
        );

        Self {
            state,
            simultaneous_state,
            action,
            statistic: Statistic::default(),
            child_nodes: vec![],
            legal_actions: OnceCell::default(),
        }
    }

    fn is_terminal(&self) -> bool {
        self.state.is_end()
    }

    /// True if the node has at least one child node which we have never simulated.
    fn is_leaf(&self) -> bool {
        if self.state.is_end() {
            return true;
        }
        let legal_actions = self.legal_actions.get();
        if legal_actions.is_none() {
            return true;
        }
        legal_actions.unwrap().len() > self.child_nodes.len()
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

    /// Descend the tree until leaf/terminal node is found.
    fn select_leaf<'a>(
        &mut self,
        root_node: &'a mut Node<'c>,
        determinization: &Determinization<'c>,
    ) -> (&'a mut Node<'c>, Vec<NodeAction<'c>>) {
        let mut history = vec![];
        let mut node: &'a mut Node<'c> = root_node;
        loop {
            if node.is_leaf() {
                return (node, history);
            }
            node = self.select_child_node(node, determinization);
            history.push(node.action.clone());
        }
    }

    fn traverse(&mut self, state: &State, hands: &[&'c Card], iterations: usize) {
        let mut root_node = self.create_root_node(state);
        for n in 0..iterations {
            let determinization = Determinization::new(
                self.determinize_player_deck(&root_node.state, hands),
                self.determinize_opponent_deck(&root_node.state),
            );
            self.iterate(&mut root_node, &determinization);
        }
    }

    fn iterate(&mut self, root_node: &mut Node<'c>, determinization: &Determinization<'c>) {
        // Selection
        debug!("Selection");
        let (mut leaf, mut history) = self.select_leaf(root_node, determinization);

        // Expansion
        debug!("Expansion");
        let leaf = if !leaf.is_terminal() {
            let tmp = self.expand(&mut leaf, determinization);
            history.push(tmp.action.clone());
            tmp
        } else {
            leaf
        };

        // Simulation
        let result = self.playout(&leaf.state, determinization);

        // Backpropagation
        debug!("Backpropagation");
        let mut node = root_node;
        node.statistic.update_with(result);
        for i in 0..history.len() {
            node = node
                .child_nodes
                .iter_mut()
                .find(|c| c.action == history[i])
                .unwrap();
            node.statistic.update_with(result);
        }
    }

    fn playout(&mut self, state: &State, determinization: &Determinization<'c>) -> (i32, i32) {
        let mut state = state.clone();
        let mut p_state = determinization.player_cards.clone();
        let mut o_state = determinization.opponent_cards.clone();
        loop {
            if state.is_end() {
                debug!("Playout result: {}", state);
                return state.board.get_scores();
            }
            let p_act =
                self.choose_random_player_action(&state, PlayerId::Player, p_state.get_hands());
            let o_act =
                self.choose_random_player_action(&state, PlayerId::Opponent, o_state.get_hands());
            state::update_state(&mut state, &p_act, &o_act);
            state::update_player_state(&mut p_state, &p_act);
            state::update_player_state(&mut o_state, &o_act);
        }
    }

    fn choose_random_player_action(
        &mut self,
        state: &State,
        player_id: PlayerId,
        hands: &[&'c Card],
    ) -> Action<'c> {
        let mut acts = vec![];
        append_valid_actions(state, hands, player_id, &mut acts);
        *acts.choose(&mut self.rng).unwrap()
    }

    fn expand<'a>(
        &mut self,
        node: &'a mut Node<'c>,
        determinization: &Determinization<'c>,
    ) -> &'a mut Node<'c> {
        // If we've never expand this node, save the legal actions to the node.
        let legal_actions = node.legal_actions.get_or_init(|| {
            if !node.simultaneous_state.player_action_filled() {
                self.precalculate_valid_actions(
                    &node.state,
                    &determinization.player_cards,
                    PlayerId::Player,
                )
            } else {
                self.precalculate_valid_actions(
                    &node.state,
                    &determinization.opponent_cards,
                    PlayerId::Opponent,
                )
            }
        });

        assert_lt!(node.child_nodes.len(), legal_actions.len());
        // There are other legal actions which have never selected.
        // Select one of them first.
        let action = &legal_actions[node.child_nodes.len()];
        let new_node = self.create_child_node(node, action);
        node.child_nodes.push(new_node);
        node.child_nodes.last_mut().unwrap()
    }

    fn create_child_node(&self, node: &Node<'c>, action: &NodeAction<'c>) -> Node<'c> {
        let new_simultaneous_state = node.simultaneous_state.with_action(action.clone());
        if new_simultaneous_state.both_actions_filled() {
            let mut new_state = node.state.clone();
            state::update_state(
                &mut new_state,
                &new_simultaneous_state.player_action.unwrap(),
                &new_simultaneous_state.opponent_action.unwrap(),
            );
            return Node::new(new_state, SimultaneousState::new(), action.clone());
        }
        Node::new(node.state.clone(), new_simultaneous_state, action.clone())
    }

    fn get_filtered_nodes<'a>(
        &mut self,
        node: &'a mut Node<'c>,
        determinization: &Determinization<'c>,
    ) -> Vec<&'a mut Node<'c>> {
        node.child_nodes
            .iter_mut()
            .filter(|n| match n.action {
                NodeAction::PlayerAction(PlayerId::Player, action) => determinization
                    .player_cards
                    .get_hands()
                    .contains(&action.get_consumed_card()),
                NodeAction::PlayerAction(PlayerId::Opponent, action) => determinization
                    .opponent_cards
                    .get_hands()
                    .contains(&action.get_consumed_card()),
                NodeAction::ChanceAction(_) => todo!(),
                NodeAction::Root => {
                    panic!("There shouldn't be a child node with a Root action")
                }
            })
            // TODO: It might be faster to return an iterator intead of a vector.
            .collect()
    }

    fn select_child_node<'a>(
        &mut self,
        node: &'a mut Node<'c>,
        determinization: &Determinization<'c>,
    ) -> &'a mut Node<'c> {
        assert!(!node.is_terminal());
        assert!(!node.is_leaf());

        let mut max_ucb1: f64 = f64::MIN;
        let mut max_index = 0;

        let mut filtered_nodes: Vec<&'a mut Node<'c>> =
            self.get_filtered_nodes(node, determinization);
        let n_sum: i32 = filtered_nodes.iter().map(|n| n.statistic.total_cnt).sum();

        let log_n_sum = (n_sum as f64).ln();
        for i in 0..filtered_nodes.len() {
            let child = &filtered_nodes[i];
            assert_gt!(child.statistic.total_cnt, 0);

            let ucb1 = Self::calc_ucb1(log_n_sum, child);
            if ucb1 > max_ucb1 {
                max_ucb1 = ucb1;
                max_index = i;
            }
        }
        assert_gt!(max_ucb1, 0.0);

        filtered_nodes.swap_remove(max_index)
    }

    fn calc_ucb1(log_n_sum: f64, child: &Node) -> f64 {
        const C: f64 = std::f64::consts::SQRT_2;
        let value: f64 = child.statistic.get_expected_value();
        let visits = child.statistic.total_cnt;
        let explore: f64 = (log_n_sum / visits as f64).sqrt();
        value + C * explore
    }

    fn precalculate_valid_actions(
        &self,
        state: &State,
        cards_state: &PlayerCardState<'c>,
        player_id: PlayerId,
    ) -> Vec<NodeAction<'c>> {
        let mut valid_actions = vec![];
        utils::append_valid_actions(
            state,
            cards_state.get_hands(),
            player_id,
            &mut valid_actions,
        );

        if player_id == PlayerId::Opponent {
            // For the opponent player, list up actions also for cards in deck so that
            // future playouts (which may have different determinized hands) can refer the precalculated actions.
            utils::append_valid_actions(
                state,
                cards_state.get_deck(),
                player_id,
                &mut valid_actions,
            );
        }

        valid_actions
            .iter()
            .map(|a| NodeAction::PlayerAction(player_id, *a))
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

    fn determinize_opponent_deck(&mut self, state: &State) -> PlayerCardState<'c> {
        let mut all_cards = self.context.all_cards.values().collect_vec();
        Self::filter_cards(&mut all_cards, &state.opponent_consumed_cards);

        all_cards.shuffle(&mut self.rng);
        let (hands, deck) = all_cards.split_at(game::HAND_SIZE);

        PlayerCardState::new(hands.to_vec(), deck.to_vec())
    }

    fn determinize_player_deck(
        &mut self,
        state: &State,
        hands: &[&'c Card],
    ) -> PlayerCardState<'c> {
        let mut deck_cards = self.player_initial_deck.clone();

        let hand_ids: Vec<u32> = hands.iter().map(|c| c.get_id()).collect();
        Self::filter_cards(&mut deck_cards, &hand_ids);

        Self::filter_cards(&mut deck_cards, &state.player_consumed_cards);

        // Shuffle cards in deck since the player don't know the order of deck.
        deck_cards.shuffle(&mut self.rng);
        PlayerCardState::new(hands.to_vec(), deck_cards)
    }

    /// Creates a node from an actual game state (visible from the player).
    fn create_root_node<'a>(&mut self, state: &State) -> Node<'c> {
        Node::new(state.clone(), SimultaneousState::new(), NodeAction::Root)
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
            &["="],
            &["="],
            &["="],
            &["="],
            &["="],
            &["="],
            &["="],
            &["="],
            &["="],
            &["="],
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
        let player_initial_deck = context.all_cards.values().collect_vec();
        let opponent_initial_deck = context.all_cards.values().collect_vec();

        let (player_hands, player_deck) = player_initial_deck.split_at(game::HAND_SIZE);
        let (opponent_hands, opponent_deck) = opponent_initial_deck.split_at(game::HAND_SIZE);

        let player_initial_deck = context.all_cards.values().collect_vec();
        let mut traverser = Traverser::new(&context, player_initial_deck, SEED);

        let state = State::new(context.board.clone(), 0, 0, 0, vec![], vec![]);
        let mut root_node = traverser.create_root_node(&state);

        // The root node is still a leaf node.
        assert!(root_node.is_leaf());
        assert!(!root_node.is_terminal());

        // Legal actions are not computed yet.
        assert!(root_node.legal_actions.get().is_none());

        let determinization = Determinization::new(
            PlayerCardState::new(player_hands.to_vec(), player_deck.to_vec()),
            PlayerCardState::new(opponent_hands.to_vec(), opponent_deck.to_vec()),
        );

        // Note that there should be "5" legal actions on the root node.
        // Once a child node is selected. All legal actions should be computed.
        // 1: Put(3)
        // 4: Pass * HAND_SIZE

        // Select leaf and expand for 5 times.
        for i in 1..=5 {
            traverser.iterate(&mut root_node, &determinization);
            // Checks that all expansion happen on the root node.
            assert_eq!(i, root_node.child_nodes.len());
        }

        // All child node should be expanded at this point.
        traverser.select_leaf(&mut root_node, &determinization);
        assert_eq!(5, root_node.child_nodes.len());
    }
}
