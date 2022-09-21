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
        Action::Put(card, pos) => check_action_put(state, player_id, &card, &pos),
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

    fn test_board(lines: &[&str]) -> crate::engine::board::Board {
        let lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
        board::load_board_from_lines(42, String::from("test board"), &lines)
    }

    fn test_card(lines: &[&str]) -> crate::engine::card::Card {
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
        let board = test_board(&[
            "#####",
            "#...#",
            "#####"
        ]);
        let card = test_card(&["==="]);

        // NO conflict
        assert!(!has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
        ));

        // DO conflict
        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 0,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 2,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 0,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
        ));
    }

    #[test]
    fn test_conflict_with_rotation() {
        init();

        #[rustfmt::skip]
        let board = test_board(&[
            "#####",
            "#...#",
            "###.#",
            "###.#",
            "#####",
            "###.#",
            "###.#",
            "#...#",
            "#####",
            "#.###",
            "#.###",
            "#...#",
            "#####",
            "#...#",
            "#.###",
            "#.###",
            "#####",
        ]);
        #[rustfmt::skip]
        let card = test_card(&[
            "===",
            "  =",
            "  =",
        ]);

        // NO conflict
        assert!(!has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(!has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 5,
                rotation: Rotation::Right,
                special: false
            },
        ));
        assert!(!has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 9,
                rotation: Rotation::Down,
                special: false
            },
        ));
        assert!(!has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 13,
                rotation: Rotation::Left,
                special: false
            },
        ));
    }

    #[test]
    fn test_conflict_out_side_of_board() {
        init();

        #[rustfmt::skip]
        let board = test_board(&[
            "#####",
            "##.##",
            "#...#",
            "##.##",
            "#####"
        ]);
        #[rustfmt::skip]
        let card = test_card(&[
            "=== ="
        ]);

        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 2,
                y: 1,
                rotation: Rotation::Right,
                special: false
            },
        ));
    }

    #[test]
    fn test_conflict_with_ink() {
        init();

        #[rustfmt::skip]
        let board = test_board(&[
            "#####",
            "##o##",
            "#p..#",
            "##.##",
            "#####"
        ]);
        #[rustfmt::skip]
        let card = test_card(&[
            "==="
        ]);

        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 2,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(!has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 2,
                rotation: Rotation::Up,
                special: true // special is ON
            },
        ));

        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 2,
                y: 1,
                rotation: Rotation::Right,
                special: false
            },
        ));
        assert!(!has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 2,
                y: 1,
                rotation: Rotation::Right,
                special: true // special is ON
            },
        ));
    }

    #[test]
    fn test_conflict_with_special() {
        init();

        #[rustfmt::skip]
        let board = test_board(&[
            "#####",
            "##O##",
            "#P..#",
            "##.##",
            "#####"
        ]);
        #[rustfmt::skip]
        let card = test_card(&[
            "==="
        ]);

        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 2,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 1,
                y: 2,
                rotation: Rotation::Up,
                special: true // special is ON
            },
        ));

        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 2,
                y: 1,
                rotation: Rotation::Right,
                special: false
            },
        ));
        assert!(has_conflict(
            &board,
            &card,
            &CardPosition {
                x: 2,
                y: 1,
                rotation: Rotation::Right,
                special: true // special is ON
            },
        ));
    }

    #[test]
    fn test_touching_with_ink() {
        init();

        #[rustfmt::skip]
        let board = test_board(&[
            "#####",
            "#p..#",
            "#...#",
            "#..o#",
            "#####"
        ]);
        let card = test_card(&["="]);

        assert!(has_touching_point(
            &board,
            PlayerId::Player,
            &card,
            &CardPosition {
                x: 2,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(has_touching_point(
            &board,
            PlayerId::Player,
            &card,
            &CardPosition {
                x: 1,
                y: 2,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(has_touching_point(
            &board,
            PlayerId::Player,
            &card,
            &CardPosition {
                x: 2,
                y: 2,
                rotation: Rotation::Up,
                special: false
            },
        ));

        // Opponent's ink shouldn't work
        assert!(!has_touching_point(
            &board,
            PlayerId::Player,
            &card,
            &CardPosition {
                x: 3,
                y: 2,
                rotation: Rotation::Up,
                special: false
            },
        ));
        assert!(!has_touching_point(
            &board,
            PlayerId::Player,
            &card,
            &CardPosition {
                x: 2,
                y: 3,
                rotation: Rotation::Up,
                special: false
            },
        ));
    }
}
