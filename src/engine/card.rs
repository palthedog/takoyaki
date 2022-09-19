use std::{
    fs::{self, File},
    io::{self, BufRead},
    path::Path,
};

#[derive(Clone, Copy, Debug)]
pub enum CardCell {
    None,
    Block,
    Special,
}

impl CardCell {
    fn to_char(&self) -> char {
        match self {
            CardCell::None => ' ',
            CardCell::Block => '=',
            CardCell::Special => '*',
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            CardCell::None => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct Card {
    pub id: u32,
    pub name: String,
    pub cell_count: u32,
    pub special_cost: u32,
    pub cells: Vec<Vec<CardCell>>,
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}\n", self.id, self.name)?;
        write!(f, "cnt: {} cost: {}\n", self.cell_count, self.special_cost)?;
        self.cells.iter().for_each(|v| {
            v.iter()
                .for_each(|cell| write!(f, "{}", cell.to_char()).unwrap());
            write!(f, "\n").unwrap();
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

    let lines = BufRead::lines(io::BufReader::new(file));
    let lines: Vec<String> = lines
        .map(|line| match line {
            Ok(line) => line,
            Err(e) => panic!("Failed to read lines: {}", e),
        })
        .collect();
    println!("lines: {:?}", lines);

    // Split the data
    let name = &lines[0];
    let cell_count: u32 = lines[1]
        .parse()
        .expect("Failed to parse the number of cells.");
    let special_cost: u32 = lines[2].parse().expect("Failed to parse the special cost");
    let (_, cell_lines) = (&lines[..]).split_at(3);

    let cells = read_cells(&cell_lines);

    assert_eq!(cell_count, count_cells(&cells));

    Card {
        id: card_id,
        name: String::from(name),
        cell_count,
        special_cost,
        cells,
    }
}

fn count_cells(cells: &Vec<Vec<CardCell>>) -> u32 {
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
