use std::sync::Arc;
use std::time::Duration;
use log::*;
use paste::paste;
use rand::seq::SliceRandom;
use rand_mt::Mt64;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;
use tokio::time::timeout;

use crate::engine::board::Board;
use crate::engine::card::Card;
use crate::engine::game::{self, Context};
use crate::engine::state::{PlayerCardState, State, self};
use crate::proto::{self, *};

use super::AContext;
use super::connection::{Connection, Error};

/// An object represents a session of a game
#[derive(Debug)]
pub struct GameSession {
    context: AContext,
    board: Arc<Board>,
    client_south: Arc<Mutex<ClientConnection>>,
    client_north: Arc<Mutex<ClientConnection>>,

    state_south: PlayerCardState,
}

impl GameSession {
    pub fn new(context: AContext, board: Arc<Board>, mut client_south: ClientConnection, mut client_north: ClientConnection, rng: Mt64) -> Self {
        client_south.set_player_id(PlayerId::Sourth);
        client_north.set_player_id(PlayerId::North);
        Self {
            context,
            board,
            client_south: Arc::new(Mutex::new(client_south)),
            client_north: Arc::new(Mutex::new(client_north)),
            state_south: PlayerCardState::new(vec![], vec![]),
        }
    }

    pub async fn start(self: &Self) -> Result<(), Error>{
        info!("New game session is started.");

        let board = self.board.clone();
        let mut south = self.client_south.clone();
        let context = self.context.clone();
        let psh = tokio::spawn(async move {
            Self::init_player(context, board, south).await
        });

        let board = self.board.clone();
        let mut north = self.client_north.clone();
        let context = self.context.clone();
        let pnh = tokio::spawn(async move {
            Self::init_player(context, board, north).await
        });

        let mut north_state: PlayerCardState = match pnh.await {
            Ok(Ok(v)) => v,
            _ => todo!(),
        };
        let mut south_state: PlayerCardState = match psh.await {
            Ok(Ok(v)) => v,
            _ => todo!(),
        };

        info!("Player state: {}, {}", north_state, south_state);
        let mut state = State::new((*self.board).clone(), 0, 0, 0, vec![], vec![]);
        for turn in 0..game::TURN_COUNT {
            /*
            state::update_state(&mut state, &player_action, &opponent_action);
            state::update_player_state(&mut player_state, &player_action);
            state::update_player_state(&mut opponent_state, &opponent_action);
             */
        }
        todo!();
    }

    async fn init_player(context: AContext, board: Arc<Board>, client: Arc<Mutex<ClientConnection>>) -> Result<PlayerCardState, Error> {
        let mut client = client.lock().await;

        let mut deck_ids = Self::get_deck(board, &mut client).await?;
        let state = Self::deal_hands(&context, &mut deck_ids, &mut client).await?;

        Ok(state)
    }

    async fn get_deck(board: Arc<Board>, client: &mut ClientConnection) -> Result<Vec<u32>, Error> {
        client.send_response(&TakoyakiResponse::Manmenmi(
            // TODO: Support multiple types of game with other boards.
            ManmenmiResponse {
                available_games: vec![
                    GameInfo {
                        game_id: 0,
                        board: proto::Board::from(board.as_ref())
                    }
                ],
            }
        )).await?;

        let join_game = client.recv_join_game().await?;
        Ok(join_game.deck)
    }

    async fn deal_hands<'a>(context: &AContext, deck_ids: &mut Vec<u32>, client: &mut ClientConnection)
                            -> Result<PlayerCardState, Error> {
        deck_ids.shuffle(&mut client.rng);

        client.send_response(&TakoyakiResponse::JoinGame(JoinGameResponse {
            player_id: client.player_id,
            initial_hands: deck_ids[0..game::HAND_SIZE].to_vec(),
        })).await?;

        let accept_hands = client.recv_accept_hands().await?;
        if !accept_hands.accept {
            // The client has asked us to re-deal hands.
            deck_ids.shuffle(&mut client.rng);
        }

        let (hand_ids, deck_ids) = deck_ids.split_at(game::HAND_SIZE);

        Ok(PlayerCardState::new(
            context.get_cards(hand_ids),
            context.get_cards(deck_ids),
        ))
    }
}

pub async fn try_establish_connection(stream: TcpStream, client_sender: Sender<ClientConnection>, seed: u64) {
    let mut conn = Connection::new(stream);
    match timeout(Duration::from_secs(10), conn.recv()).await {
        Ok(Ok(TakoyakiRequest::Manmenmi(m))) => {
            conn.set_preferred_format(m.preferred_format);
            let client = ClientConnection::new(
                m.name,
                Mt64::new(seed),
                conn,
            );
            client_sender.send(client).await.unwrap();
        },
        Ok(Ok(_)) => {
            conn.send(
                &TakoyakiResponse::Error(ErrorResponse{
                    code: ErrorCode::BadRequest,
                    message: "Expected request type: SetDeckRequest".into()
                })
            ).await.unwrap_or_default();
        }
        Ok(Err(e)) => {
            conn.send(&TakoyakiResponse::Error(err_to_res(e))).await.unwrap_or_default();
        }
        Err(_elapsed) => {
            conn.send(&TakoyakiResponse::Error(ErrorResponse::new_timeout())).await.unwrap_or_default();
        }
    }
}

#[derive(Debug)]
pub struct ClientConnection {
    pub name: String,
    pub player_id: PlayerId,

    pub rng: Mt64,
    pub connection: Connection,
}

fn err_to_res(e: Error) -> ErrorResponse {
    ErrorResponse{
        code: e.code,
        message: e.message,
    }
}

macro_rules! def_rpc {
    ($root:ty) => {
        paste! {
            async fn [<recv_ $root:snake>](&mut self) -> Result<[<$root Request>], Error> {
                let req : TakoyakiRequest = self.recv_request().await?;
                if let TakoyakiRequest::$root(v) = req {
                    Ok(v)
                } else {
                    Err(Error{
                        code: ErrorCode::BadRequest,
                        message: format!("Expected request type: {} but {:?}", stringify!($root), req)
                    })
                }
            }
        }
    }
}

impl ClientConnection {
    fn new(name: String, rng: Mt64, connection: Connection) ->Self {
        Self {
            name,
            rng,
            connection,
            player_id: PlayerId::North,
        }
    }

    pub fn set_player_id(&mut self, pid: PlayerId) {
        self.player_id = pid;
    }

    pub async fn recv_request(&mut self) -> Result<TakoyakiRequest, Error> {
        self.connection.recv::<TakoyakiRequest>().await
    }

    pub async fn send_response(&mut self, response: &TakoyakiResponse) -> Result<(), Error> {
        self.connection.send::<TakoyakiResponse>(response).await
    }

    def_rpc!(JoinGame);
    def_rpc!(AcceptHands);
}
