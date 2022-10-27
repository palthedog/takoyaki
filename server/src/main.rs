use clap::{self, Parser};
use log::*;

use rand_mt::Mt64;
use tokio::{
    self, sync::mpsc::{Sender, Receiver, self}};
use tokio::net::TcpListener;
use std::{sync::Arc, path::PathBuf};

use engine::{Context, Board};

mod session;
use session::*;

pub type AContext = Arc<Context>;

#[derive(Parser)]
pub struct ServerArgs {
    #[clap(long, short, value_parser, default_value_t = 3333)]
    port: u32,

    #[clap(long, short, value_parser, default_value = "data/boards/massugu_street")]
    board_path: PathBuf,

    /// a directory path where holds all card data. no need to specify for many cases.
    #[clap(long, value_parser, default_value_t = String::from("data/cards"))]
    card_dir: String,
}

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let args = ServerArgs::parse();

    let all_cards = engine::load_cards(&args.card_dir);
    let context = Context {
        all_cards,
        enabled_step_execution: false,
    };
    run_server(context, args);
}

async fn create_session_loop(context: AContext, board: Board, seed: u64)-> Sender<ClientConnection> {
    let mut rng = Mt64::from(seed);
    let (sender, mut receiver): (Sender<ClientConnection>, Receiver<ClientConnection>) = mpsc::channel(8);
    info!("Create session loop is started");
    tokio::spawn(async move {
        loop {
            let c0 = receiver.recv().await.expect("Server closed while receiving.");
            info!("Client 0 joined: {:?}", c0.name);
            let c1 = receiver.recv().await.expect("Server closed while receiving.");
            info!("Client 1 joined: {:?}", c1.name);
            let seed = rng.next_u64();
            let board = board.clone();
            let context = context.clone();
            tokio::spawn(async move {
                let context = context;
                let board = board;
                let client_south = c0;
                let client_north = c1;
                let south_name = client_south.name.clone();
                let north_name = client_north.name.clone();
                let rng = Mt64::from(seed);
                let session = Arc::new(GameSession::new(
                    context,
                    Arc::new(board),
                    client_south,
                    client_north,
                    rng,
                ));
                let result = session.start().await;
                match result {
                    Ok(r) => {
                        info!("Result: {}({}) v.s. {}({})",
                              south_name, r.south_score,
                              north_name, r.north_score
                        );

                    },
                    Err(_) => todo!(),
                }
            });
        }
    });
    sender
}

async fn run_server_async(context: Context, args: ServerArgs) {
    let mut rng = Mt64::from(42);
    let shared_context = Arc::new(context.clone());
    let listener: TcpListener = TcpListener::bind(
        &format!("127.0.0.1:{}", args.port))
        .await
        .unwrap_or_else(|err| panic!("Failed to listen on the port: {}\n{}", args.port, err));
    info!("Listening at localhost:{}", args.port);
    let board = engine::load_board(&args.board_path);

    let client_sender = create_session_loop(shared_context.clone(), board, rng.next_u64()).await;
    loop {
        debug!("Waiting for a new client.");
        match listener.accept().await{
            Ok((stream, addr)) => {
                let sender = client_sender.clone();
                let seed = rng.next_u64();
                tokio::spawn(async move {
                    info!("New client is coming from {}", addr);
                    try_establish_connection(stream, sender, seed).await;
                });
            }
            Err(e) => {
                warn!("Listener is closed: {:?}", e);
                break;
            },
        };
    }
}

pub fn run_server(context: Context, args: ServerArgs) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move {
        run_server_async(context, args).await
    });
    info!("Server is exiting...");
}
