use clap::Args;
use log::*;
use rand_mt::Mt64;
use std::{
    io::{BufRead, BufReader, Read},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
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
    stream: BufReader<TcpStream>,

    preferred_format: Format,
    name: String,
}

impl Client {
    fn new(stream: BufReader<TcpStream>, manmenmi: ManmenmiRequest) -> Self {
        Self {
            stream,
            preferred_format: manmenmi.preferred_format,
            name: manmenmi.name,
        }
    }

    fn try_establish_connection(mut stream: TcpStream, sender: Sender<Client>) {
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        let read = stream
            .read_line(&mut line)
            .expect("Failed read data from the stream");
        info!("Read line: {}", line.trim_end());
        let manmenmi = serde_json::from_str::<TakoyakiRequest>(&line);
        match manmenmi {
            Ok(TakoyakiRequest::Manmenmi(m)) => {
                info!("{:?}", m);
                let client = Client::new(stream, m);
                sender.send(client).unwrap();
            }
            Err(err) => error!("The client sent a invalid message: {}", line),
        }
    }
}

pub fn run_server(context: &Context, args: ServerArgs) {
    let mut rng = Mt64::from(42);

    let shared_context = Arc::new(context.clone());
    let (sender, mut receiver): (Sender<Client>, Receiver<Client>) = mpsc::channel();

    let create_session_handler = thread::spawn(move || loop {
        let c0 = receiver.recv().expect("Server closed while receiving.");
        let c1 = receiver.recv().expect("Server closed while receiving.");
        let seed = rng.next_u64();
        let context = shared_context.clone();
        thread::spawn(move || {
            let mut session = GameSession::new(context, c0, c1, Mt64::from(seed));
            session.start();
        });
    });

    let listener = TcpListener::bind(&format!("127.0.0.1:{}", args.port))
        .unwrap_or_else(|err| panic!("Failed to listen on the port: {}\n{}", args.port, err));
    info!("Listening at localhost:{}", args.port);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                info!("A new client is connected: {}", stream.peer_addr().unwrap());
                Client::try_establish_connection(stream, sender.clone());
            }
            Err(e) => {
                warn!("Connection failed");
            }
        }
    }
    info!("Listening port has closed.");
}
