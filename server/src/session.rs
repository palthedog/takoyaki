use log::*;
use paste::paste;
use rand::seq::SliceRandom;
use rand_mt::Mt64;
use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::Sender,
        Mutex,
    },
    time::timeout,
};

use engine::{
    self,
    Board,
    Context,
    PlayerCardState,
    State,
};
use proto::{
    self,
    connection::{
        Connection,
        Error,
    },
    *,
};

use crate::stats::NamedScore;

/// An object represents a session of a game
#[derive(Debug)]
pub struct GameSession {
    context: Arc<Context>,
    board: Arc<Board>,
    time_control: TimeControl,
    client_south: Arc<Mutex<ClientConnection>>,
    client_north: Arc<Mutex<ClientConnection>>,
}

impl GameSession {
    pub fn new(
        context: Arc<Context>,
        board: Arc<Board>,
        time_control: TimeControl,
        mut client_south: ClientConnection,
        mut client_north: ClientConnection,
        _rng: Mt64,
    ) -> Self {
        client_south.set_player_id(PlayerId::Sourth);
        client_north.set_player_id(PlayerId::North);
        Self {
            context,
            board,
            time_control,
            client_south: Arc::new(Mutex::new(client_south)),
            client_north: Arc::new(Mutex::new(client_north)),
        }
    }

    pub async fn start(&self) -> Result<(NamedScore, NamedScore), Error> {
        info!("New game session is started.");

        let board = self.board.clone();
        let time_control = self.time_control.clone();
        let south = self.client_south.clone();
        let ctx = self.context.clone();
        let h_ps =
            tokio::spawn(async move { Self::init_player(ctx, board, time_control, south).await });

        let board = self.board.clone();
        let time_control = self.time_control.clone();
        let north = self.client_north.clone();
        let ctx = self.context.clone();
        let h_pn =
            tokio::spawn(async move { Self::init_player(ctx, board, time_control, north).await });

        let mut north_state: PlayerCardState = match h_pn.await {
            Ok(Ok(v)) => v,
            _ => todo!(),
        };
        let mut south_state: PlayerCardState = match h_ps.await {
            Ok(Ok(v)) => v,
            _ => todo!(),
        };

        let t_start_game = Instant::now();

        let state = Arc::new(Mutex::new(State::new(
            (*self.board).clone(),
            0,
            0,
            0,
            vec![],
            vec![],
        )));
        for turn in 0..engine::TURN_COUNT {
            debug!(
                "Turn {}, Player state: {}, {}",
                turn, north_state, south_state
            );

            let south = self.client_south.clone();
            let action_s = tokio::spawn(async move { Self::get_action(south).await });
            let north = self.client_north.clone();
            let action_n = tokio::spawn(async move { Self::get_action(north).await });

            let action_s = action_s.await.unwrap()?;
            let action_n = action_n.await.unwrap()?;
            debug!("action_s: {:?}", action_s);
            debug!("action_n: {:?}", action_n);

            let south_action = action_s.convert(&self.context);
            let north_action = action_n.convert(&self.context);
            {
                let mut state = state.lock().await;
                engine::update_state(&mut state, &south_action, &north_action);
                engine::update_player_state(&state, &mut south_state, &south_action);
                engine::update_player_state(&state, &mut north_state, &north_action);
            }

            let state_s = state.clone();
            let south = self.client_south.clone();
            let hands = engine::to_ids(south_state.get_hands());
            let opponent_action = action_n;
            let send_result_s = tokio::spawn(async move {
                Self::send_result(&opponent_action, hands, state_s, south).await
            });
            let state_n = state.clone();
            let north = self.client_north.clone();
            let hands = engine::to_ids(north_state.get_hands());
            let opponent_action = action_s;
            let send_result_n = tokio::spawn(async move {
                Self::send_result(&opponent_action, hands, state_n, north).await
            });

            send_result_s.await.unwrap().unwrap();
            send_result_n.await.unwrap().unwrap();

            let st = state.lock().await;
            if st.is_end() {
                info!("Elapsed time: {:?}", t_start_game.elapsed());
                let scores = st.board.get_scores();
                return Ok((
                    NamedScore::new(&self.client_south.lock().await.name, scores.0),
                    NamedScore::new(&self.client_north.lock().await.name, scores.1),
                ));
            }
        }
        panic!();
    }

