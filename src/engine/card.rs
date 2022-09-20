use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use super::game::Position;

use log::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardCell {
    pub position: Position,
    pub cell_type: CardCellType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardCellType {
    None,
    Block,
    Special,
}

impl CardCellType {
    fn to_char(self) -> char {
        match self {
            CardCellType::None => ' ',
            CardCellType::Block => '=',
            CardCellType::Special => '*',
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, CardCellType::None)
    }
}

#[derive(Debug, Clone)]
pub struct Card {
    id: u32,
    name: String,
    cell_count: u32,
    special_cost: u32,
    cells: HashMap<Position, CardCell>,
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.id, self.name)?;
        writeln!(f, "cnt: {} cost: {}", self.cell_count, self.special_cost)?;

        let width = self.cells.keys().map(|pos| pos.x).max().unwrap() + 1;
        let height = self.cells.keys().map(|pos| pos.y).max().unwrap() + 1;
        debug!("The displaying card width: {}, height: {}", width, height);
        for y in 0..height {
            for x in 0..width {
                let pos = Position { x, y };
                let ch = match self.cells.get(&pos) {
                    Some(cell) => match cell.cell_type {
                        CardCellType::None => ' ',
                        CardCellType::Block => '=',
                        CardCellType::Special => '*',
                    },
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
    let cell_count: usize = cell_count.trim().parse().unwrap_or_else(|e| {
        panic!(
            "Failed to parse the cell count: {}\nGiven string: {}",
            e, cell_count
        )
    });
    let mut special_cost: String = String::new();
    reader
        .read_line(&mut special_cost)
        .expect("Failed to read cost info.");
    let special_cost: u32 = special_cost
        .trim()
        .parse()
        .expect("Failed to parse the special cost");

    let cell_lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
    let cells = read_cells(&cell_lines);

    assert_eq!(cell_count, cells.len());

    Card {
        id: card_id,
        name,
        cell_count: cell_count as u32,
        special_cost,
        cells,
    }
}

fn read_cells(lines: &[String]) -> HashMap<Position, CardCell> {
    let mut card_cells: HashMap<Position, CardCell> = HashMap::new();
    for (y, line) in lines.iter().enumerate() {
        for (x, cell_type) in line
            .as_bytes()
            .iter()
            .map(|ch| match ch {
                b' ' => CardCellType::None,
                b'=' => CardCellType::Block,
                b'*' => CardCellType::Special,
                _ => panic!("Found an invalid card cell: '{}'", char::from(*ch)),
            })
            .enumerate()
        {
            if cell_type.is_none() {
                continue;
            }
            let position = Position {
                x: x as u32,
                y: y as u32,
            };
            card_cells.insert(
                position,
                CardCell {
                    position,
                    cell_type,
                },
            );
        }
    }

    card_cells
}
