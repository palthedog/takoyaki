use log::*;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_mt::Mt64;

use crate::engine::{
    board::Board,
    card::Card,
    game::{self, Action, Context, PlayerId},
    state::{PlayerState, State},
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
        utils::list_valid_actions(
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

#[derive(Clone)]
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

struct Deck<'c> {
    cards: Vec<&'c Card>,
}

enum ChanceAction<'c> {
    DealInitialHand(PlayerId, [&'c Card; game::HAND_SIZE]),
    DealCard(PlayerId, &'c Card),
}

enum NodeAction<'c> {
    PlayerAction(Action<'c>),
    ChanceAction,
}

/// Game state which is visible from a player.
/// It includes presumed information (e.g. opponent's hand/deck)
struct NodeState<'c> {
    state: State,
    player_state: PlayerState<'c>,
    opponent_deck: Vec<&'c Card>,
}

struct Node<'c> {
    statistic: Statistic,
    visit_count: u32,

    legal_actions: Vec<NodeAction<'c>>,
}

impl<'c> Node<'c> {
    fn new(legal_actions: Vec<NodeAction<'c>>) -> Self {
        Self {
            statistic: Statistic::default(),
            visit_count: 0,
            legal_actions,
        }
    }
}

struct Traverser<'c> {
    context: &'c Context,
    rng: Mt64,
}

impl<'c> Traverser<'c> {
    fn new(context: &'c Context, seed: u64) -> Self {
        Self {
            context,
            rng: Mt64::new(seed),
        }
    }

    /*
        fn traverse(
            &mut self,
            state: &State,
            player_state: &PlayerState<'c>,
            opponent_deck: &[&'c Card],
        ) {
            let root_node = self.create_node(state, player_state, opponent_deck);
            let mut node = &root_node;
            for n in 0..10 {
                //let action = self.select_action(n, node);
            }
        }
    */

    fn playout(node: &Node, action: Action) {}

    /// Choose an action based on UCB1.
    //fn select_action(&self, total_simulation_cnt: u32, node: &Node) -> NodeAction {
    //let actions = node.legal_actions;
    //}

    fn gen_valid_actions(
        &self,
        state: &State,
        player_state: &PlayerState<'c>,
    ) -> Vec<NodeAction<'c>> {
        let mut valid_actions: Vec<Action> = vec![];

        utils::list_valid_actions(
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

    fn create_node<'a>(
        &self,
        state: &State,
        player_state: &'a PlayerState<'c>,
        opponent_deck: &[&'c Card],
    ) -> Node<'c> {
        let simulation_state: NodeState = NodeState {
            state: state.clone(),
            player_state: player_state.clone(),
            opponent_deck: opponent_deck.to_vec(),
        };
        Node::new(self.gen_valid_actions(&state, &player_state))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::engine::state::{self, tests::new_test_card, Phase};

    use super::*;

    fn new_test_all_cards(card_strs: &[&[&str]]) -> HashMap<u32, Card> {
        let mut tmp: HashMap<u32, Card> = HashMap::new();
        card_strs.iter().enumerate().for_each(|(i, s)| {
            tmp.insert(i as u32, state::tests::new_test_card_impl(s, i as u32, 0));
        });
        tmp
    }

    #[test]
    fn test_initial_node() {
        #[rustfmt::skip]
        let all_cards = new_test_all_cards(&[
            &[
                "="
            ],
            &[
                "=="
            ],
            &[
                "==="
            ],
            &[
                "====",
            ],
            &[
                "=",
                "=",
                "===",
            ],
        ]);
        #[rustfmt::skip]
        let state = state::tests::new_test_state(
            Phase::Initial,
            &[
            "#####",
            "#.O.#",
            "#.P.#",
            "#####"
        ], 0, 0, 0);
        let context = Context {
            board: state.board.clone(),
            all_cards,
            enabled_step_execution: false,
        };
        const SEED: u64 = 42;
        let traverser = Traverser::new(&context, SEED);
        let player_hands = vec![
            context.card_ref(0),
            context.card_ref(1),
            context.card_ref(2),
            context.card_ref(3),
        ];
        let player_deck = vec![context.card_ref(4)];
        let player_state = PlayerState::new(&player_hands, &player_deck);
        let opponent_deck = vec![
            context.card_ref(0),
            context.card_ref(1),
            context.card_ref(2),
            context.card_ref(3),
            context.card_ref(4),
        ];
        traverser.create_node(&state, &player_state, &opponent_deck);
    }
}
