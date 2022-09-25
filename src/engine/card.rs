use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use super::{
    board::{BoardCell, BoardPosition},
    game::{PlayerId, Rotation},
};

use log::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardCellPosition {
    pub x: i32,
    pub y: i32,
}

impl Display for CardCellPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{},{}]", self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardCell {
    pub position: CardCellPosition,
    pub cell_type: CardCellType,

    /// It is used when both players put a cell on an exact same place.
    /// Lower one is prioritized.
    pub priority: i32,
}

impl CardCell {
    pub const PRIORITY_MAX: i32 = 1000;

    /// Calculate cell priority used when both players actions conflict
    /// (a same cell is going to be filled by both players on a same turn)
    ///   - Special block is more prioritized than normal ink.
    ///   - If same ink blocks conflict, a card with lower cost (number of cells) is prioritized.
    /// Note that special attack doesn't affect the priority.
    fn calc_cell_priority(cell_type: &CardCellType, cell_count: i32) -> i32 {
        match cell_type {
            CardCellType::None => panic!(),
            CardCellType::Ink => cell_count + 128,
            CardCellType::Special => cell_count,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardCellType {
    None,
    Ink,
    Special,
}

impl CardCellType {
    fn to_char(self) -> char {
        match self {
            CardCellType::None => ' ',
            CardCellType::Ink => '=',
            CardCellType::Special => '*',
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, CardCellType::None)
    }

    pub fn to_board_cell(&self, player_id: PlayerId) -> BoardCell {
        match self {
            CardCellType::None => BoardCell::None,
            CardCellType::Ink => BoardCell::Ink(player_id),
            CardCellType::Special => BoardCell::Special(player_id),
        }
    }
}

#[derive(Debug)]
pub struct Card {
    id: u32,
    name: String,
    cell_count: i32,
    special_cost: i32,
    cells: HashMap<Rotation, HashMap<CardCellPosition, CardCell>>,
}

impl Card {
    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_special_cost(&self) -> i32 {
        self.special_cost
    }

    pub fn get_cells(&self, rotation: Rotation) -> &HashMap<CardCellPosition, CardCell> {
        self.cells.get(&rotation).unwrap()
    }

    pub fn get_cells_on_board_coord<'a>(
        &'a self,
        card_position: &'a CardPosition,
    ) -> impl Iterator<Item = (BoardPosition, &'a CardCell)> + 'a {
        let cells = self.get_cells(card_position.rotation);
        cells.values().map(|cell| {
            let cell_position = cell.position;
            let board_pos = BoardPosition {
                x: card_position.x + cell_position.x,
                y: card_position.y + cell_position.y,
            };
            (board_pos, cell)
        })
    }

    fn calculate_width(&self, rotation: Rotation) -> i32 {
        self.get_cells(rotation).keys().map(|p| p.x).max().unwrap() + 1
    }

    fn calculate_height(&self, rotation: Rotation) -> i32 {
        self.get_cells(rotation).keys().map(|p| p.y).max().unwrap() + 1
    }
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.id, self.name)?;
        writeln!(f, "cnt: {} cost: {}", self.cell_count, self.special_cost)?;
        let rotation = Rotation::Up;
        let width = self.calculate_width(rotation);
        let height = self.calculate_height(rotation);

        for y in 0..height {
            for x in 0..width {
                let pos = CardCellPosition { x, y };
                let ch = match self.get_cells(rotation).get(&pos) {
                    Some(cell) => cell.cell_type.to_char(),
                    None => ' ',
                };
                write!(f, "{}", ch)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub fn load_cards(cards_dir: &str) -> Vec<Card> {
    info!("Start loading card data from: {}", cards_dir);

    let mut cards: Vec<Card> = vec![];
    for entry in fs::read_dir(cards_dir).expect("Couldn't open the card dir") {
        let dir = entry.unwrap();
        let path = dir.path();
        let path = path.to_str().unwrap();
        let card = load_card(path);
        debug!("{}", card);
        cards.push(card);
    }
    cards
}

pub fn load_card(card_path: &str) -> Card {
    debug!("loading {}", card_path);

    let path = Path::new(card_path);
    let card_id: u32 = path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u32>()
        .unwrap_or_else(|_| panic!("Card file name should be a number but {:?}", path));
    let file = File::open(card_path).unwrap_or_else(|_| panic!("Failed to open: {}", card_path));
    let mut reader = BufReader::new(file);
    // Split the data
    let mut name: String = String::new();
    reader
        .read_line(&mut name)
        .expect("The card data doesn't contain card name");
    let name = String::from(name.trim());

    let mut cell_count: String = String::new();
    reader
        .read_line(&mut cell_count)
        .expect("The card data doesn't contain cell count");
    let cell_count: i32 = cell_count.trim().parse().unwrap_or_else(|e| {
        panic!(
            "Failed to parse the cell count: {}\nGiven string: {}",
            e, cell_count
        )
    });
    let mut special_cost: String = String::new();
    reader
        .read_line(&mut special_cost)
        .expect("Failed to read cost info.");
    let special_cost: i32 = special_cost
        .trim()
        .parse()
        .expect("Failed to parse the special cost");

    let cell_lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
    load_card_from_lines(card_id, name, cell_count, special_cost, &cell_lines)
}

pub fn load_card_from_lines(
    id: u32,
    name: String,
    cell_count: i32,
    special_cost: i32,
    lines: &[String],
) -> Card {
    let cells = read_cells(cell_count, lines);
    assert_eq!(
        cell_count,
        cells.len() as i32,
        "The parsed cell count is different from the one in card data"
    );

    let mut cells_variations: HashMap<Rotation, HashMap<CardCellPosition, CardCell>> =
        HashMap::new();
    for rot in [
        Rotation::Up,
        Rotation::Right,
        Rotation::Down,
        Rotation::Left,
    ]
    .iter()
    {
        let rot_cells = rotate_card_cells(*rot, &cells);
        cells_variations.insert(*rot, convert_to_cell_map(rot_cells));
    }
    assert_eq!(4, cells_variations.len());

    Card {
        id,
        name,
        cell_count: cell_count as i32,
        special_cost,
        cells: cells_variations,
    }
}

fn rotate_card_cells(rotation: Rotation, cells: &[CardCell]) -> Vec<CardCell> {
    cells
        .iter()
        .map(|&c| rotate_card_cell(rotation, c))
        .collect()
}

fn convert_to_cell_map(cells: Vec<CardCell>) -> HashMap<CardCellPosition, CardCell> {
    let mut cell_map: HashMap<CardCellPosition, CardCell> = HashMap::new();
    for cell in cells {
        let old_value = cell_map.insert(cell.position, cell);
        if old_value.is_some() {
            panic!("The card seems to have duplicated cell: {:?}", cell);
        }
    }
    cell_map
}

fn rotate_card_cell(rotation: Rotation, cell: CardCell) -> CardCell {
    let position = cell.position;
    let rotated_pos = match rotation {
        Rotation::Up => position,
        Rotation::Right => CardCellPosition {
            x: -position.y,
            y: position.x,
        },
        Rotation::Down => CardCellPosition {
            x: -position.x,
            y: -position.y,
        },
        Rotation::Left => CardCellPosition {
            x: position.y,
            y: -position.x,
        },
    };

    CardCell {
        position: rotated_pos,
        ..cell
    }
}

fn read_cells(cell_count: i32, lines: &[String]) -> Vec<CardCell> {
    let mut card_cells: Vec<CardCell> = vec![];
    for (y, line) in lines.iter().enumerate() {
        for (x, cell_type) in line
            .as_bytes()
            .iter()
            .map(|ch| match ch {
                b' ' => CardCellType::None,
                b'=' => CardCellType::Ink,
                b'*' => CardCellType::Special,
                _ => panic!("Found an invalid card cell: '{}'", char::from(*ch)),
            })
            .enumerate()
        {
            if cell_type.is_none() {
                continue;
            }
            let position = CardCellPosition {
                x: x as i32,
                y: y as i32,
            };
            let priority = CardCell::calc_cell_priority(&cell_type, cell_count);
            card_cells.push(CardCell {
                position,
                cell_type,
                priority,
            });
        }
    }

    card_cells
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardPosition {
    pub x: i32,
    pub y: i32,
    pub rotation: Rotation,
    pub special: bool,
}

impl Display for CardPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[p: [{},{}], r: {}, s: {}]",
            self.x, self.y, self.rotation, self.special
        )
    }
}
