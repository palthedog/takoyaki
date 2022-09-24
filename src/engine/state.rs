use std::{cmp::Ordering, collections::HashMap, fmt::Display};

use log::*;
use more_asserts::*;

use super::{
    board::{Board, BoardCell, BoardPosition},
    card::{Card, CardCell},
};
use super::{
    card::CardPosition,
    game::{Action, PlayerId},
};

#[derive(Debug, Clone)]
pub struct PlayerState<'a> {
    hands: Vec<&'a Card>,
    deck: Vec<&'a Card>,
}

impl<'a> PlayerState<'a> {
    pub fn new(hands: &[&'a Card], deck: &[&'a Card]) -> PlayerState<'a> {
        PlayerState {
            hands: hands.to_vec(),
            deck: deck.to_vec(),
        }
    }

    pub fn get_hands(&self) -> &[&Card] {
        &self.hands
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

impl<'a> Display for PlayerState<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Hands:[")?;
        for card in self.hands.iter() {
            f.write_str(&textwrap::indent(&format!("{}\n", card), "    "))?;
        }
        writeln!(f, "]")?;

        Ok(())
    }
}

// TODO: Rename it to PublicState
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub board: Board,
    pub turn: u32,
    pub player_special_count: u32,
    pub opponent_special_count: u32,
}

