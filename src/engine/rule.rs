use log::debug;
use log::trace;

use super::board::Board;
use super::board::BoardPosition;
use super::board::PlayerId;
use super::card::Card;
use super::card::CardPosition;
use super::game::Action;
use super::game::State;

pub fn is_valid_action(state: &State, player_id: PlayerId, action: Action) -> bool {
    match action {
        Action::Pass(_) => true,
        Action::Put(card, pos) => check_action_put(state, player_id, card, &pos),
    }
}

fn check_action_put(
    state: &State,
    player_id: PlayerId,
    card: &Card,
    position: &CardPosition,
) -> bool {
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
    let cells = card.get_cells(card_position.rotation);
    trace!("card pos: [{},{}]", card_position.x, card_position.y);
    for cell_pos in cells.keys() {
        trace!("  cell pos: {}", cell_pos);
        let board_pos = BoardPosition {
            x: card_position.x + cell_pos.x,
            y: card_position.y + cell_pos.y,
        };
        trace!("  board pos: {}", board_pos);
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

pub fn update(state: &mut State, player_action: Action, opponent_action: Action) -> bool {
    debug!(
        "Player action is valid? {}",
        is_valid_action(state, PlayerId::Player, player_action)
    );
    debug!(
        "Opponent action is valid? {}",
        is_valid_action(state, PlayerId::Opponent, opponent_action)
    );
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{board, card, game::Rotation};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board,
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board,
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board.clone(),
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board,
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
            &State {
                board: board,
                turn: 0
            },
            PlayerId::Player,
            Action::Put(
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
}
