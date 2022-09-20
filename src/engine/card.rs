use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

#[derive(Clone, Copy, Debug)]
pub enum CardCell {
    None,
    Block,
    Special,
}

impl CardCell {
    fn to_char(self) -> char {
        match self {
            CardCell::None => ' ',
            CardCell::Block => '=',
            CardCell::Special => '*',
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, CardCell::None)
    }
}

#[derive(Debug)]
pub struct Card {
    id: u32,
    name: String,
    cell_count: u32,
    special_cost: u32,
    cells: Vec<Vec<CardCell>>,
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.id, self.name)?;
        writeln!(f, "cnt: {} cost: {}", self.cell_count, self.special_cost)?;
        self.cells.iter().for_each(|v| {
            v.iter()
                .for_each(|cell| write!(f, "{}", cell.to_char()).unwrap());
            writeln!(f).unwrap();
        });
        Ok(())
    }
}

pub fn load_cards(cards_dir: &str) -> Vec<Card> {
    let mut cards: Vec<Card> = vec![];
    for entry in fs::read_dir(cards_dir).expect("Couldn't open the card dir") {
        let dir = entry.unwrap();
        let path = dir.path();
        let path = path.to_str().unwrap();
        cards.push(load_card(path));
    }
    cards
}

pub fn load_card(card_path: &str) -> Card {
    println!("loading {}", card_path);
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
    let cell_count: u32 = cell_count.trim().parse().unwrap_or_else(|e| {
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

    assert_eq!(cell_count, count_cells(&cells));

    Card {
        id: card_id,
        name,
        cell_count,
        special_cost,
        cells,
    }
}

fn count_cells(cells: &[Vec<CardCell>]) -> u32 {
    cells
        .iter()
        .map(|line| line.iter().filter(|&c| !c.is_none()).count() as u32)
        .sum()
}

fn read_cells(lines: &[String]) -> Vec<Vec<CardCell>> {
    let mut card_cells: Vec<Vec<CardCell>> = vec![];
    for line in lines {
        let cell_line: Vec<CardCell> = line
            .as_bytes()
            .iter()
            .map(|ch| match ch {
                b' ' => CardCell::None,
                b'=' => CardCell::Block,
                b'*' => CardCell::Special,
                _ => panic!("Found an invalid card cell: '{}'", char::from(*ch)),
            })
            .collect();
        card_cells.push(cell_line);
    }

    // Make all rows to have same size.
    let width_max = card_cells.iter().map(|c| c.len()).max().unwrap();
    card_cells
        .iter_mut()
        .for_each(|cell_line| cell_line.resize(width_max, CardCell::None));

    card_cells
}