impl State {
    pub fn new(
        board: Board,
        turn: u32,
        player_special_count: u32,
        opponent_special_count: u32,
    ) -> State {
        State {
            board,
            turn,
            player_special_count,
            opponent_special_count,
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Turn: {}", self.turn + 1)?;
        writeln!(
            f,
            "Special: player: {}, opponent: {}",
            self.player_special_count, self.opponent_special_count
        )?;
        write!(f, "{}", self.board)
    }
}

pub fn update_player_state(player_state: &mut PlayerState, action: &Action) {
    player_state.consume_card(action.get_consumed_card());
    player_state.draw_card();
}

pub fn update_state(state: &mut State, player_action: &Action, opponent_action: &Action) {
    if !is_valid_action(&state.board, PlayerId::Player, player_action) {
        todo!("Player should lose");
    }
    if !is_valid_action(&state.board, PlayerId::Opponent, opponent_action) {
        todo!("Opponent should lose");
    }

    // Activated special ink count
    let activated_cell_cnts = state.board.count_surrounded_special_ink();

    fill_cells(state, player_action, opponent_action);

    let activated_cell_cnts_later = state.board.count_surrounded_special_ink();

    assert_le!(activated_cell_cnts.0, activated_cell_cnts_later.0);
    assert_le!(activated_cell_cnts.1, activated_cell_cnts_later.1);

    state.player_special_count += activated_cell_cnts_later.0 - activated_cell_cnts.0;
    if player_action.is_pass() {
        state.player_special_count += 1;
    }

    state.opponent_special_count += activated_cell_cnts_later.1 - activated_cell_cnts.1;
    if opponent_action.is_pass() {
        state.opponent_special_count += 1;
    }

    warn!("TODO: Check special inks and update special points.");
    state.turn += 1
}

fn fill_cells(state: &mut State, player_action: &Action, opponent_action: &Action) {
    let mut priorities: HashMap<BoardPosition, u32> = HashMap::new();

    // Filling player's cell
    if let Action::Put(card, card_position) = player_action {
        for (board_pos, &cell) in card.get_putting_cells(card_position) {
            // Modify board
            let fill = cell.cell_type.to_board_cell(PlayerId::Player);
            debug!("Filling {} at {}", board_pos, fill);
            state.board.put_cell(board_pos, fill);
            // Remember the priority
            priorities.insert(board_pos, cell.priority);
        }
    }

    if let Action::Put(card, card_position) = opponent_action {
        for (board_pos, &cell) in card.get_putting_cells(card_position) {
            // Modify board
            let priority: u32 = *priorities
                .get(&board_pos)
                .unwrap_or(&CardCell::PRIORITY_MAX);
            match priority.cmp(&cell.priority) {
                Ordering::Greater => {
                    let fill = cell.cell_type.to_board_cell(PlayerId::Opponent);
                    debug!("Filling {} at {}", board_pos, fill);
                    state.board.put_cell(board_pos, fill);
                }
                Ordering::Equal => {
                    debug!("Filling {} at {}", board_pos, BoardCell::Wall);
                    state.board.put_cell(board_pos, BoardCell::Wall);
                }
                Ordering::Less => (),
            }
        }
    }
}

pub fn is_valid_action(board: &Board, player_id: PlayerId, action: &Action) -> bool {
    match action {
        Action::Pass(_) => true,
        Action::Put(card, pos) => check_action_put(board, player_id, card, pos),
    }
}

fn check_action_put(
    board: &Board,
    player_id: PlayerId,
    card: &Card,
    position: &CardPosition,
) -> bool {
    if has_conflict(board, card, position) {
        return false;
    }

    if !has_touching_point(board, player_id, card, position) {
        return false;
    }
    true
}

fn has_conflict(board: &Board, card: &Card, card_position: &CardPosition) -> bool {
    let special = card_position.special;

    for (board_pos, &_cell) in card.get_putting_cells(card_position) {
        let board_cell = board.get_cell(board_pos);
        let conflict = match (board_cell, special) {
            (crate::engine::board::BoardCell::None, _) => false,
            (crate::engine::board::BoardCell::Wall, _) => true,
            (crate::engine::board::BoardCell::Ink(_), true) => false,
            (crate::engine::board::BoardCell::Ink(_), false) => true,
            (crate::engine::board::BoardCell::Special(_), _) => true,
        };
        if conflict {
            trace!(
                "A cell has conflict at: {:?}. cell type: {:?}",
                board_pos,
                board_cell
            );
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
    let cells = card.get_cells(card_position.rotation);
    for cell_pos in cells.keys() {
        for diff in AROUND_DIFF {
            let board_pos = BoardPosition {
                x: card_position.x + cell_pos.x + diff.0,
                y: card_position.y + cell_pos.y + diff.1,
            };
            trace!("  board pos: {}", board_pos);

            let board_cell = board.get_cell(board_pos);
            let touching = match (board_cell, special) {
                (crate::engine::board::BoardCell::Ink(pid), false) => player_id == pid,
                (crate::engine::board::BoardCell::Special(pid), _) => player_id == pid,
                _ => false,
            };
            if touching {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{board, card, game::Rotation};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn new_test_state(
        lines: &[&str],
        turn: u32,
        player_special_count: u32,
        opponent_special_count: u32,
    ) -> State {
        State::new(
            new_test_board(lines),
            turn,
            player_special_count,
            opponent_special_count,
        )
    }

    fn new_test_board(lines: &[&str]) -> crate::engine::board::Board {
        let lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
        board::load_board_from_lines(42, String::from("test board"), &lines)
    }

    fn new_test_card(lines: &[&str]) -> crate::engine::card::Card {
        let lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
        let cell_cnt: u32 = lines
            .iter()
            .map(|line| {
                line.as_bytes()
                    .iter()
                    .filter(|&ch| *ch == b'=' || *ch == b'*')
                    .count() as u32
            })
            .sum();
        card::load_card_from_lines(42, String::from("test card"), cell_cnt, 42, &lines)
    }

    #[test]
    fn test_conflict() {
        init();

        #[rustfmt::skip]
        let board = new_test_board(&[
            "########",
            "#...P..#",
            "########"
        ]);
        let card = new_test_card(&["==="]);

        // NO conflict
        assert!(is_valid_action(
            &board,
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
            &board.clone(),
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
            &board.clone(),
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
        let board = new_test_board(&[
            "######",
            "#P.o.#",
            "######"
        ]);
        let card = new_test_card(&["==="]);

        assert!(!is_valid_action(
            &board.clone(),
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
        let board = new_test_board(&[
            "########",
            "#.....P#",
            "########"
        ]);
        let card = new_test_card(&["==="]);

        // NO touching point
        assert!(!is_valid_action(
            &board.clone(),
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
            &board.clone(),
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
        let board = new_test_board(&[
            "######",
            "##...#",
            "##.#.#",
            "#..##",
            "##p#",
            "####",
        ]);
        #[rustfmt::skip]
        let card = new_test_card(&[
            "===",
            "  ="
        ]);

        // Only Rotation::Right one should fit

        assert!(!is_valid_action(
            &board.clone(),
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
        assert!(is_valid_action(
            &board.clone(),
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 2,
                    y: 1,
                    rotation: Rotation::Right,
                    special: false
                }
            )
        ));
        assert!(!is_valid_action(
            &board.clone(),
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 2,
                    y: 1,
                    rotation: Rotation::Down,
                    special: false
                }
            )
        ));
        assert!(!is_valid_action(
            &board.clone(),
            PlayerId::Player,
            &Action::Put(
                &card,
                CardPosition {
                    x: 2,
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
        let card = new_test_card(&[
            "===",
        ]);

        #[rustfmt::skip]
        let board = new_test_board(&[
            "###",
            "#.#",
            "#.#",
            "#.#",
            "#p#",
            "###",
        ]);
        // Special attack can't be triggered without special ink on the board.
        assert!(!is_valid_action(
            &board.clone(),
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
        let board = new_test_board(&[
            "###",
            "#.#",
            "#.#",
            "#.#",
            "#P#",
            "###",
        ]);
        // Now we have a special ink.
        assert!(is_valid_action(
            &board.clone(),
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
        let board = new_test_board(&[
            "###",
            "#o#",
            "#p#",
            "#.#",
            "#P#",
            "###",
        ]);
        // Special attack can overdraw other ink
        assert!(is_valid_action(
            &board.clone(),
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
            &board.clone(),
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
        let board = new_test_board(&[
            "###",
            "#P#",
            "#.#",
            "#.#",
            "#P#",
            "###",
        ]);
        // Special attack can NOT overdraw player's SPECIAL ink too
        assert!(!is_valid_action(
            &board.clone(),
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
        let mut state = new_test_state(&[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0
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
        let expected = new_test_state(&[
            "#######",
            "#.pOo.#",
            "#.P.O.#",
            "#.pPo.#",
            "#######"],
            1,
            0,
            0
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
        let mut state = new_test_state(&[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0
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
        let expected = new_test_state(&[
            "#######",
            "#..O..#",
            "#pP#Oo#",
            "#..P..#",
            "#######"],
            1,
            0,
            0
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
        let mut state = new_test_state(&[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0
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
        let expected = new_test_state(&[
            "#######",
            "#..O..#",
            "#pPpOo#",
            "#..P.o#",
            "#######"],
            1,
            0,
            0
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
        let mut state = new_test_state(&[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0
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
        let expected = new_test_state(&[
            "#######",
            "#..O..#",
            "#pPOoo#",
            "#..P.o#",
            "#######"],
            1,
            0,
            0
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
        let mut state = new_test_state(&[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0
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
                    special: true,
                },
            ),
        );

        // Opponent used special attack.
        // The conflicted cell should become a wall.
        #[rustfmt::skip]
        let expected = new_test_state(&[
            "#######",
            "#..O..#",
            "#pP#Oo#",
            "#..P..#",
            "#######"],
            1,
            0,
            0
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
        let mut state = new_test_state(&[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0
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
        let expected = new_test_state(&[
            "#######",
            "#..O..#",
            "#pPp..#",
            "#..P..#",
            "#######"],
            1,
            0,
            1 // Passed player earned a one special point.
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
        let mut state = new_test_state(&[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0
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
        let expected = new_test_state(&[
            "#######",
            "#.pOp.#",
            "#.ppp.#",
            "#.pPp.#",
            "#######"],
            1,
            1, // Player has a surrounded special ink @ [33]
            2 // Passed opponent earned one more special point.
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
        let mut state = new_test_state(&[
            "#######",
            "#..O..#",
            "#.....#",
            "#..P..#",
            "#######"],
            0,
            0,
            0
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
        let expected = new_test_state(&[
            "#######",
            "#..OoP#",
            "#...oo#",
            "#..Ppp#",
            "#######"],
            1,
            1, // Player has a surrounded special ink @ [33]
            0
        );
        assert_eq!(
            state, expected,
            "\nActual:\n{}\nExpected:\n{}",
            state, expected
        );
    }
} // mod tests
