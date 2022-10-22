use clap::Args;
use log::debug;
use rand_mt::Mt64;
use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
};

use crate::engine::game::Context;

#[derive(Args, Debug)]
pub struct ServerArgs {
    #[clap(long, short, value_parser, default_value_t = 3333)]
    port: u32,
}

#[derive(Debug)]
struct GameSession {
    client_south: Client,
    client_north: Client,

    rng: Mt64,
}

impl GameSession {
    fn new(client_south: Client, client_north: Client, rng: Mt64) -> Self {
        Self {
            client_south,
            client_north,
            rng,
        }
    }

    fn start(&self) {
        debug!("{:#?}", self);
    }
}

#[derive(Debug)]
struct Client {
    stream: TcpStream,
    addr: SocketAddr,
}

impl Client {
    fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        Self { stream, addr }
    }

    fn from(t: (TcpStream, SocketAddr)) -> Self {
        Self::new(t.0, t.1)
    }
}

pub fn run_server(context: &Context, args: ServerArgs) {
    let mut rng = Mt64::from(42);

    let listener = TcpListener::bind(&format!("127.0.0.1:{}", args.port))
        .unwrap_or_else(|err| panic!("Failed to listen on the port: {}\n{}", args.port, err));
    loop {
        let c0 = listener.accept().expect("Couldn't get client.");
        let c1 = listener.accept().expect("Couldn't get client.");
        let seed = rng.next_u64();
        thread::spawn(move || {
            let session = GameSession::new(Client::from(c0), Client::from(c1), Mt64::from(seed));
            session.start();
        });
    }
}
