use std::{
    fmt::{
        Display,
        Formatter,
    },
    fs::{
        self,
        File,
    },
    io::{
        BufRead,
        BufReader,
    },
    path::PathBuf,
};

use log::*;

use super::game::PlayerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
            BoardCell::Ink(PlayerId::South) => 'p',
            BoardCell::Special(PlayerId::South) => 'P',
            BoardCell::Ink(PlayerId::North) => 'o',
            BoardCell::Special(PlayerId::North) => 'O',
        }
    }

    fn from_char(ch: char) -> Result<BoardCell, String> {
        match ch {
            '.' => Ok(BoardCell::None),
            ' ' | '#' => Ok(BoardCell::Wall),
            'p' => Ok(BoardCell::Ink(PlayerId::South)),
            'P' => Ok(BoardCell::Special(PlayerId::South)),
            'o' => Ok(BoardCell::Ink(PlayerId::North)),
            'O' => Ok(BoardCell::Special(PlayerId::North)),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Board {
    name: String,
    cells: Vec<Vec<BoardCell>>,

    width: i32,
    height: i32,

    x_range: Vec<i32>,
    y_range: Vec<i32>,
}

impl Board {
    pub fn new(name: String, cells: Vec<Vec<BoardCell>>) -> Self {
        let width = cells[0].len() as i32;
        let height = cells.len() as i32;
        Self {
            name,
            cells,
            width,
            height,
            x_range: (1..width - 1).collect(),
            y_range: (1..height - 1).collect(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_scores(&self) -> (u32, u32) {
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
                    BoardCell::Ink(PlayerId::South) | BoardCell::Special(PlayerId::South) => {
                        player_cnt += 1;
                    }
                    BoardCell::Ink(PlayerId::North) | BoardCell::Special(PlayerId::North) => {
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

    pub fn get_x_range(&self) -> &[i32] {
        &self.x_range
    }

    pub fn get_y_range(&self) -> &[i32] {
        &self.y_range
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
                        PlayerId::South => player_cnt += 1,
                        PlayerId::North => opponent_cnt += 1,
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
            y >= 0 || x >= 0 || y < self.height || x < self.width,
            "Cannot update a cell at out side of the board"
        );
        self.cells[y as usize][x as usize] = cell;
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "{}", self.name)?;
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

    let file =
        File::open(board_path).unwrap_or_else(|_| panic!("Failed to open: {:?}", board_path));
    let mut reader = BufReader::new(file);
    let mut name: String = String::new();
    reader.read_line(&mut name).unwrap();
    let name = String::from(name.trim());

    let board_lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
    let refs: Vec<&str> = board_lines.iter().map(AsRef::as_ref).collect();
    load_board_from_lines(name, &refs)
}

pub fn load_board_from_lines(name: String, lines: &[&str]) -> Board {
    let cells = read_cells(lines);
    Board::new(name, cells)
}

fn read_cells(lines: &[&str]) -> Vec<Vec<BoardCell>> {
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
