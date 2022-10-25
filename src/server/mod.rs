use clap::Args;use log::*;
use rand_mt::Mt64;
use tokio::{
    self, sync::mpsc::{Sender, Receiver, self}};
use tokio::net::TcpListener;
use std::sync::Arc;

use crate::engine::game::Context;

pub mod connection;
mod session;

use session::*;

pub type AContext = Arc<Context>;

#[derive(Args, Debug)]
pub struct ServerArgs {
    #[clap(long, short, value_parser, default_value_t = 3333)]
    port: u32,
}

async fn create_session_loop(context: AContext)-> Sender<ClientConnection> {
    let mut rng = Mt64::from(42);
    let (sender, mut receiver): (Sender<ClientConnection>, Receiver<ClientConnection>) = mpsc::channel(8);
    info!("Create session loop is started");
    tokio::spawn(async move {
        loop {
            let c0 = receiver.recv().await.expect("Server closed while receiving.");
            info!("Client 0 joined: {:?}", c0);
            let c1 = receiver.recv().await.expect("Server closed while receiving.");
            info!("Client 1 joined: {:?}", c1);
            let seed = rng.next_u64();
            let context = context.clone();
            tokio::spawn(async move {
                let session = Arc::new(GameSession::new(context, c0, c1, Mt64::from(seed)));
                session.start().await;
            });
        }
    });
    sender
}

async fn run_server_async(context: &Context, args: ServerArgs) {
    let shared_context = Arc::new(context.clone());
    let listener: TcpListener = TcpListener::bind(
        &format!("127.0.0.1:{}", args.port))
        .await
        .unwrap_or_else(|err| panic!("Failed to listen on the port: {}\n{}", args.port, err));
    info!("Listening at localhost:{}", args.port);

    let client_sender = create_session_loop(shared_context.clone()).await;
    loop {
        debug!("Waiting for a new client.");
        match listener.accept().await{
            Ok((stream, addr)) => {
                let sender = client_sender.clone();
                tokio::spawn(async move {
                    info!("New client is coming from {}", addr);
                    try_establish_connection(stream, sender).await;
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
