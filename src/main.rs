use takoyaki::engine::card;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(about)]
struct Args {
    #[clap(long, value_parser, default_value_t = String::from("data/cards"))]
    card_dir: String,
}

fn main() {
    let args = Args::parse();
    let all_cards = card::load_cards(&args.card_dir);

    all_cards.iter().for_each(|c| println!("{:?}\n{}", c, c));
}
