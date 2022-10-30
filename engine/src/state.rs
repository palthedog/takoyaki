use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::Display,
    hash::Hash,
};

use log::*;
use more_asserts::*;

use super::{
    board::{
        Board,
        BoardCell,
        BoardPosition,
    },
    card::{
        Card,
        CardCell,
        CardPosition,
    },
    game,
    game::{
        Action,
        PlayerId,
    },
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PlayerCardState {
    player_id: PlayerId,
    hands: Vec<Card>,
    deck: Vec<Card>,
}

impl PlayerCardState {
    pub fn new(player_id: PlayerId, hands: Vec<Card>, deck: Vec<Card>) -> PlayerCardState {
        PlayerCardState {
            player_id,
            hands,
            deck,
        }
    }

    pub fn get_hands(&self) -> &[Card] {
        &self.hands
    }

    pub fn get_deck(&self) -> &[Card] {
        &self.deck
    }

    // We may want a randomized version later for random simulation.
    fn draw_card(&mut self) {
        if self.deck.is_empty() {
            debug!("There is no card in the deck.");
            return;
        }
        let draw = self.deck.remove(0);
        self.hands.push(draw);
    }

    pub fn consume_card(&mut self, card: &Card) {
        for i in 0..self.hands.len() {
            if self.hands[i].get_id() == card.get_id() {
                self.hands.remove(i);
                return;
            }
        }
        panic!(
            "Couldn't find the consumed card from hands.\nconsumed: {}\nhands: {:?}\n",
            card, self.hands
        );
    }
}

impl Display for PlayerCardState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Hands:[")?;
        for card in self.hands.iter() {
            f.write_str(&textwrap::indent(&format!("{}\n", card), "    "))?;
        }
        writeln!(f, "]")?;

        Ok(())
    }
}

/// Observable information about the current state of the game.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    pub board: Board,
    pub turn: i32,
    pub player_special_count: i32,
    pub opponent_special_count: i32,

    player_consumed_cards: Vec<u32>,
    opponent_consumed_cards: Vec<u32>,
}

impl State {
    pub fn new(
        board: Board,
        turn: i32,
        player_special_count: i32,
        opponent_special_count: i32,
        player_consumed_cards: Vec<u32>,
        opponent_consumed_cards: Vec<u32>,
    ) -> Self {
        Self {
            board,
            turn,
            player_special_count,
            opponent_special_count,
            player_consumed_cards,
            opponent_consumed_cards,
        }
    }

    pub fn is_end(&self) -> bool {
        self.turn == game::TURN_COUNT
    }

