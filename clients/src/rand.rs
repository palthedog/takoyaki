extern crate env_logger;
extern crate log;

use takoyaki::{
use players::random::RandomPlayer, client::{Client, common::{self, ClientCommonArgs}}, proto::{Format, GameInfo}};

use clap::{self, Parser};
use log::*;


#[derive(Parser)]
pub struct AppArgs {
    #[clap(flatten)]
    common: ClientCommonArgs,
}

fn main() {
    // Initialize env_logger with a default log level of INFO.
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = AppArgs::parse();
    let (context, deck) = common::init_common(&args.common);

}
