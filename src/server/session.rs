use std::sync::Arc;
use std::time::Duration;
use log::*;
use rand::seq::SliceRandom;
use rand_mt::Mt64;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;
use tokio::time::timeout;

use crate::engine::board::Board;
use crate::engine::card::Card;
use crate::engine::game::{self, Context};
use crate::engine::state::PlayerCardState;
use crate::proto::{ErrorResponse, TakoyakiResponse, BoardCell, ManmenmiResponse, TakoyakiRequest, ErrorCode, self, GameInfo};

use super::AContext;
use super::connection::{Connection, Error};

/// An object represents a session of a game
#[derive(Debug)]
pub struct GameSession<'c> {
    context: AContext,
    board: Arc<Board>,
    client_south: Arc<Mutex<ClientConnection>>,
    client_north: Arc<Mutex<ClientConnection>>,

    state_south: PlayerCardState<'c>,
}

impl<'c> GameSession<'c> {
    pub fn new(context: AContext, board: Arc<Board>, client_south: ClientConnection, client_north: ClientConnection, rng: Mt64) -> Self {
        Self {
            context,
            board,
            client_south: Arc::new(Mutex::new(client_south)),
            client_north: Arc::new(Mutex::new(client_north)),
            state_south: PlayerCardState::new(vec![], vec![]),
        }
    }

    pub async fn start(self: &'c Self) {
        info!("New game session is started.");

        //let context = self.context.clone();
        let board = self.board.clone();
        let mut south = self.client_south.clone();
        let deck_ids_s = tokio::spawn(async move {
            Self::get_deck(board, south).await
        });

        let deck_s: Vec<&'c Card> = match deck_ids_s.await {
            Ok(Ok(ids)) => {
                let mut v = vec![];
                for id in &ids {
                    v.push(self.context.card_ref(*id));
                }
                v
            },
            Ok(Err(e)) => {
                todo!("{:?}", e);
            }
            Err(e) => {
                todo!("{:?}", e);
            },
        };
    }

    async fn get_deck(board: Arc<Board>, client: Arc<Mutex<ClientConnection>>) -> Result<Vec<u32>, Error> {
        client.lock().await.send_response(&TakoyakiResponse::Manmenmi(
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

        let req : TakoyakiRequest = client.lock().await.recv_request().await?;
        if let TakoyakiRequest::JoinGame(join_game) = req {
            Ok(join_game.deck)
        } else {
            Err(Error{
                code: ErrorCode::BadRequest,
                message: "Expected request type: JoinGameRequest".into()
            })
        }
    }

    async fn deal_hands<'a>(context: &'c AContext, mut deck: Vec<&'c Card>, client: &mut ClientConnection,
                            player_state: &'c mut PlayerCardState<'c>)
                            -> Result<(), Error> {
        deck.shuffle(&mut client.rng);
        *player_state = PlayerCardState::new(
            deck[0..game::HAND_SIZE].to_vec(),
            deck[game::HAND_SIZE..].to_vec(),
        );
        Ok(())
    }
}

pub async fn try_establish_connection(stream: TcpStream, client_sender: Sender<ClientConnection>, seed: u64) {
    let mut conn = Connection::new(stream);
    match timeout(Duration::from_secs(10), conn.recv()).await {
        Ok(Ok(TakoyakiRequest::Manmenmi(m))) => {
            conn.set_preferred_format(m.preferred_format);
            let client = ClientConnection {
                name: m.name,
                rng: Mt64::new(seed),
                connection: conn,
            };
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
    pub rng: Mt64,
    pub connection: Connection,
}

fn err_to_res(e: Error) -> ErrorResponse {
    ErrorResponse{
        code: e.code,
        message: e.message,
    }
}

impl ClientConnection {
    pub async fn recv_request(&mut self) -> Result<TakoyakiRequest, Error> {
        self.connection.recv::<TakoyakiRequest>().await
    }

    pub async fn send_response(&mut self, response: &TakoyakiResponse) -> Result<(), Error> {
        self.connection.send::<TakoyakiResponse>(response).await
    }
}
