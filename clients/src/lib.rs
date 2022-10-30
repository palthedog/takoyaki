use std::{
    fmt::{
        Display,
        Formatter,
    },
    sync::Arc,
};

use paste::paste;
use tokio::net::TcpStream;

use log::*;

use players::Player;
use proto::{
    connection::Connection,
    *,
};

use engine::{
    Card,
    Context,
    State,
};

pub type GamePickerFn = Box<dyn Fn(&[GameInfo]) -> (GameId, Vec<Card>)>;

pub struct GameResult {
    pub my_score: u32,
    pub opponent_score: u32,
}

impl Display for GameResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GameResult[")?;
        match self.my_score.cmp(&self.opponent_score) {
            std::cmp::Ordering::Less => write!(f, "LOSE")?,
            std::cmp::Ordering::Equal => write!(f, "DRAW")?,
            std::cmp::Ordering::Greater => write!(f, "WIN")?,
        };
        write!(f, " ({}, {})]", self.my_score, self.opponent_score)?;

        Ok(())
    }
}

pub struct Client<P: Player> {
    context: Arc<Context>,
    preferred_format: WireFormat,
    player: P,
    player_id: PlayerId,
    game_picker: GamePickerFn,
}

struct Session<'p, P: Player> {
    client: &'p mut Client<P>,
    connection: Connection,
}

impl<P: Player> Client<P> {
    pub fn new(
        context: Context,
        preferred_format: WireFormat,
        player: P,
        game_picker: GamePickerFn,
    ) -> Self {
        Self {
            context: Arc::new(context),
            preferred_format,
            player,
            player_id: PlayerId::North,
            game_picker,
        }
    }

    pub fn start(&mut self, host: &str) -> Result<GameResult, String> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut session = self.join_game_async(host).await?;
            let result = session.start().await?;
            Ok(match self.player_id {
                PlayerId::Sourth => GameResult {
                    my_score: result.south_score,
                    opponent_score: result.north_score,
                },
                PlayerId::North => GameResult {
                    my_score: result.north_score,
                    opponent_score: result.south_score,
                },
            })
        })
    }

    async fn join_game_async<'p>(&'p mut self, host: &str) -> Result<Session<'p, P>, String> {
        let stream = TcpStream::connect(host).await;
        let stream = match stream {
            Ok(v) => v,
            Err(e) => {
                return Err(format!("Connection failed: {}", e));
            }
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

                // TODO: Fix me... it's sad to set the format here.
                // The client send our preferred format.
                // We can use our preferred one from next message.
                // Note that we must set the format before start receiving a next message
                // since the server will sent next message encoded as preferred one.
                self.connection.set_preferred_format(self.client.preferred_format);

                let res: [<$root Response>] = match self.connection.recv().await {
                    Ok(TakoyakiResponse::$root(v)) => v,
                    Ok(v) => {
                        error!("Unexpected message: {:?}", v);
                        return Err(format!("Recv unexpected message: Expected {} but: {:?}", stringify!($root), v));
                    },
                    Err(e) => {
                        error!("Network error: {:?}", e);
                        return Err(format!("Recv RPC error: {:?}", e));
                    },
                };
                Ok(res)
            }
        }
    }
}

impl<'p, P: Player> Session<'p, P> {
    async fn start(&mut self) -> Result<proto::Scores, String> {
        let mut game_list = self.manmenmi().await?;
        let (game_id, deck) = (*self.client.game_picker)(&game_list);
        let join_game = self
            .send_join_game(JoinGameRequest {
                game_id,
                deck: engine::to_ids(&deck),
            })
            .await?;
        self.client.player_id = join_game.player_id;

        // TODO: We know our server supports only one game for now...
        assert_eq!(1, game_list.len());
        let game_info = game_list.remove(0);

        self.client
            .player
            .init_game(self.client.player_id.into(), &self.client.context, deck);

        let hands = self.client.context.get_cards(&join_game.initial_hands);
        info!("Initial Hand dealed: {}", engine::format_cards(&hands));
        let need_redeal = self.client.player.need_redeal_hands(&hands);
        let accept_hands_res = self
            .send_accept_hands(AcceptHandsRequest {
                accept: !need_redeal,
            })
            .await?;

        let mut state = State::new(game_info.board.into(), 0, 0, 0, vec![], vec![]);
        let mut hands = self.client.context.get_cards(&accept_hands_res.hands);

        loop {
            let action = self.client.player.get_action(&state, &hands);
            let res = self
                .send_select_action(SelectActionRequest {
                    action: action.clone().into(),
                })
                .await?;
            let opponent_action = res.opponent_action.convert(&self.client.context);
            hands = self.client.context.get_cards(&res.hands);

            let (action_s, action_n) = match self.client.player_id {
                PlayerId::Sourth => (action, opponent_action),
                PlayerId::North => (opponent_action, action),
            };

            engine::update_state(&mut state, &action_s, &action_n);
            info!("State updated: {}", state);
            info!("Act-South: {}", action_s);
            info!("Act-North: {}", action_n);
            info!("Player ID: {:?}", self.client.player_id);

            if let Some(result) = res.game_result {
                return Ok(result);
            }
        }
    }

    async fn manmenmi(&mut self) -> Result<Vec<GameInfo>, String> {
        let res = self
            .send_manmenmi(ManmenmiRequest {
                name: self.client.player.get_name().into(),
                preferred_format: self.client.preferred_format,
            })
            .await;

        let res = match res {
            Ok(v) => v,
            Err(e) => {
                return Err(format!("Got error at Manmenmi: {}", e));
            }
        };
        Ok(res.available_games)
    }

    // Following macros generate methods named line `send_manmenmi` or `send_join_game`
    def_rpc!(Manmenmi);
    def_rpc!(JoinGame);
    def_rpc!(AcceptHands);
    def_rpc!(SelectAction);
}
