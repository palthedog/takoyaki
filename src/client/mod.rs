use log::info;
use tokio::net::TcpStream;

use paste::paste;

use crate::{
    players::Player, proto::*, engine::{game::Context, card::{Card, self}}, server::{connection::Connection, AContext},
};

pub struct Client<P: Player> {
    context: AContext,
    preferred_format: Format,
    player: P,
    game_picker: Box<dyn Fn(&[GameInfo]) -> (GameId, Vec<Card>)>,
}

struct Session<'p, P: Player> {
    client: &'p mut Client<P>,
    connection: Connection,
}

impl<P: Player> Client<P> {
    pub fn new(context: AContext, preferred_format: Format, player: P,
               game_picker: Box<dyn Fn(&[GameInfo]) -> (GameId, Vec<Card>)>
    )-> Self {
        Self {
            context,
            preferred_format,
            player,
            game_picker,
        }
    }

    pub fn join_game(&mut self, host: &str) -> Result<GameResult, String> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut session =  self.join_game_async(host).await?;
            session.start().await
        })
    }

    pub async fn join_game_async<'p>(&'p mut self, host: &str) -> Result<Session<'p, P>, String> {
        let stream = TcpStream::connect(host).await;
        let stream = match stream {
            Ok(v) => v,
            Err(e) => {
                return Err(format!("Connection failed: {}", e));
            },
        };
        Ok(Session {
            client: self,
            connection: Connection::new(stream),
        })
    }
}

macro_rules! def_rpc {
    ($root:ty) => {
        paste! {
            async fn [<send_ $root:snake>](&mut self, req: [<$root Request>]) -> Result<[<$root Response>], String> {
                if let Err(e) = self.connection.send(&TakoyakiRequest::$root(req)).await {
                    return Err(format!("Send RPC error: {:?}", e));
                }
                let res: [<$root Response>] = match self.connection.recv().await {
                    Ok(TakoyakiResponse::$root(v)) => v,
                    Ok(v) => {
                        return Err(format!("Recv unexpected message: Expected {} but: {:?}", stringify!($root), v));
                    },
                    Err(e) => {
                        return Err(format!("Recv RPC error: {:?}", e));
                    },
                };
                Ok(res)
            }
        }
    }
}

impl <'p, P: Player> Session<'p, P> {
    async fn start(&mut self) -> Result<GameResult, String> {
        let game_list = self.manmenmi().await?;
        let (game_id, deck) = (*self.client.game_picker)(&game_list);
        let join_game = self.send_join_game(JoinGameRequest {
            game_id,
            deck: card::to_ids(&deck),
        }).await?;

        let hands = self.client.context.get_cards(&join_game.initial_hands);
        let need_redeal = self.client.player.need_redeal_hands(&hands);
        let accept_hands_res = self.send_accept_hands(AcceptHandsRequest { accept: !need_redeal }).await?;
        
        todo!("Handle accept hands");
    }

    async fn manmenmi(&mut self) -> Result<Vec<GameInfo>, String>{
        let res = self.send_manmenmi(
            ManmenmiRequest {
                name: self.client.player.get_name().into(),
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
        Ok(res.available_games)
    }

    // Following macros generate methods named line `send_manmenmi` or `send_join_game`
    def_rpc!(Manmenmi);
    def_rpc!(JoinGame);
    def_rpc!(AcceptHands);
}
