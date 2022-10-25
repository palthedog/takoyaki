use clap::Args;use log::*;
use rand_mt::Mt64;
use tokio::{
    self, sync::mpsc::{Sender, Receiver, self}};
use tokio::net::TcpListener;
use std::{sync::Arc, path::PathBuf};

use crate::engine::{game::Context, board::{Board, load_board}};

pub mod connection;
mod session;

use session::*;

pub type AContext = Arc<Context>;

#[derive(Args, Debug)]
pub struct ServerArgs {
    #[clap(long, short, value_parser, default_value_t = 3333)]
    port: u32,

    #[clap(long, short, value_parser, default_value = "data/boards/massugu_street")]
    board_path: PathBuf
}

async fn create_session_loop(context: AContext, board: Board, seed: u64)-> Sender<ClientConnection> {
    let mut rng = Mt64::from(seed);
    let (sender, mut receiver): (Sender<ClientConnection>, Receiver<ClientConnection>) = mpsc::channel(8);
    info!("Create session loop is started");
    tokio::spawn(async move {
        loop {
            let c0 = receiver.recv().await.expect("Server closed while receiving.");
            info!("Client 0 joined: {:?}", c0);
            let c1 = receiver.recv().await.expect("Server closed while receiving.");
            info!("Client 1 joined: {:?}", c1);
            let seed = rng.next_u64();
            let board = board.clone();
            let context = context.clone();
            tokio::spawn(async move {
                let context = context;
                let board = board;
                let client_south = c0;
                let client_north = c1;
                let rng = Mt64::from(seed);
                let session = Arc::new(GameSession::new(
                    context,
                    Arc::new(board),
                    client_south,
                    client_north,
                    rng,
                ));
                session.start().await;
            });
        }
    });
    sender
}

async fn run_server_async(context: &Context, args: ServerArgs) {
    let mut rng = Mt64::from(42);
    let shared_context = Arc::new(context.clone());
    let listener: TcpListener = TcpListener::bind(
        &format!("127.0.0.1:{}", args.port))
        .await
        .unwrap_or_else(|err| panic!("Failed to listen on the port: {}\n{}", args.port, err));
    info!("Listening at localhost:{}", args.port);
    let board = load_board(&args.board_path);

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
                    info!("Client is disconnected {}", addr);
                });
            }
            Err(e) => {
                warn!("Listener is closed: {:?}", e);
                break;
            },
        };
    }
}

pub fn run_server(context: &Context, args: ServerArgs) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move {
        run_server_async(context, args).await
    });
    info!("Server is exiting...");
}