    pub fn get_consumed_cards(&self, player_id: PlayerId) -> &[u32] {
        match player_id {
            PlayerId::South => &self.player_consumed_cards,
            PlayerId::North => &self.opponent_consumed_cards,
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Turn: {}", self.turn + 1)?;
        write!(f, "{}", self.board)?;
        writeln!(
            f,
            "Special: {}, {}",
            self.player_special_count, self.opponent_special_count
        )?;
        Ok(())
    }
}

pub fn update_player_state(player_state: &mut PlayerCardState, action: &Action) {
    // update hands
    player_state.consume_card(action.get_consumed_card());
    player_state.draw_card();
}

pub fn update_state(state: &mut State, player_action: &Action, opponent_action: &Action) {
    if !is_valid_action(state, PlayerId::South, player_action) {
        todo!(
            "Invalid action. Player should lose/nstate: {}/naction: {}",
            state,
            player_action
        );
    }
    if !is_valid_action(state, PlayerId::North, opponent_action) {
        todo!(
            "Opponent should lose/nstate: {}/naction: {}",
            state,
            opponent_action
        );
    }

    // Activated special ink count
    let activated_cell_cnts = state.board.count_surrounded_special_ink();

    fill_cells(state, player_action, opponent_action);

    let activated_cell_cnts_later = state.board.count_surrounded_special_ink();

    assert_le!(activated_cell_cnts.0, activated_cell_cnts_later.0);
    assert_le!(activated_cell_cnts.1, activated_cell_cnts_later.1);

    // consume cards
    state
        .player_consumed_cards
        .push(player_action.get_consumed_card().get_id());
    state
        .opponent_consumed_cards
        .push(opponent_action.get_consumed_card().get_id());

    // consume special points
    maybe_consume_special_points(&mut state.player_special_count, player_action);
    state.player_special_count += activated_cell_cnts_later.0 - activated_cell_cnts.0;
    if player_action.is_pass() {
        state.player_special_count += 1;
    }

    maybe_consume_special_points(&mut state.opponent_special_count, opponent_action);
    state.opponent_special_count += activated_cell_cnts_later.1 - activated_cell_cnts.1;
    if opponent_action.is_pass() {
        state.opponent_special_count += 1;
    }
    state.turn += 1
}

fn maybe_consume_special_points(special_points: &mut i32, action: &Action) {
    if let Action::Special(card, _) = action {
        *special_points -= card.get_special_cost();
    }
}

fn fill_cells(state: &mut State, player_action: &Action, opponent_action: &Action) {
    let mut priorities: HashMap<BoardPosition, i32> = HashMap::new();

    // Filling player's cell
    if !player_action.is_pass() {
        let (card, card_position) = player_action.get_card_and_position();
        for (board_pos, cell) in card.get_cells_on_board_coord(card_position) {
            // Modify board
            let fill = cell.cell_type.to_board_cell(PlayerId::South);
            state.board.put_cell(board_pos, fill);
            // Remember the priority
            priorities.insert(board_pos, cell.priority);
        }
    }

    if !opponent_action.is_pass() {
        let (card, card_position) = opponent_action.get_card_and_position();
        for (board_pos, cell) in card.get_cells_on_board_coord(card_position) {
            // Modify board
            let priority: i32 = *priorities
                .get(&board_pos)
                .unwrap_or(&CardCell::PRIORITY_MAX);
            match priority.cmp(&cell.priority) {
                Ordering::Greater => {
                    let fill = cell.cell_type.to_board_cell(PlayerId::North);
                    state.board.put_cell(board_pos, fill);
                }
                Ordering::Equal => {
                    state.board.put_cell(board_pos, BoardCell::Wall);
                }
                Ordering::Less => (),
            }
        }
    }
}

pub fn is_valid_action(state: &State, player_id: PlayerId, action: &Action) -> bool {
    match action {
        Action::Pass(_) => true,
        Action::Put(card, pos) => is_valid_action_put(state, player_id, card, pos, false),
        Action::Special(card, pos) => is_valid_action_put(state, player_id, card, pos, true),
    }
}

fn is_valid_action_put(
    state: &State,
    player_id: PlayerId,
    card: &Card,
    position: &CardPosition,
    special: bool,
) -> bool {
    if special {
        match player_id {
            PlayerId::South => {
                if state.player_special_count < card.get_special_cost() {
                    return false;
                }
            }
            PlayerId::North => {
                if state.opponent_special_count < card.get_special_cost() {
                    return false;
                }
            }
        }
    }

    if has_conflict(&state.board, card, position, special) {
        return false;
    }

    if !has_touching_point(&state.board, player_id, card, position, special) {
        return false;
    }
    true
}

fn has_conflict(board: &Board, card: &Card, card_position: &CardPosition, special: bool) -> bool {
    for (board_pos, _cell) in card.get_cells_on_board_coord(card_position) {
        let board_cell = board.get_cell(board_pos);
        let conflict = match (board_cell, special) {
            (BoardCell::None, _) => false,
            (BoardCell::Wall, _) => true,
            (BoardCell::Ink(_), true) => false,
            (BoardCell::Ink(_), false) => true,
            (BoardCell::Special(_), _) => true,
        };
        if conflict {
            return true;
        }
    }
    false
}

fn has_touching_point(
    board: &Board,
    player_id: PlayerId,
    card: &Card,
    card_position: &CardPosition,
    special: bool,
) -> bool {
    #[rustfmt::skip]
    const AROUND_DIFF: [(i32, i32); 8] = [
        (-1, -1),  (0, -1),  (1, -1),
        (-1,  0),/*(0,  0),*/(1,  0),
        (-1,  1),  (0,  1),  (1,  1),
    ];
    for (board_pos, _cell) in card.get_cells_on_board_coord(card_position) {
        for diff in AROUND_DIFF {
            let board_pos = BoardPosition {
                x: board_pos.x + diff.0,
                y: board_pos.y + diff.1,
            };
            let board_cell = board.get_cell(board_pos);
            let touching = match (board_cell, special) {
                (BoardCell::Ink(pid), false) => player_id == pid,
                (BoardCell::Special(pid), _) => player_id == pid,
                _ => false,
            };
            if touching {
                return true;
            }
        }
    }
    false
}

// TODO: Consider to move test utils under a separate module so that other
// tests can use them without making the `tests` mod public.
#[cfg(test)]
pub mod tests {
    use crate::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    pub fn new_test_state(
        lines: &[&str],
        turn: i32,
        player_special_count: i32,
        opponent_special_count: i32,
        player_consumed_cards: Vec<u32>,
        opponent_consumed_cards: Vec<u32>,
    ) -> State {
        State::new(
            new_test_board(lines),
            turn,
            player_special_count,
            opponent_special_count,
            player_consumed_cards,
            opponent_consumed_cards,
        )
    }

