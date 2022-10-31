use clap::{
    self,
    Parser,
};
use log::*;
use proto::TimeControl;
use rand_mt::Mt64;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        Mutex,
    },
    time::{
        Duration,
        Instant,
    },
};
use tokio::{
    self,
    net::TcpListener,
    sync::mpsc::{
        self,
        Receiver,
        Sender,
    },
};

use engine::{
    Board,
    Context,
};
use server::{
    session::{
        self,
        ClientConnection,
        GameSession,
    },
    stats::StatsCounter,
};

#[derive(Parser)]
pub struct ServerArgs {
    #[clap(long, short, value_parser, default_value_t = 3333)]
    port: u32,

    #[clap(
        long,
        short,
        value_parser,
        default_value = "data/boards/massugu_street"
    )]
    board_path: PathBuf,

    /// a directory path where holds all card data. no need to specify for many cases.
    #[clap(long, value_parser, default_value_t = String::from("data/cards"))]
    card_dir: String,

    /// Specify the time limit in seconds.
    #[clap(long, short, value_parser)]
    time_limit: Option<u32>,
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

async fn create_session_loop(
    context: Arc<Context>,
    board: Board,
    seed: u64,
    args: ServerArgs,
) -> Sender<ClientConnection> {
    let mut rng = Mt64::from(seed);
    let (sender, mut receiver): (Sender<ClientConnection>, Receiver<ClientConnection>) =
        mpsc::channel(8);
    info!("Create session loop is started");
    tokio::spawn(async move {
        let stats_counter = Arc::new(Mutex::new(StatsCounter::new()));
        let print_interval = Arc::new(Mutex::new(Instant::now()));
        loop {
            let c0 = receiver
                .recv()
                .await
                .expect("Server closed while receiving.");
            info!("Client 0 joined: {:?}", c0.name);
            let c1 = receiver
                .recv()
                .await
                .expect("Server closed while receiving.");
            info!("Client 1 joined: {:?}", c1.name);
            let seed = rng.next_u64();
            let board = board.clone();
            let context = context.clone();
            let stats_counter = stats_counter.clone();
            let print_interval = print_interval.clone();
            let time_control = match args.time_limit {
                Some(secs) => TimeControl::PerAction {
                    time_limit_in_seconds: secs,
                },
                None => TimeControl::Infinite,
            };
            tokio::spawn(async move {
                let context = context;
                let board = board;
                let client_south = c0;
                let client_north = c1;
                let rng = Mt64::from(seed);
                let session = Arc::new(GameSession::new(
                    context,
                    Arc::new(board),
                    time_control,
                    client_south,
                    client_north,
                    rng,
                ));
                let result = session.start().await;
                match result {
                    Ok(r) => {
                        let mut sc = stats_counter.lock().unwrap();
                        info!("Result: {} v.s. {}", r.0, r.1);
                        sc.push_result(&r.0, &r.1);

                        let mut print_interval = print_interval.lock().unwrap();

                        // Print once per second at most.
                        if print_interval.elapsed() > Duration::from_secs(1) {
                            info!("{}", sc);
                            *print_interval = Instant::now();
                        }
                    }
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
    let listener: TcpListener = TcpListener::bind(&format!("127.0.0.1:{}", args.port))
        .await
        .unwrap_or_else(|err| panic!("Failed to listen on the port: {}\n{}", args.port, err));
    info!("Listening at localhost:{}", args.port);
    let board = engine::load_board(&args.board_path);

    let client_sender =
        create_session_loop(shared_context.clone(), board, rng.next_u64(), args).await;
    loop {
        debug!("Waiting for a new client.");
        match listener.accept().await {
            Ok((stream, addr)) => {
                let sender = client_sender.clone();
                let seed = rng.next_u64();
                tokio::spawn(async move {
                    info!("New client is coming from {}", addr);
                    session::try_establish_connection(stream, sender, seed).await;
                });
            }
            Err(e) => {
                warn!("Listener is closed: {:?}", e);
                break;
            }
        };
    }
}

pub fn run_server(context: Context, args: ServerArgs) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move { run_server_async(context, args).await });
    info!("Server is exiting...");
}
