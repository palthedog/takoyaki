use std::{
    fmt::{Display, Formatter},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::PathBuf,
};

use log::*;

use super::game::PlayerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardCell {
    None,
    Wall,
    Ink(PlayerId),
    Special(PlayerId),
}

impl Display for BoardCell {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

impl BoardCell {
    fn to_char(self) -> char {
        match self {
            BoardCell::None => '.',
            BoardCell::Wall => '#',
            BoardCell::Ink(PlayerId::Player) => 'p',
            BoardCell::Special(PlayerId::Player) => 'P',
            BoardCell::Ink(PlayerId::Opponent) => 'o',
            BoardCell::Special(PlayerId::Opponent) => 'O',
        }
    }

    fn from_char(ch: char) -> Result<BoardCell, String> {
        match ch {
            '.' => Ok(BoardCell::None),
            ' ' | '#' => Ok(BoardCell::Wall),
            'p' => Ok(BoardCell::Ink(PlayerId::Player)),
            'P' => Ok(BoardCell::Special(PlayerId::Player)),
            'o' => Ok(BoardCell::Ink(PlayerId::Opponent)),
            'O' => Ok(BoardCell::Special(PlayerId::Opponent)),
            _ => Err(format!("Invalid character for a board cell: '{}'", ch)),
        }
    }

    pub fn is_none(&self) -> bool {
        *self == BoardCell::None
    }

    pub fn is_filled(&self) -> bool {
        !self.is_none()
    }

    pub fn is_wall(&self) -> bool {
        *self == BoardCell::Wall
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoardPosition {
    // We choose `i32` here so that we can describe the position of out side of the board.
    pub x: i32,
    pub y: i32,
}

impl Display for BoardPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{},{}]", self.x, self.y)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    id: u32,
    name: String,
    cells: Vec<Vec<BoardCell>>,

    width: i32,
    height: i32,
}

impl Board {
    pub fn get_scores(&self) -> (i32, i32) {
        let mut player_cnt = 0;
        let mut opponent_cnt = 0;
        let (width, height) = self.get_size();
        for y in 0..height {
            for x in 0..width {
                let position = BoardPosition {
                    x: x as i32,
                    y: y as i32,
                };
                match self.get_cell(position) {
                    BoardCell::Ink(PlayerId::Player) | BoardCell::Special(PlayerId::Player) => {
                        player_cnt += 1;
                    }
                    BoardCell::Ink(PlayerId::Opponent) | BoardCell::Special(PlayerId::Opponent) => {
                        opponent_cnt += 1;
                    }
                    _ => {}
                }
            }
        }
        (player_cnt, opponent_cnt)
    }

    pub fn get_cell(&self, position: BoardPosition) -> BoardCell {
        let x = position.x;
        let y = position.y;
        if x < 0 || y < 0 || y >= self.height || x >= self.width {
            return BoardCell::Wall;
        }
        self.cells[y as usize][x as usize]
    }

    pub fn get_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    pub fn count_surrounded_special_ink(&self) -> (i32, i32) {
        let mut player_cnt = 0;
        let mut opponent_cnt = 0;
        let (width, height) = self.get_size();
        for y in 0..height {
            for x in 0..width {
                let position = BoardPosition {
                    x: x as i32,
                    y: y as i32,
                };
                if let BoardCell::Special(player_id) = self.get_cell(position) {
                    if !self.is_surrounded(&position) {
                        continue;
                    }
                    match player_id {
                        PlayerId::Player => player_cnt += 1,
                        PlayerId::Opponent => opponent_cnt += 1,
                    }
                }
            }
        }
        (player_cnt, opponent_cnt)
    }

    fn is_surrounded(&self, center_position: &BoardPosition) -> bool {
        #[rustfmt::skip]
        const AROUND_DIFF: [(i32, i32); 8] = [
            (-1, -1),  (0, -1),  (1, -1),
            (-1,  0),/*(0,  0),*/(1,  0),
            (-1,  1),  (0,  1),  (1,  1),
        ];
        for diff in AROUND_DIFF {
            let around_pos = BoardPosition {
                x: center_position.x + diff.0,
                y: center_position.y + diff.1,
            };
            if self.get_cell(around_pos).is_none() {
                return false;
            }
        }
        true
    }

    pub fn put_cell(&mut self, position: BoardPosition, cell: BoardCell) {
        let x = position.x;
        let y = position.y;
        assert!(
            y >= 0 || x >= 0 || y < self.height as i32 || x < self.width as i32,
            "Cannot update a cell at out side of the board"
        );
        self.cells[y as usize][x as usize] = cell;
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.id, self.name)?;
        self.cells.iter().for_each(|v| {
            v.iter()
                .for_each(|cell| write!(f, "{}", cell.to_char()).unwrap());
            writeln!(f).unwrap();
        });
        let scores = self.get_scores();
        writeln!(f, "Score: {}, {}", scores.0, scores.1)?;
        Ok(())
    }
}

pub fn load_boards(boards_dir: &str) -> Vec<Board> {
    info!("Start loading board data from: {}", boards_dir);

    let mut boards: Vec<Board> = vec![];
    for entry in fs::read_dir(boards_dir).expect("Couldn't open the board dir") {
        let dir = entry.unwrap();
        let path = dir.path();
        let board = load_board(&path);
        debug!("{}", board);
        boards.push(board);
    }
    boards
}

pub fn load_board(board_path: &PathBuf) -> Board {
    debug!("loading {:?}", board_path);

    let board_id: u32 = board_path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u32>()
        .unwrap_or_else(|_| panic!("Board file name should be a number but {:?}", board_path));

    let file =
        File::open(board_path).unwrap_or_else(|_| panic!("Failed to open: {:?}", board_path));
    let mut reader = BufReader::new(file);
    let mut name: String = String::new();
    reader.read_line(&mut name).unwrap();
    let name = String::from(name.trim());

    let board_lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
    load_board_from_lines(board_id, name, &board_lines)
}

pub fn load_board_from_lines(id: u32, name: String, lines: &[String]) -> Board {
    let cells = read_cells(lines);
    let width: i32 = cells[0].len() as i32;
    let height: i32 = cells.len() as i32;
    Board {
        id,
        name,
        cells,
        width,
        height,
    }
}

fn read_cells(lines: &[String]) -> Vec<Vec<BoardCell>> {
    let mut cells: Vec<Vec<BoardCell>> = vec![];
    for line in lines {
        let cell_line: Vec<BoardCell> = line
            .as_bytes()
            .iter()
            .map(|ch| BoardCell::from_char(*ch as char))
            .collect::<Result<_, _>>()
            .unwrap();
        cells.push(cell_line);
    }

    // Make all rows to have same size.
    let width_max = cells.iter().map(|c| c.len()).max().unwrap();
    cells
        .iter_mut()
        .for_each(|cell_line| cell_line.resize(width_max, BoardCell::Wall));

    cells
}