    async fn init_player(
        context: Arc<Context>,
        board: Arc<Board>,
        time_control: TimeControl,
        client: Arc<Mutex<ClientConnection>>,
    ) -> Result<PlayerCardState, Error> {
        let mut client = client.lock().await;

        let mut deck_ids = Self::get_deck(board, time_control, &mut client).await?;
        let state = Self::deal_hands(&context, &mut deck_ids, &mut client).await?;
        Ok(state)
    }

    async fn get_deck(
        board: Arc<Board>,
        time_control: TimeControl,
        client: &mut ClientConnection,
    ) -> Result<Vec<u32>, Error> {
        client
            .send_response(&TakoyakiResponse::Manmenmi(
                // TODO: Support multiple types of game with other boards.
                ManmenmiResponse {
                    available_games: vec![GameInfo {
                        game_id: 0,
                        time_control,
                        board: proto::Board::from(board.as_ref()),
                    }],
                },
            ))
            .await?;

        let join_game = client.recv_join_game().await?;
        Ok(join_game.deck)
    }

    async fn deal_hands<'a>(
        context: &Arc<Context>,
        deck_ids: &mut Vec<u32>,
        client: &mut ClientConnection,
    ) -> Result<PlayerCardState, Error> {
        deck_ids.shuffle(&mut client.rng);

        client
            .send_response(&TakoyakiResponse::JoinGame(JoinGameResponse {
                player_id: client.player_id,
                initial_hands: deck_ids[0..engine::HAND_SIZE].to_vec(),
            }))
            .await?;

        let accept_hands = client.recv_accept_hands().await?;
        if !accept_hands.accept {
            // The client has asked us to re-deal hands.
            deck_ids.shuffle(&mut client.rng);
        }

        let (hand_ids, deck_ids) = deck_ids.split_at(engine::HAND_SIZE);
        client
            .send_response(&TakoyakiResponse::AcceptHands(AcceptHandsResponse {
                hands: hand_ids.to_vec(),
            }))
            .await?;

        Ok(PlayerCardState::new(
            client.player_id.into(),
            context.get_cards(hand_ids),
            context.get_cards(deck_ids),
        ))
    }

    async fn get_action(client: Arc<Mutex<ClientConnection>>) -> Result<Action, Error> {
        let mut client = client.lock().await;
        let select = client.recv_select_action().await?;
        Ok(select.action)
    }

    async fn send_result(
        opponent_action: &Action,
        hands: Vec<CardId>,
        state: Arc<Mutex<State>>,
        client: Arc<Mutex<ClientConnection>>,
    ) -> Result<(), Error> {
        let mut client = client.lock().await;
        let game_result = {
            let state = state.lock().await;
            if state.is_end() {
                let (s, n) = state.board.get_scores();
                Some(Scores {
                    south_score: s,
                    north_score: n,
                })
            } else {
                None
            }
        };
        let res = SelectActionResponse {
            opponent_action: *opponent_action,
            hands,
            game_result,
        };
        client
            .send_response(&TakoyakiResponse::SelectAction(res))
            .await?;
        Ok(())
    }
}

pub async fn try_establish_connection(
    stream: TcpStream,
    client_sender: Sender<ClientConnection>,
    seed: u64,
) {
    let mut conn = Connection::new(stream);
    match timeout(Duration::from_secs(10), conn.recv()).await {
        Ok(Ok(TakoyakiRequest::Manmenmi(m))) => {
            conn.set_preferred_format(m.preferred_format);
            let client = ClientConnection::new(m.name, Mt64::new(seed), conn);
            client_sender.send(client).await.unwrap();
        }
        Ok(Ok(_)) => {
            conn.send(&TakoyakiResponse::Error(ErrorResponse {
                code: ErrorCode::BadRequest,
                message: "Expected request type: SetDeckRequest".into(),
            }))
            .await
            .unwrap_or_default();
        }
        Ok(Err(e)) => {
            conn.send(&TakoyakiResponse::Error(err_to_res(e)))
                .await
                .unwrap_or_default();
        }
        Err(_elapsed) => {
            conn.send(&TakoyakiResponse::Error(ErrorResponse::new_timeout()))
                .await
                .unwrap_or_default();
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
    ErrorResponse {
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
    fn new(name: String, rng: Mt64, connection: Connection) -> Self {
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
    def_rpc!(SelectAction);
}
