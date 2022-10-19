use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::Display,
    hash::{Hash, Hasher},
};

use log::*;
use more_asserts::*;

use super::{
    board::{Board, BoardCell, BoardPosition},
    card::{Card, CardCell},
    game,
};
use super::{
    card::CardPosition,
    game::{Action, PlayerId},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PlayerCardState<'c> {
    hands: Vec<&'c Card>,
    deck: Vec<&'c Card>,
}

impl<'a> PlayerCardState<'a> {
    pub fn new(hands: Vec<&'a Card>, deck: Vec<&'a Card>) -> PlayerCardState<'a> {
        PlayerCardState {
            hands: hands,
            deck: deck,
        }
    }

    pub fn get_hands(&self) -> &[&'a Card] {
        &self.hands
    }

    pub fn get_deck(&self) -> &[&'a Card] {
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

impl<'a> Display for PlayerCardState<'a> {
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

    pub player_consumed_cards: Vec<u32>,
    pub opponent_consumed_cards: Vec<u32>,
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
    if !is_valid_action(state, PlayerId::Player, player_action) {
        todo!("Player should lose");
    }
    if !is_valid_action(state, PlayerId::Opponent, opponent_action) {
        todo!("Opponent should lose");
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
    if let Action::Put(card, card_position) = action {
        if card_position.special {
            *special_points -= card.get_special_cost();
        }
    }
}

fn fill_cells(state: &mut State, player_action: &Action, opponent_action: &Action) {
    let mut priorities: HashMap<BoardPosition, i32> = HashMap::new();

    // Filling player's cell
    if let Action::Put(card, card_position) = player_action {
        for (board_pos, &cell) in card.get_cells_on_board_coord(card_position) {
            // Modify board
            let fill = cell.cell_type.to_board_cell(PlayerId::Player);
            state.board.put_cell(board_pos, fill);
            // Remember the priority
            priorities.insert(board_pos, cell.priority);
        }
    }

    if let Action::Put(card, card_position) = opponent_action {
        for (board_pos, &cell) in card.get_cells_on_board_coord(card_position) {
            // Modify board
            let priority: i32 = *priorities
                .get(&board_pos)
                .unwrap_or(&CardCell::PRIORITY_MAX);
            match priority.cmp(&cell.priority) {
                Ordering::Greater => {
                    let fill = cell.cell_type.to_board_cell(PlayerId::Opponent);
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
        Action::Put(card, pos) => is_valid_action_put(state, player_id, card, pos),
    }
}

fn is_valid_action_put(
    state: &State,
    player_id: PlayerId,
    card: &Card,
    position: &CardPosition,
) -> bool {
    if position.special {
        match player_id {
            PlayerId::Player => {
                if state.player_special_count < card.get_special_cost() {
                    return false;
                }
            }
            PlayerId::Opponent => {
                if state.opponent_special_count < card.get_special_cost() {
                    return false;
                }
            }
        }
    }

    if has_conflict(&state.board, card, position) {
        return false;
    }

    if !has_touching_point(&state.board, player_id, card, position) {
        return false;
    }
    true
}

fn has_conflict(board: &Board, card: &Card, card_position: &CardPosition) -> bool {
    let special = card_position.special;
    for (board_pos, &_cell) in card.get_cells_on_board_coord(card_position) {
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
) -> bool {
    #[rustfmt::skip]
    const AROUND_DIFF: [(i32, i32); 8] = [
        (-1, -1),  (0, -1),  (1, -1),
        (-1,  0),/*(0,  0),*/(1,  0),
        (-1,  1),  (0,  1),  (1,  1),
    ];
    let special = card_position.special;
    for (board_pos, &_cell) in card.get_cells_on_board_coord(card_position) {
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
    use super::*;
    use crate::engine::{board, card, game::Rotation};

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

    pub fn new_test_board(lines: &[&str]) -> crate::engine::board::Board {
        board::load_board_from_lines(String::from("test board"), lines)
    }

    pub fn new_test_card(lines: &[&str]) -> crate::engine::card::Card {
        // Using a huge special cost to prevent test codes accidentally
        // use a special attack.
        new_test_card_with_special_cost(lines, 42)
    }

    fn new_test_card_with_special_cost(lines: &[&str], special_cost: i32) -> Card {
        new_test_card_impl(lines, 42, special_cost)
    }

    pub fn new_test_card_impl(
        lines: &[&str],
        id: u32,
        special_cost: i32,
    ) -> crate::engine::card::Card {
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false
                }
            )
        ));

        // DO conflict with wall
        assert!(!is_valid_action(
            &state,
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 5,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false
                }
            )
        ));

        // DO conflict with ink
        assert!(!is_valid_action(
            &state,
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 2,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false
                }
            )
        ));

        // touch!
        assert!(is_valid_action(
            &state,
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 3,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 5,
                    y: 4,
                    rotation: Rotation::Up,
                    special: false
                }
            )
        ));
        assert!(is_valid_action(
            &state,
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 3,
                    y: 5,
                    rotation: Rotation::Right,
                    special: false
                }
            )
        ));
        assert!(is_valid_action(
            &state,
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 3,
                    rotation: Rotation::Down,
                    special: false
                }
            )
        ));
        assert!(is_valid_action(
            &state,
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 4,
                    y: 1,
                    rotation: Rotation::Left,
                    special: false
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                    special: true
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                    special: true
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                    special: true
                }
            )
        ));
        assert!(!is_valid_action(
            &state,
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                    special: false // special is OFF
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                    special: true
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                    special: true
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
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 1,
                    y: 1,
                    rotation: Rotation::Right,
                    special: true
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
                &card,
                CardPosition {
                    x: 2,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false,
                },
            ),
            &Action::Put(
                &card,
                CardPosition {
                    x: 4,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false,
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
                &card,
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                    special: false,
                },
            ),
            &Action::Put(
                &card,
                CardPosition {
                    x: 3,
                    y: 2,
                    rotation: Rotation::Up,
                    special: false,
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
                &card,
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                    special: false,
                },
            ),
            &Action::Put(
                &card_large,
                CardPosition {
                    x: 3,
                    y: 2,
                    rotation: Rotation::Up,
                    special: false,
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
                &card,
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                    special: false,
                },
            ),
            &Action::Put(
                &card_large,
                CardPosition {
                    x: 3,
                    y: 2,
                    rotation: Rotation::Up,
                    special: false,
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
                &card,
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                    special: false,
                },
            ),
            &Action::Put(
                &card,
                CardPosition {
                    x: 3,
                    y: 2,
                    rotation: Rotation::Up,
                    special: true,
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
                &card,
                CardPosition {
                    x: 1,
                    y: 2,
                    rotation: Rotation::Up,
                    special: false,
                },
            ),
            &Action::Pass(&card),
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
                &card,
                CardPosition {
                    x: 2,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false,
                },
            ),
            &Action::Pass(&card),
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
                &card,
                CardPosition {
                    x: 4,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false,
                },
            ),
            &Action::Put(
                &card2,
                CardPosition {
                    x: 4,
                    y: 1,
                    rotation: Rotation::Up,
                    special: false,
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
