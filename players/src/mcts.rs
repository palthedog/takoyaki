use itertools::Itertools;
use log::*;
use more_asserts::*;
use rand::{
    seq::SliceRandom,
    Rng,
};
use rand_mt::Mt64;
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::Display,
    time::{
        Duration,
        Instant,
    },
};

use engine::{
    Action,
    Board,
    Card,
    Context,
    PlayerCardState,
    PlayerId,
    State,
};

use super::{
    utils::append_valid_actions,
    Player,
};

// It looks good enough acording to random battles.
pub const UCT_CONST_DEFAULT: f64 = 0.9;

pub struct MctsPlayer {
    iterations: usize,
    uct_const: f64,

    name: String,
    player_id: PlayerId,
    traverser: Option<Traverser>,
    board: Option<Board>,
    rng: Mt64,
}

impl MctsPlayer {
    pub fn new(name: String, seed: u64, iterations: usize, uct_constant: f64) -> Self {
        let rng = Mt64::new(seed);
        MctsPlayer {
            name,
            iterations,
            uct_const: uct_constant,
            player_id: PlayerId::South,
            traverser: None,
            board: None,
            rng,
        }
    }
}

impl Player for MctsPlayer {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn init_game(
        &mut self,
        player_id: PlayerId,
        context: &Context,
        board: &Board,
        deck: Vec<Card>,
    ) {
        self.player_id = player_id;
        self.traverser = Some(Traverser::new(
            context,
            player_id,
            deck,
            self.uct_const,
            self.rng.next_u64(),
        ));
        self.board = Some(board.clone());
    }

    fn need_redeal_hands(&mut self, dealed_cards: &[Card], time_limit: &Duration) -> bool {
        //self.rng.gen_bool(0.5)
        self.traverser.as_mut().unwrap().search_need_redeal_hands(
            self.board.as_ref().unwrap(),
            dealed_cards,
            self.iterations,
            time_limit,
        )
    }

