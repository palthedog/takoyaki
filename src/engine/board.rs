use std::{
    fmt::{Display, Formatter},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use log::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerId {
    Player,
    Opponent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardCell {
    None,
    Wall,
    Ink(PlayerId),
    Special(PlayerId),
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

#[derive(Debug, Copy, Clone)]
pub struct BoardPosition {
    pub x: i32,
    pub y: i32,
}

impl Display for BoardPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{},{}]", self.x, self.y)
    }
}

#[derive(Clone, Debug)]
pub struct Board {
    id: u32,
    name: String,
    cells: Vec<Vec<BoardCell>>,
}

impl Board {
    pub fn get_cell(&self, position: BoardPosition) -> BoardCell {
        let x = position.x as usize;
        let y = position.y as usize;
        if y >= self.cells.len() || x >= self.cells.get(y).unwrap().len() {
            return BoardCell::Wall;
        }
        self.cells[y][x]
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
        Ok(())
    }
}

pub fn load_boards(boards_dir: &str) -> Vec<Board> {
    info!("Start loading board data from: {}", boards_dir);

    let mut boards: Vec<Board> = vec![];
    for entry in fs::read_dir(boards_dir).expect("Couldn't open the board dir") {
        let dir = entry.unwrap();
        let path = dir.path();
        let path = path.to_str().unwrap();

        let board = load_board(path);
        debug!("{}", board);
        boards.push(board);
    }
    boards
}

pub fn load_board(board_path: &str) -> Board {
    debug!("loading {}", board_path);

    let path = Path::new(board_path);
    let board_id: u32 = path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u32>()
        .unwrap_or_else(|_| panic!("Board file name should be a number but {:?}", path));

    let file = File::open(board_path).unwrap_or_else(|_| panic!("Failed to open: {}", board_path));
    let mut reader = BufReader::new(file);
    let mut name: String = String::new();
    reader.read_line(&mut name).unwrap();
    let name = String::from(name.trim());

    let board_lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
    load_board_from_lines(board_id, name, &board_lines)
}

pub fn load_board_from_lines(id: u32, name: String, lines: &[String]) -> Board {
    let cells = read_cells(lines);
    Board { id, name, cells }
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
