use log::debug;
use log::trace;

use super::board::Board;
use super::board::BoardPosition;
use super::card::Card;
use super::card::CardPosition;
use super::game::Action;
use super::game::State;

pub fn is_valid_action(state: &State, action: Action) -> bool {
    match action {
        Action::Pass(_) => true,
        Action::Put(card, pos) => check_action_put(state, &card, &pos),
    }
}

fn check_action_put(state: &State, card: &Card, position: &CardPosition) -> bool {
    if has_conflict_with_wall(&state.board, position, card) {
        return false;
    }
    true
}

fn has_conflict_with_wall(board: &Board, card_position: &CardPosition, card: &Card) -> bool {
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
        if board_cell.is_wall() {
            trace!(
                "A cell has conflict with a wall at: {:?}. cell type: {:?}",
                board_pos,
                board_cell
            );
            return true;
        }
    }
    false
}

pub fn update(state: &mut State, player_action: Action, opponent_action: Action) -> bool {
    debug!(
        "Player action is valid? {}",
        is_valid_action(state, player_action)
    );
    debug!(
        "Opponent action is valid? {}",
        is_valid_action(state, opponent_action)
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
    fn test_conflict_with_wall() {
        init();

        #[rustfmt::skip]
        let board = test_board(&[
            "#####",
            "#...#",
            "#####"
        ]);
        let card = test_card(&["==="]);

        // NO conflict
        assert!(!has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 1,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
            &card
        ));

        // DO conflict
        assert!(has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 1,
                y: 0,
                rotation: Rotation::Up,
                special: false
            },
            &card
        ));
        assert!(has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 1,
                y: 2,
                rotation: Rotation::Up,
                special: false
            },
            &card
        ));
        assert!(has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 0,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
            &card
        ));
    }

    #[test]
    fn test_conflict_with_wall_rot() {
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
        assert!(!has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 1,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
            &card
        ));
        assert!(!has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 1,
                y: 5,
                rotation: Rotation::Right,
                special: false
            },
            &card
        ));
        assert!(!has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 1,
                y: 9,
                rotation: Rotation::Down,
                special: false
            },
            &card
        ));
        assert!(!has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 1,
                y: 13,
                rotation: Rotation::Left,
                special: false
            },
            &card
        ));
    }

    #[test]
    fn test_conflict_with_wall_out_side_of_board() {
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

        assert!(has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 1,
                y: 1,
                rotation: Rotation::Up,
                special: false
            },
            &card
        ));
        assert!(has_conflict_with_wall(
            &board,
            &CardPosition {
                x: 2,
                y: 1,
                rotation: Rotation::Right,
                special: false
            },
            &card
        ));
    }
}
