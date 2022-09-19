use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

#[derive(Debug)]
pub struct Card {
    pub id: i32,
}

impl Card {
    pub fn new(card_id: i32) -> Card {
        Card { id: card_id }
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
    let card_id: i32 = path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<i32>()
        .unwrap_or_else(|_| panic!("Card file name should be a number but {:?}", path));
    let mut file =
        File::open(card_path).unwrap_or_else(|_| panic!("Failed to open: {}", card_path));
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .unwrap_or_else(|_| panic!("Failed to read {}", card_path));
    println!("content: {}", contents);
    Card::new(card_id)
}
