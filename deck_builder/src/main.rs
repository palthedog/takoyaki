use clap::Parser;
use deck_builder::DeckBuilderArgs;

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = DeckBuilderArgs::parse();
    deck_builder::train_deck(args);
}
