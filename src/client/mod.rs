use log::info;
use tokio::net::TcpStream;

use crate::{
    players::Player, proto::*, engine::game::Context, server::connection::Connection,
};

pub struct Client<'c, P: Player<'c>> {
    context: &'c Context,
    preferred_format: Format,
    player: P,
}

struct Session<'p, 'c: 'p, P: Player<'c>> {
    client: &'p Client<'c, P>,
    connection: Connection,
}

impl<'c,  P: Player<'c>> Client<'c, P> {
    pub fn new(context: &'c Context, preferred_format: Format, player: P) -> Self {
        Self {
            context,
            preferred_format,
            player,
        }
    }

    pub fn join_game(&self, host: &str) -> Result<GameResult, String> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            self.join_game_async(host).await
        })
    }

    pub async fn join_game_async(&self, host: &str) -> Result<GameResult, String> {
        let stream = TcpStream::connect(host).await;
        let stream = match stream {
            Ok(v) => v,
            Err(e) => {
                return Err(format!("Connection failed: {}", e));
            },
        };

        // Clients ALWAYS use Json format for the first message.
        let mut session = Session {
            client: self,
            connection: Connection::new(stream),
        };
        session.start().await
    }
}

impl <'p, 'c, P: Player<'c>> Session<'p, 'c, P> {
    async fn start(&mut self) -> Result<GameResult, String> {
        let res = self.send_manmenmi(
            ManmenmiRequest {
                name: "ika".into(),
                preferred_format: self.client.preferred_format,
            }).await;
        let res = match res {
            Ok(v) => v,
            Err(e) => {
                return Err(format!("Got error at Manmenmi: {}", e));
            },
        };
        // The client send our preferred format.
        // We can use our preferred one from next message.
        self.connection.set_preferred_format(self.client.preferred_format);

        info!("{:?}", res);

        todo!()
    }

    async fn send_manmenmi(&mut self, req: ManmenmiRequest) -> Result<ManmenmiResponse, String> {
        if let Err(e) = self.connection.send(&TakoyakiRequest::Manmenmi(req)).await {
            return Err(format!("Send RPC error: {:?}", e));
        }
        let res: ManmenmiResponse = match self.connection.recv().await {
            Ok(TakoyakiResponse::Manmenmi(v)) => v,
            Ok(v) => {
                return Err(format!("Recv unexpected message: Expected Manmenmi but: {:?}", v));
            },
            Err(e) => {
                return Err(format!("Recv RPC error: {:?}", e));
            },
        };
        Ok(res)
    }
}