    pub fn new_test_board(lines: &[&str]) -> Board {
        load_board_from_lines(String::from("test board"), lines)
    }

    pub fn new_test_card(lines: &[&str]) -> Card {
        // Using a huge special cost to prevent test codes accidentally
        // use a special attack.
        new_test_card_with_special_cost(lines, 42)
    }

    fn new_test_card_with_special_cost(lines: &[&str], special_cost: i32) -> Card {
        new_test_card_impl(lines, 42, special_cost)
    }

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
        card::load_card_from_lines(
            id,
            String::from("test card"),
            cell_cnt,
            special_cost,
            &lines,
        )
    }

    #[test]
    fn test_conflict() {
        init();

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "########",
            "#...P..#",
            "########"
        ], 0, 0, 0, vec![], vec![]);
        let card = new_test_card(&["==="]);

        // NO conflict
        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Up,
                }
            )
        ));

        // DO conflict with wall
        assert!(!is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 5,
                    y: 1,
                    rotation: Rotation::Up,
                }
            )
        ));

        // DO conflict with ink
        assert!(!is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card,
                CardPosition {
                    x: 2,
                    y: 1,
                    rotation: Rotation::Up,
                }
            )
        ));
    }

    #[test]
    fn test_conflict_with_opponents_ink() {
        init();

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "######",
            "#P.o.#",
            "######"
        ], 0, 0, 0, vec![], vec![]);
        let card = new_test_card(&["==="]);

        assert!(!is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Up,
                }
            )
        ));
    }

    #[test]
    fn test_touching_point() {
        init();

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "########",
            "#.....P#",
            "########"
        ], 0, 0, 0, vec![], vec![]);
        let card = new_test_card(&["==="]);

        // NO touching point
        assert!(!is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Up,
                }
            )
        ));

        // touch!
        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card,
                CardPosition {
                    x: 3,
                    y: 1,
                    rotation: Rotation::Up,
                }
            )
        ));
    }

    #[test]
    fn test_rotation() {
        init();

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "#########",
            "####..###",
            "####.####",
            "#.##.####",
            "#...P...#",
            "####.##.#",
            "####.####",
            "###..####",
            "#########",
        ], 0, 0, 0, vec![], vec![]);
        #[rustfmt::skip]
        let card = new_test_card(&[
            "===",
            "  ="
        ]);

        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 5,
                    y: 4,
                    rotation: Rotation::Up,
                }
            )
        ));
        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 3,
                    y: 5,
                    rotation: Rotation::Right,
                }
            )
        ));
        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 3,
                    rotation: Rotation::Down,
                }
            )
        ));
        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card,
                CardPosition {
                    x: 4,
                    y: 1,
                    rotation: Rotation::Left,
                }
            )
        ));
    }

    #[test]
    fn test_special() {
        init();

        #[rustfmt::skip]
        let card = new_test_card_with_special_cost(&[
            "===",
        ], 2);

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "###",
            "#.#",
            "#.#",
            "#.#",
            "#p#",
            "###",
        ],
        0,
        2, // player's special points
        0,
        vec![],
        vec![]);
        // Special attack can't be triggered without special ink on the board.
        assert!(!is_valid_action(
            &state,
            PlayerId::South,
            &Action::Special(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                }
            )
        ));

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "###",
            "#.#",
            "#.#",
            "#.#",
            "#P#",
            "###",
        ],
        0,
        2,  // player's special points
        0,
        vec![],
        vec![]);
        // Now we have a special ink.
        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Special(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                }
            )
        ));

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "###",
            "#o#",
            "#p#",
            "#.#",
            "#P#",
            "###",
        ],
        0,
        2,
        0, vec![], vec![]);
        // Special attack can overdraw other ink
        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Special(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                }
            )
        ));
        assert!(!is_valid_action(
            &state,
            PlayerId::South,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                }
            )
        ));

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "###",
            "#P#",
            "#.#",
            "#.#",
            "#P#",
            "###",
        ],
        0,
        2,
        0, vec![], vec![]);
        // Special attack can NOT overdraw player's SPECIAL ink too
        assert!(!is_valid_action(
            &state,
            PlayerId::South,
            &Action::Special(
                card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                }
            )
        ));
    }

    #[test]
    fn test_special_not_enough_points() {
        init();

        // this card requires 2 special points to use.
        #[rustfmt::skip]
        let card = new_test_card_with_special_cost(&[
            "===",
        ], 2);

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "###",
            "#.#",
            "#.#",
            "#.#",
            "#P#",
            "###",
        ],
        0,
        2,  // player's special points
        0, vec![], vec![]);
        // Now we have a special ink.
        assert!(is_valid_action(
            &state,
            PlayerId::South,
            &Action::Special(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                }
            )
        ));

        #[rustfmt::skip]
        let state = new_test_state(
            &[
            "###",
            "#.#",
            "#.#",
            "#.#",
            "#P#",
            "###",
        ],
        0,
        1,  // player's special points is NOT enough.
        0, vec![], vec![]);
        assert!(!is_valid_action(
            &state,
            PlayerId::South,
            &Action::Special(
                card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                }
            )
        ));
    }

    #[test]
    fn test_update_state() {
        init();

        #[rustfmt::skip]
        let mut state = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0, vec![], vec![]
        );
        #[rustfmt::skip]
        let card = new_test_card(&[
            "=",
            "*",
            "="
        ]);

        update_state(
            &mut state,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 2,
                    y: 1,
                    rotation: Rotation::Up,
                },
            ),
            &Action::Put(
                card,
                CardPosition {
                    x: 4,
                    y: 1,
                    rotation: Rotation::Up,
                },
            ),
        );

        #[rustfmt::skip]
        let expected = new_test_state(
            &[
            "#######",
            "#.pOo.#",
            "#.P.O.#",
            "#.pPo.#",
            "#######"],
            1,
            0,
            0, vec![42], vec![42]
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }

    #[test]
    fn test_update_state_conflict() {
        init();

        #[rustfmt::skip]
        let mut state = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0, vec![], vec![]
        );
        #[rustfmt::skip]
        let card = new_test_card(&[
            "=*=",
        ]);

        update_state(
            &mut state,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
            &Action::Put(
                card,
                CardPosition {
                    x: 3,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
        );

        #[rustfmt::skip]
        let expected = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#pP#Oo#",
            "#..P..#",
            "#######"],
            1,
            0,
            0, vec![42], vec![42]
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }

    #[test]
    fn test_update_state_prioritize_smaller() {
        init();

        #[rustfmt::skip]
        let mut state = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0, vec![], vec![]
        );
        #[rustfmt::skip]
        let card = new_test_card(&[
            "=*=",
        ]);
        #[rustfmt::skip]
        let card_large = new_test_card(&[
            "=*=",
            "  =",
        ]);

        update_state(
            &mut state,
            &Action::Put(
                card,
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
            &Action::Put(
                card_large,
                CardPosition {
                    x: 3,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
        );

        // smaller card should be prioritized
        #[rustfmt::skip]
        let expected = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#pPpOo#",
            "#..P.o#",
            "#######"],
            1,
            0,
            0, vec![42], vec![42]
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }

    #[test]
    fn test_update_state_prioritize_special_ink() {
        init();

        #[rustfmt::skip]
        let mut state = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0, vec![], vec![]
        );
        #[rustfmt::skip]
        let card = new_test_card(&[
            "=*=",
        ]);
        #[rustfmt::skip]
        let card_large = new_test_card(&[
            "*==",
            "  =",
        ]);

        update_state(
            &mut state,
            &Action::Put(
                card,
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
            &Action::Put(
                card_large,
                CardPosition {
                    x: 3,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
        );

        // smaller card should be prioritized
        #[rustfmt::skip]
        let expected = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#pPOoo#",
            "#..P.o#",
            "#######"],
            1,
            0,
            0, vec![42], vec![42]
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }

    #[test]
    fn test_update_state_special_attack_should_not_be_prioritized() {
        init();

        #[rustfmt::skip]
        let mut state = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            4, vec![], vec![]
        );
        #[rustfmt::skip]
        let card = new_test_card_with_special_cost(&[
            "=*=",
        ], 1);

        update_state(
            &mut state,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
            &Action::Special(
                card,
                CardPosition {
                    x: 3,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
        );

        // Opponent used special attack.
        // The conflicted cell should become a wall.
        #[rustfmt::skip]
        let expected = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#pP#Oo#",
            "#..P..#",
            "#######"],
            1,
            0,
            3, vec![42], vec![42]
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }

    #[test]
    fn test_update_state_pass() {
        init();

        #[rustfmt::skip]
        let mut state = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0, vec![], vec![]
        );
        #[rustfmt::skip]
        let card = new_test_card(&[
            "=*=",
        ]);

        update_state(
            &mut state,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                },
            ),
            &Action::Pass(card),
        );

        // Opponent used special attack.
        // The conflicted cell should become a wall.
        #[rustfmt::skip]
        let expected = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#pPp..#",
            "#..P..#",
            "#######"],
            1,
            0,
            1, // Passed player earned a one special point.
            vec![42], vec![42]
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }

    #[test]
    fn test_update_state_surround_special_ink() {
        init();

        #[rustfmt::skip]
        let mut state = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0, vec![], vec![]
        );
        #[rustfmt::skip]
        let card = new_test_card(&[
            "= =",
            "===",
            "= =",
        ]);

        update_state(
            &mut state,
            &Action::Put(
                card.clone(),
                CardPosition {
                    x: 2,
                    y: 1,
                    rotation: Rotation::Up,
                },
            ),
            &Action::Pass(card),
        );

        // Opponent used special attack.
        // The conflicted cell should become a wall.
        #[rustfmt::skip]
        let expected = new_test_state(
            &[
            "#######",
            "#.pOp.#",
            "#.ppp.#",
            "#.pPp.#",
            "#######"],
            1,
            1, // Player has a surrounded special ink @ [33]
            2, // Passed opponent earned one more special point.
            vec![42], vec![42]
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }

    #[test]
    fn test_update_state_new_special_ink_surrounded() {
        init();

        #[rustfmt::skip]
        let mut state = new_test_state(
            &[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0, vec![], vec![]
        );
        #[rustfmt::skip]
        let card = new_test_card(&[
            " *",
            " =",
            "==",
        ]);
        #[rustfmt::skip]
        let card2 = new_test_card(&[
            "=",
            "==",
        ]);

        update_state(
            &mut state,
            &Action::Put(
                card,
                CardPosition {
                    x: 4,
                    y: 1,
                    rotation: Rotation::Up,
                },
            ),
            &Action::Put(
                card2,
                CardPosition {
                    x: 4,
                    y: 1,
                    rotation: Rotation::Up,
                },
            ),
        );

        // Opponent used special attack.
        // The conflicted cell should become a wall.
        #[rustfmt::skip]
        let expected = new_test_state(
            &[
            "#######",
            "#..OoP#",
            "#...oo#",
            "#..Ppp#",
            "#######"],
            1,
            1, // Player has a surrounded special ink @ [33]
            0, vec![42], vec![42]
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }
} // mod tests
