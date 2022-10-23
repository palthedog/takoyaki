use clap::Args;use log::*;
use rand_mt::Mt64;
use tokio::{
    self,
    net::{TcpListener, TcpStream}, io::AsyncBufReadExt, sync::mpsc::{Sender, Receiver, self}};
use std::{
    sync::Arc,
    thread, io::Error,
};

use crate::{
    engine::game::Context,
    proto::{Format, ManmenmiRequest, TakoyakiRequest},
};

type AContext = Arc<Context>;

#[derive(Args, Debug)]
pub struct ServerArgs {
    #[clap(long, short, value_parser, default_value_t = 3333)]
    port: u32,
}

#[derive(Debug)]
struct GameSession {
    context: AContext,
    client_south: Client,
    client_north: Client,

    rng: Mt64,
}

impl GameSession {
    fn new(context: AContext, client_south: Client, client_north: Client, rng: Mt64) -> Self {
        Self {
            context,
            client_south,
            client_north,
            rng,
        }
    }

    fn start(&mut self) {
        info!("New game session is started.");
    }
}

#[derive(Debug)]
struct Client {
    stream: tokio::io::BufReader<TcpStream>,

    preferred_format: Format,
    name: String,
}

impl Client {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream: tokio::io::BufReader::new(stream),
            preferred_format: Format::Json,
            name: "<new comer>".into(),
        }
    }

    fn init(&mut self, manmenmi: ManmenmiRequest) {
        self.name = manmenmi.name;
        self.preferred_format = manmenmi.preferred_format;
    }

    async fn recv_json_request(&mut self) -> Result<TakoyakiRequest, Error> {
        let mut line = String::new();
        self.stream
            .read_line(&mut line)
            .await?;
        info!("Read line: {}", line.trim_end());
        Ok(serde_json::from_str::<TakoyakiRequest>(&line)?)
    }

    async fn recv_request(&mut self) -> Result<TakoyakiRequest, Error> {
        match self.preferred_format {
            Format::Json => self.recv_json_request().await,
            Format::FlexBuffer => todo!(),
        }
    }

    async fn try_establish_connection(stream: TcpStream, sender: Sender<Client>) {
        let mut client = Client::new(stream);
        match client.recv_request().await {
            Ok(TakoyakiRequest::Manmenmi(m)) => {
                client.init(m);
                sender.send(client).await.unwrap();
            },
            Ok(_) => todo!("Bad request"),
            Err(e) => warn!("Client sent an invalid message: {}", e),
        }
    }
}

async fn create_session_loop(context: AContext)-> Sender<Client> {
    let mut rng = Mt64::from(42);
    let (sender, mut receiver): (Sender<Client>, Receiver<Client>) = mpsc::channel(8);
    info!("Create session loop is started");
    tokio::spawn(async move {
        loop {
            let c0 = receiver.recv().await.expect("Server closed while receiving.");
            info!("Client 0 joined: {}", c0.name);
            let c1 = receiver.recv().await.expect("Server closed while receiving.");
            info!("Client 1 joined: {}", c1.name);
            let seed = rng.next_u64();
            let context = context.clone();
            thread::spawn(move || {
                let mut session = GameSession::new(context, c0, c1, Mt64::from(seed));
                session.start();
            });
        }
    });
    sender
}

async fn run_server_async(context: &Context, args: ServerArgs) {
    let shared_context = Arc::new(context.clone());
    let sender = create_session_loop(shared_context.clone()).await;

    let listener: TcpListener = TcpListener::bind(
        &format!("127.0.0.1:{}", args.port))
        .await
        .unwrap_or_else(|err| panic!("Failed to listen on the port: {}\n{}", args.port, err));
    info!("Listening at localhost:{}", args.port);

    loop {
        debug!("Waiting for a new client.");
        match listener.accept().await{
            Ok((stream, addr)) => {
                let sender = sender.clone();
                tokio::spawn(async move {
                    info!("New client is coming from {}", addr);
                    Client::try_establish_connection(stream, sender).await;
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