    fn get_action(&mut self, state: &State, hands: &[Card], time_limit: &Duration) -> Action {
        self.traverser
            .as_mut()
            .unwrap()
            .search_action(state, hands, self.iterations, time_limit)
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
struct Statistic {
    total_cnt: i32,
    win_cnt: i32,
    lose_cnt: i32,
    draw_cnt: i32,
    score_diff: i32,
}

impl Statistic {
    fn update_with(&mut self, (p, o): (u32, u32)) {
        self.total_cnt += 1;
        // TODO: Might be better to update the logic here.
        //       Checks how often visit other child nodes.
        self.score_diff += p as i32 - o as i32;
        match p.cmp(&o) {
            Ordering::Equal => self.draw_cnt += 1,
            Ordering::Less => self.lose_cnt += 1,
            Ordering::Greater => self.win_cnt += 1,
        }
    }

    fn get_expected_value(&self) -> f64 {
        self.score_diff as f64 / self.total_cnt as f64
        //(self.win_cnt - self.lose_cnt) as f64 / self.total_cnt as f64
    }

    fn get_visit_count(&self) -> i32 {
        self.total_cnt
    }
}

impl Display for Statistic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stats(Visited: {}, win: {}, lose: {}, draw: {}",
            self.total_cnt, self.win_cnt, self.lose_cnt, self.draw_cnt
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum NodeAction {
    TurnRoot,
    /// TurnRoot
    ///     -> PlayerAction(player) -> PlayerAction(opponent)
    ///     -> DealCard(player)
    PlayerAction(PlayerId, Action),
    DealCard(Card), // chance node

    /// The very beginning of the game tree.
    /// GameRoot
    ///     -> AcceptInitialHands(player) -> DealAcceptedHands(player)
    ///     -> TurnRoot...
    GameRoot,
    AcceptInitialHands(bool),
    DealAcceptedHands(Vec<Card>), // chance node
}

impl Display for NodeAction {
    fn fmt(&self, f: &mut __core::fmt::Formatter<'_>) -> __core::fmt::Result {
        match self {
            NodeAction::TurnRoot => write!(f, "RootNode"),
            NodeAction::PlayerAction(pid, act) => write!(f, "PlayerAction({}, {})", pid, act),
            NodeAction::DealCard(card) => write!(f, "DealCard: {}", card),
            NodeAction::GameRoot => write!(f, "GameRoot"),
            NodeAction::AcceptInitialHands(accepted) => {
                write!(f, "AcceptInitialHands({})", accepted)
            }
            NodeAction::DealAcceptedHands(cards) => {
                write!(f, "DealAcceptedHands({})", engine::format_cards(cards))
            }
        }
    }
}

/// Game state which is visible from a player.
/// It includes presumed information (e.g. opponent's hand/deck)
#[derive(Debug, PartialEq, Clone)]
struct Determinization {
    player_cards: [PlayerCardState; 2],
}

impl Determinization {
    fn new(player_cards_a: PlayerCardState, player_cards_b: PlayerCardState) -> Self {
        if player_cards_a.get_player_id() == PlayerId::North {
            return Self::new(player_cards_b, player_cards_a);
        }

        assert_eq!(PlayerId::South, player_cards_a.get_player_id());
        assert_eq!(PlayerId::North, player_cards_b.get_player_id());
        Determinization {
            player_cards: [player_cards_a, player_cards_b],
        }
    }

    fn get_cards_as_mut(&mut self, player_id: PlayerId) -> &mut PlayerCardState {
        &mut self.player_cards[player_id.to_index()]
    }

    fn set_hands(&mut self, player_id: PlayerId, hands: &[Card], rng: &mut Mt64) {
        let cards = self.get_cards_as_mut(player_id);

        let mut new_deck: Vec<Card> = Vec::new();
        for h in cards.get_hands().iter().filter(|c| !hands.contains(c)) {
            new_deck.push(h.clone());
        }
        for d in cards.get_deck().iter().filter(|c| !hands.contains(c)) {
            new_deck.push(d.clone());
        }
        new_deck.shuffle(rng);

        let new_cards = PlayerCardState::new(player_id, hands.to_vec(), new_deck);
        self.player_cards[player_id.to_index()] = new_cards
    }

    fn get_cards(&self, player_id: PlayerId) -> &PlayerCardState {
        &self.player_cards[player_id.to_index()]
    }

    fn is_consistent(&self, state: &State) -> bool {
        self.is_consistent_for_player(state, PlayerId::South)
            && self.is_consistent_for_player(state, PlayerId::North)
    }

    fn is_consistent_for_player(&self, state: &State, player_id: PlayerId) -> bool {
        let cards = self.get_cards(player_id);
        let consumed = state.get_consumed_cards(player_id);

        for consumed_id in consumed {
            for h in cards.get_hands() {
                if h.get_id() == *consumed_id {
                    return false;
                }
            }
            for d in cards.get_deck() {
                if d.get_id() == *consumed_id {
                    return false;
                }
            }
        }
        true
    }
}

impl Display for Determinization {
    fn fmt(&self, f: &mut __core::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Determinization {{")?;
        writeln!(
            f,
            "    South hands: {:?}",
            self.get_cards(PlayerId::South)
                .get_hands()
                .iter()
                .map(|c| c.get_id())
                .collect::<Vec<_>>()
        )?;
        writeln!(
            f,
            "    South deck: {:?}",
            self.get_cards(PlayerId::South)
                .get_deck()
                .iter()
                .map(|c| c.get_id())
                .collect::<Vec<_>>()
        )?;
        writeln!(
            f,
            "    North hands: {:?}",
            self.get_cards(PlayerId::North)
                .get_hands()
                .iter()
                .map(|c| c.get_id())
                .collect::<Vec<_>>()
        )?;
        writeln!(
            f,
            "    North deck: {:?}",
            self.get_cards(PlayerId::North)
                .get_deck()
                .iter()
                .map(|c| c.get_id())
                .collect::<Vec<_>>()
        )?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
struct SimultaneousState {
    south_action: Option<Action>,
    north_action: Option<Action>,

    state: State,
}

impl SimultaneousState {
    fn new(state: State) -> Self {
        Self {
            south_action: None,
            north_action: None,
            state,
        }
    }

    fn get_state(&self) -> &State {
        &self.state
    }

    fn get_turn(&self) -> i32 {
        self.state.turn
    }

    fn is_end(&self) -> bool {
        self.south_action.is_none() && self.north_action.is_none() && self.state.is_end()
    }

    fn action_is_filled(&self, player_id: PlayerId) -> bool {
        match player_id {
            PlayerId::South => self.south_action.is_some(),
            PlayerId::North => self.north_action.is_some(),
        }
    }

    fn with_action(self, node_action: NodeAction) -> Self {
        let mut south_action = self.south_action;
        let mut north_action = self.north_action;
        let mut state = self.state;
        match node_action {
            NodeAction::PlayerAction(PlayerId::South, act) => {
                assert!(south_action.is_none());
                south_action = Some(act);
            }
            NodeAction::PlayerAction(PlayerId::North, act) => {
                assert!(north_action.is_none());
                north_action = Some(act);
            }
            NodeAction::DealCard(_card) => unimplemented!(),
            NodeAction::TurnRoot => unimplemented!(),
            NodeAction::GameRoot => unimplemented!(),
            NodeAction::AcceptInitialHands(_) => unimplemented!(),
            NodeAction::DealAcceptedHands(_) => unimplemented!(),
        }
        match (south_action, north_action) {
            (Some(sa), Some(na)) => {
                // Both action is filled. Update the State itself.
                engine::update_state(&mut state, &sa, &na);
                Self {
                    south_action: None,
                    north_action: None,
                    state,
                }
            }
            (sa, na) => Self {
                south_action: sa,
                north_action: na,
                state,
            },
        }
    }

    fn both_actions_filled(&self) -> bool {
        self.south_action.is_some() && self.north_action.is_some()
    }
}

impl Display for SimultaneousState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SimultaneousState {{")?;
        writeln!(f, "    South: {:?}", self.south_action)?;
        writeln!(f, "    North: {:?}", self.north_action)?;
        writeln!(f, "    State: {}", self.state)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
struct Node {
    traverser_player_id: PlayerId,
    simultaneous_state: SimultaneousState,
    action: NodeAction,

    statistic: Statistic,

    child_nodes: HashMap<NodeAction, Node>,

    /// Lists of legal actions based on the consumed hands.
    /// The entry may not exist if the traverser has never accessed the value.
    legal_actions: HashMap<Card, Vec<NodeAction>>,
}

impl Node {
    fn new(
        traverser_player_id: PlayerId,
        simultaneous_state: SimultaneousState,
        action: NodeAction,
    ) -> Self {
        assert!(
            !simultaneous_state.both_actions_filled(),
            "If all actions are filled, we should create the new node with a updated State."
        );

        Self {
            traverser_player_id,
            simultaneous_state,
            action,
            statistic: Statistic::default(),
            child_nodes: HashMap::new(),
            legal_actions: HashMap::new(),
        }
    }

    fn is_terminal(&self) -> bool {
        self.simultaneous_state.is_end()
    }

    fn is_pure_state(&self) -> bool {
        match self.action {
            NodeAction::TurnRoot => true,
            NodeAction::PlayerAction(pid, _) => {
                pid != self.traverser_player_id
                    && self.simultaneous_state.get_turn() == engine::TURN_COUNT
            }
            NodeAction::DealCard(_) => true,

            NodeAction::GameRoot => false,
            NodeAction::AcceptInitialHands(_) => false,
            NodeAction::DealAcceptedHands(_) => true,
        }
    }

    fn get_prev_player_id(&self) -> PlayerId {
        match self.action {
            NodeAction::TurnRoot => unimplemented!(),
            NodeAction::GameRoot => unimplemented!(),

            NodeAction::PlayerAction(pid, _) => pid,
            NodeAction::DealCard(_) => self.traverser_player_id,
            NodeAction::AcceptInitialHands(_) => self.traverser_player_id,
            NodeAction::DealAcceptedHands(_) => self.traverser_player_id,
        }
    }

    fn get_legal_actions(&mut self, determinization: &Determinization) -> Vec<NodeAction> {
        assert_lt!(
            self.simultaneous_state.state.get_turn(),
            engine::TURN_COUNT,
            "There shouldn't be any child ndoes since this node is a terminal node."
        );
        match self.action {
            NodeAction::TurnRoot => {
                self.get_legal_player_actions(self.traverser_player_id, determinization)
            }
            NodeAction::PlayerAction(pid, _) => {
                if pid == self.traverser_player_id {
                    // Opponent's action.
                    self.get_legal_player_actions(pid.another(), determinization)
                } else {
                    // Next node would be deal action
                    self.get_deal_action(pid.another(), determinization)
                }
            }
            NodeAction::DealCard(_) => {
                self.get_legal_player_actions(self.traverser_player_id, determinization)
            }
            NodeAction::GameRoot => self.get_legal_accept_initial_hands_action(),
            NodeAction::AcceptInitialHands(accept) => {
                self.get_legal_deal_accepted_hands_action(accept, determinization)
            }
            NodeAction::DealAcceptedHands(_) => {
                // Here is the end of dealing phase.
                // Let players to play the game.
                self.get_legal_player_actions(self.traverser_player_id, determinization)
            }
        }
    }

    fn get_legal_player_actions(
        &mut self,
        player_id: PlayerId,
        determinization: &Determinization,
    ) -> Vec<NodeAction> {
        let hands = determinization.get_cards(player_id).get_hands();

        let mut v = vec![];
        for c in hands {
            let actions = self.get_legal_actions_for_card(player_id, c);
            for act in actions {
                v.push(act.clone());
            }
        }
        v
    }

    fn get_legal_actions_for_card(&mut self, next_pid: PlayerId, card: &Card) -> &Vec<NodeAction> {
        assert!(!self.simultaneous_state.action_is_filled(next_pid));
        let entry = self.legal_actions.entry(card.clone()).or_insert_with(|| {
            let mut actions: Vec<Action> = vec![];
            append_valid_actions(
                self.simultaneous_state.get_state(),
                &[card.clone()],
                next_pid,
                &mut actions,
            );
            actions
                .into_iter()
                .map(|act| NodeAction::PlayerAction(next_pid, act))
                .collect()
        });
        entry
    }

    fn get_deal_action(
        &mut self,
        player_id: PlayerId,
        determinization: &Determinization,
    ) -> Vec<NodeAction> {
        let cards = determinization.get_cards(player_id);
        vec![NodeAction::DealCard(cards.get_next_deal_card().clone())]
    }

    fn get_legal_accept_initial_hands_action(&mut self) -> Vec<NodeAction> {
        vec![
            NodeAction::AcceptInitialHands(false),
            NodeAction::AcceptInitialHands(true),
        ]
    }

    fn get_legal_deal_accepted_hands_action(
        &mut self,
        accept: bool,
        determinization: &Determinization,
    ) -> Vec<NodeAction> {
        if accept {
            // The player accepted the card.
            // There is no multiple chance node which deal new hands.
            let dealed_hands = determinization
                .get_cards(self.traverser_player_id)
                .get_hands();
            return vec![NodeAction::DealAcceptedHands(dealed_hands.to_vec())];
        }

        // Lists all possible hands.
        let all_cards = determinization
            .get_cards(self.traverser_player_id)
            .get_all_cards();
        debug!("# of all cards: {}", all_cards.len());
        let hands: Vec<NodeAction> = all_cards
            .into_iter()
            .combinations(engine::HAND_SIZE)
            .map(NodeAction::DealAcceptedHands)
            .collect();
        debug!("# of possible hands: {}", hands.len());
        hands
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut __core::fmt::Formatter<'_>) -> __core::fmt::Result {
        writeln!(f, "Node: {}", self.action)?;
        writeln!(f, "    children: [")?;
        for k in self.child_nodes.keys() {
            writeln!(f, "       {}", k)?;
        }
        writeln!(f, "    ]")?;
        Ok(())
    }
}

struct Traverser {
    context: Context,
    traverser_player_id: PlayerId,
    my_initial_deck: Vec<Card>,

    uct_const: f64,

    rng: Mt64,
}

impl Traverser {
    fn new(
        context: &Context,
        traverser_player_id: PlayerId,
        player_initial_deck: Vec<Card>,
        uct_const: f64,
        seed: u64,
    ) -> Self {
        Self {
            context: context.clone(), // TODO: Stop cloning it.
            traverser_player_id,
            my_initial_deck: player_initial_deck,
            uct_const,
            rng: Mt64::new(seed),
        }
    }

    /// Descend the tree until leaf/terminal node is found.
    fn select_leaf<'a>(
        &mut self,
        root_node: &'a mut Node,
        determinization: &mut Determinization,
    ) -> (&'a mut Node, Vec<NodeAction>) {
        let mut history = vec![];
        let mut node: &'a mut Node = root_node;
        while !self.is_leaf_node(node, determinization) {
            node = self.select_child_node(node, determinization);
            history.push(node.action.clone());

            self.update_determinization_by_node_action(&node.action, determinization);
        }
        (node, history)
    }

    fn update_determinization_by_node_action(
        &mut self,
        node_action: &NodeAction,
        determinization: &mut Determinization,
    ) {
        // Update determinization by actions.
        match node_action {
            NodeAction::TurnRoot => unimplemented!(),
            NodeAction::PlayerAction(pid, act) => {
                let player_cards = determinization.get_cards_as_mut(*pid);
                let consumed_card = act.get_consumed_card();
                player_cards.consume_card(consumed_card);
            }
            NodeAction::DealCard(card) => {
                let player_cards = determinization.get_cards_as_mut(self.traverser_player_id);
                assert_eq!(card, player_cards.get_next_deal_card());
                player_cards.draw_card();

                let opponent_cards =
                    determinization.get_cards_as_mut(self.traverser_player_id.another());
                opponent_cards.draw_card();
            }

            NodeAction::GameRoot => unimplemented!(),
            NodeAction::AcceptInitialHands(_accept) => {
                // No need to update determinization.
            }
            NodeAction::DealAcceptedHands(hands) => {
                determinization.set_hands(self.traverser_player_id, hands, &mut self.rng);
            }
        };
    }

    fn search_action(
        &mut self,
        state: &State,
        hands: &[Card],
        iterations: usize,
        time_limit: &Duration,
    ) -> Action {
        let mut root_node = self.create_turn_root_node(self.traverser_player_id, state.clone());
        let timer = Instant::now();
        for n in 0..iterations {
            let mut determinization = Determinization::new(
                self.determinize_my_deck(root_node.simultaneous_state.get_state(), hands),
                self.determinize_another_deck(root_node.simultaneous_state.get_state()),
            );
            self.iterate(&mut root_node, &mut determinization);

            if timer.elapsed() > *time_limit {
                info!("Time limit exceeded: Ran {} iterations", n + 1);
                break;
            }
        }

        // Choose the best hand.
        if log::log_enabled!(Level::Debug) {
            debug!("Legal actions");
            root_node
                .child_nodes
                .values()
                .for_each(|c| debug!("    {}: {}", c.action, c.statistic));
        }

        let most_visited = root_node
            .child_nodes
            .values()
            .max_by(|a, b| {
                a.statistic
                    .get_visit_count()
                    .cmp(&b.statistic.get_visit_count())
            })
            .unwrap();
        if let NodeAction::PlayerAction(player_id, action) = &most_visited.action {
            assert_eq!(self.traverser_player_id, *player_id);
            action.clone()
        } else {
            panic!(
                "The root node has an invalid action for the player: {:#?}",
                root_node.child_nodes
            );
        }
    }

    fn search_need_redeal_hands(
        &mut self,
        board: &Board,
        hands: &[Card],
        iterations: usize,
        time_limit: &Duration,
    ) -> bool {
        info!("Should we redeal hands? {}", engine::format_cards(hands));
        let state = State::new(board.clone(), 0, 0, 0, vec![], vec![]);
        let mut root_node = self.create_game_root_node(self.traverser_player_id, state);
        let timer = Instant::now();
        for n in 0..iterations {
            let mut determinization = Determinization::new(
                self.determinize_my_deck(root_node.simultaneous_state.get_state(), hands),
                self.determinize_another_deck(root_node.simultaneous_state.get_state()),
            );
            self.iterate(&mut root_node, &mut determinization);

            if timer.elapsed() > *time_limit {
                info!("Time limit exceeded: Ran {} iterations", n + 1);
                break;
            }
        }

        // Choose the best hand.
        info!("Legal actions");
        root_node
            .child_nodes
            .values()
            .for_each(|c| info!("    {}: {}", c.action, c.statistic));

        let most_visited = root_node
            .child_nodes
            .values()
            .max_by(|a, b| {
                a.statistic
                    .get_visit_count()
                    .cmp(&b.statistic.get_visit_count())
            })
            .unwrap();
        if let NodeAction::AcceptInitialHands(accept) = most_visited.action {
            info!("Should we redeal hands? {}", accept);
            accept
        } else {
            panic!(
                "The game root node should have only AcceptInitialHands action as their children: {:?}",
                root_node.child_nodes
            );
        }
    }

    fn iterate(&mut self, root_node: &mut Node, determinization: &mut Determinization) {
        // Selection
        debug!("Selection");
        let (leaf, mut history) = self.select_leaf(root_node, determinization);

        // Expansion
        debug!("Expansion");
        let leaf = if !leaf.is_terminal() {
            let new_leaf = self.expand(leaf, determinization);
            history.push(new_leaf.action.clone());
            new_leaf
        } else {
            leaf
        };

        // Simulation
        let result = self.playout(leaf, determinization);

        // Backpropagation
        debug!("Backpropagation");
        let mut node = root_node;
        node.statistic.update_with(result);
        for visited_node in history {
            node = node
                .child_nodes
                .values_mut()
                .find(|c| c.action == visited_node)
                .unwrap();
            node.statistic.update_with(result);
        }
    }

    fn playout(&mut self, node: &mut Node, determinization: &mut Determinization) -> (u32, u32) {
        debug_assert!(
            determinization.is_consistent(node.simultaneous_state.get_state()),
            "Inconsistent state with the determination:\nConsumed cards:\nSouth: {:?}\nNorth: {:?}\nDeterminization: {}",
            &node.simultaneous_state.get_state().get_consumed_cards(PlayerId::South),
            &node.simultaneous_state.get_state().get_consumed_cards(PlayerId::North),
            determinization
        );

        let mut node: Node = node.clone();

        // Simulate the game till intermidiate state is solved.
        while !node.is_pure_state() {
            let acts = node.get_legal_actions(determinization);
            let rand_action = acts.choose(&mut self.rng).unwrap();
            node = self.create_child_node(&node, rand_action);
            self.update_determinization_by_node_action(rand_action, determinization);
        }

        let mut p_state = determinization.get_cards(PlayerId::South).clone();
        let mut o_state = determinization.get_cards(PlayerId::North).clone();
        let simul_state = node.simultaneous_state.clone();
        // Both actions shouldn't be filled at this point.
        assert!(!simul_state.action_is_filled(PlayerId::South));
        assert!(!simul_state.action_is_filled(PlayerId::North));

        let mut state = simul_state.state;

        while !state.is_end() {
            let p_act =
                self.choose_random_player_action(&state, PlayerId::South, p_state.get_hands());
            let o_act =
                self.choose_random_player_action(&state, PlayerId::North, o_state.get_hands());

            engine::update_state(&mut state, &p_act, &o_act);
            engine::update_player_state(&state, &mut p_state, &p_act);
            engine::update_player_state(&state, &mut o_state, &o_act);
        }
        trace!("Playout result: {}", state);
        state.board.get_scores()
    }

    fn choose_random_player_action(
        &mut self,
        state: &State,
        player_id: PlayerId,
        hands: &[Card],
    ) -> Action {
        let mut acts: Vec<Action> = vec![];
        append_valid_actions(state, hands, player_id, &mut acts);
        let i = self.rng.gen_range(0..acts.len());
        acts.swap_remove(i)
    }

    fn expand<'a>(
        &mut self,
        node: &'a mut Node,
        determinization: &mut Determinization,
    ) -> &'a mut Node {
        let legal_actions = node.get_legal_actions(determinization);

        debug!("# of legal actions: {}", legal_actions.len());
        // There can be other legal actions which have never selected.
        // Select one of them first.
        let mut expanding_action: Option<NodeAction> = None;
        for act in &legal_actions {
            if !node.child_nodes.contains_key(act) {
                let _ = expanding_action.insert(act.clone());
                break;
            }
        }
        // The leaf node is going to be expanded with this action.
        let action_for_expanding = expanding_action.unwrap_or_else(|| {
            let mut s = "Couldn't find unvisited nodes from legal actions: [".to_string();
            for act in legal_actions {
                s += &format!("{}, ", act);
            }
            s += "]\n";
            s += &format!("node: {}", node);
            panic!("{}", s);
        });
        self.update_determinization_by_node_action(&action_for_expanding, determinization);
        let new_node = self.create_child_node(node, &action_for_expanding);
        node.child_nodes
            .insert(action_for_expanding.clone(), new_node);
        node.child_nodes.get_mut(&action_for_expanding).unwrap()
    }

    fn create_child_node(&self, node: &Node, action: &NodeAction) -> Node {
        match action {
            NodeAction::TurnRoot => unimplemented!(),
            NodeAction::GameRoot => unimplemented!(),
            NodeAction::PlayerAction(_pid, _act) => {
                let new_simultaneous_state =
                    node.simultaneous_state.clone().with_action(action.clone());
                Node::new(
                    node.traverser_player_id,
                    new_simultaneous_state,
                    action.clone(),
                )
            }
            _ => Node::new(
                node.traverser_player_id,
                node.simultaneous_state.clone(),
                action.clone(),
            ),
        }
    }

    fn get_filtered_nodes<'a>(
        &mut self,
        node: &'a mut Node,
        determinization: &Determinization,
    ) -> Vec<&'a mut Node> {
        let legal_actions = node.get_legal_actions(determinization);
        node.child_nodes
            .iter_mut()
            .filter(|(act, _child)| legal_actions.contains(act))
            .map(|(_k, v)| v)
            .collect()
    }

    fn is_leaf_node(&mut self, node: &mut Node, determinization: &Determinization) -> bool {
        if node.is_terminal() {
            return true;
        }

        let legal_actions = node.get_legal_actions(determinization);
        for act in legal_actions {
            if !node.child_nodes.contains_key(&act) {
                // This node doesn't have a child node for `act` yet.
                return true;
            }
        }
        false
    }

    /// Select a child node which is consistent with the determinization.
    fn select_child_node<'a>(
        &mut self,
        node: &'a mut Node,
        determinization: &Determinization,
    ) -> &'a mut Node {
        assert!(!node.is_terminal());

        let mut max_ucb1: f64 = f64::MIN;
        let mut max_index = 0;

        let mut filtered_nodes: Vec<&'a mut Node> = self.get_filtered_nodes(node, determinization);
        assert_gt!(filtered_nodes.len(), 0);
        let n_sum: i32 = filtered_nodes.iter().map(|n| n.statistic.total_cnt).sum();
        let log_n_sum = (n_sum as f64).ln();
        debug!("UCB1");
        for (i, child) in filtered_nodes.iter().enumerate() {
            assert_gt!(child.statistic.total_cnt, 0);
            debug!("   {}, {}:", child.action, child.statistic);
            let ucb1 = Self::calc_ucb1(log_n_sum, self.uct_const, child);
            if ucb1 > max_ucb1 {
                max_ucb1 = ucb1;
                max_index = i;
            }
        }
        filtered_nodes.swap_remove(max_index)
    }

    fn calc_ucb1(log_n_sum: f64, c: f64, child: &Node) -> f64 {
        let mut value: f64 = child.statistic.get_expected_value();

        if child.get_prev_player_id() == PlayerId::North {
            value = -value;
        }

        let visits = child.statistic.total_cnt;
        let explore: f64 = (log_n_sum / visits as f64).sqrt();
        debug!(
            "     {} + {} * {} = {}",
            value,
            c,
            explore,
            value + c * explore
        );
        value + c * explore
    }

    fn filter_cards(cards: &mut Vec<Card>, remove_card_ids: &[u32]) {
        let expected_len = cards.len() - remove_card_ids.len();
        remove_card_ids.iter().for_each(|r| {
            for i in 0..cards.len() {
                if cards[i].get_id() == *r {
                    cards.swap_remove(i);
                    break;
                }
            }
        });
        assert_eq!(expected_len, cards.len());
    }

    fn determinize_another_deck(&mut self, state: &State) -> PlayerCardState {
        let another_player_id = self.traverser_player_id.another();
        let mut all_cards = self.context.all_cards.values().cloned().collect_vec();
        Self::filter_cards(&mut all_cards, state.get_consumed_cards(another_player_id));

        all_cards.shuffle(&mut self.rng);
        let (hands, deck) = all_cards.split_at(engine::HAND_SIZE);

        PlayerCardState::new(another_player_id, hands.to_vec(), deck.to_vec())
    }

    fn determinize_my_deck(&mut self, state: &State, hands: &[Card]) -> PlayerCardState {
        let mut deck_cards = self.my_initial_deck.clone();

        let hand_ids: Vec<u32> = engine::to_ids(hands);
        Self::filter_cards(&mut deck_cards, &hand_ids);

        Self::filter_cards(
            &mut deck_cards,
            state.get_consumed_cards(self.traverser_player_id),
        );

        // Shuffle cards in deck since the player don't know the order of deck.
        deck_cards.shuffle(&mut self.rng);
        PlayerCardState::new(self.traverser_player_id, hands.to_vec(), deck_cards)
    }

    /// Creates a node from an actual game state (visible from the player).
    fn create_turn_root_node(&mut self, traverser_player_id: PlayerId, state: State) -> Node {
        Node::new(
            traverser_player_id,
            SimultaneousState::new(state),
            NodeAction::TurnRoot,
        )
    }

    /// Creates a root node where the dealer is asking for players about either they want the dealer
    /// to re-deal their hand or not.
    fn create_game_root_node(&mut self, traverser_player_id: PlayerId, state: State) -> Node {
        Node::new(
            traverser_player_id,
            SimultaneousState::new(state),
            NodeAction::GameRoot,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::Arc,
    };

    use engine;

    use super::*;

    pub fn new_test_card_impl(lines: &[&str], id: u32, special_cost: i32) -> Card {
        let lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
        let cell_cnt: i32 = lines
            .iter()
            .map(|line| {
                line.as_bytes()
                    .iter()
                    .filter(|&ch| *ch == b'=' || *ch == b'*')
                    .count() as i32
            })
            .sum();
        engine::load_card_from_lines(
            id,
            String::from("test card"),
            cell_cnt,
            special_cost,
            &lines,
        )
    }

    fn new_test_all_cards(card_strs: &[&[&str]]) -> HashMap<u32, Card> {
        let mut tmp: HashMap<u32, Card> = HashMap::new();
        card_strs.iter().enumerate().for_each(|(i, s)| {
            const SPECIAL_COST: i32 = 10;
            tmp.insert(i as u32, new_test_card_impl(s, i as u32, SPECIAL_COST));
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
        let board = engine::load_board_from_lines(
            String::from("test_board"),
            &[
            "#####",
            "#.O##",
            "#..P#",
            "#####"
            ]);
        let context = Arc::new(Context {
            all_cards,
            enabled_step_execution: false,
        });
        const SEED: u64 = 42;
        let sorted_cards = context
            .all_cards
            .values()
            .cloned()
            .sorted_by(|a, b| a.get_id().cmp(&b.get_id()))
            .collect_vec();
        let player_initial_deck = sorted_cards.clone();
        let opponent_initial_deck = sorted_cards;

        let (player_hands, player_deck) = player_initial_deck.split_at(engine::HAND_SIZE);
        let (opponent_hands, opponent_deck) = opponent_initial_deck.split_at(engine::HAND_SIZE);

        let player_initial_deck = context.all_cards.values().cloned().collect_vec();
        let mut traverser = Traverser::new(
            &context,
            PlayerId::South,
            player_initial_deck,
            std::f64::consts::SQRT_2,
            SEED,
        );

        let state = State::new(board, 0, 0, 0, vec![], vec![]);
        let mut root_node = traverser.create_turn_root_node(PlayerId::South, state);

        // The root node is still a leaf node.
        assert!(!root_node.is_terminal());

        let determinization = Determinization::new(
            PlayerCardState::new(PlayerId::South, player_hands.to_vec(), player_deck.to_vec()),
            PlayerCardState::new(
                PlayerId::North,
                opponent_hands.to_vec(),
                opponent_deck.to_vec(),
            ),
        );

        // Note that there should be "5" legal actions on the root node.
        // Once a child node is selected. All legal actions should be computed.
        // 1: Put(3)
        // 4: Pass * HAND_SIZE

        // Select leaf and expand for 5 times.
        for i in 1..=5 {
            let mut determinization = determinization.clone();
            traverser.iterate(&mut root_node, &mut determinization);
            // Checks that all expansion happen on the root node.
            assert_eq!(i, root_node.child_nodes.len());
        }

        // All child node should be expanded at this point.
        // So additional iteration doesn't add a child node to the root node.
        let mut determinization = determinization;
        traverser.iterate(&mut root_node, &mut determinization);
        assert_eq!(5, root_node.child_nodes.len());
    }
}
