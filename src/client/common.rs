use std::path::PathBuf;

use clap::{Args, ValueHint};

use crate::engine::{game::Context, card::{self, Card}};

#[derive(Args)]
pub struct ClientCommonArgs {
    /// a directory path where holds all card data. no need to specify for many cases.
    #[clap(long, value_parser, default_value = "data/cards")]
    pub card_dir: String,

    /// A file path which is a list of cards the player use for the game.
    #[clap(
        short,
        long,
        value_parser,
        value_hint=ValueHint::FilePath,
        default_value = "data/decks/starter"
    )]
    pub deck_path: PathBuf,

    #[clap(long, short, value_parser, default_value = "localhost:3333")]
    pub server: String,
}

pub fn init_common(common_args: &ClientCommonArgs) -> (Context, Vec<Card>) {
    let all_cards = card::load_cards(&common_args.card_dir);
    let context = Context {
        all_cards,
        enabled_step_execution: false,
    };
    let deck_ids = card::load_deck(&common_args.deck_path);
    let deck: Vec<Card> = context.get_cards(&deck_ids);

    (context, deck)
}
