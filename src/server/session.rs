use std::sync::Arc;
use std::time::Duration;

use log::*;
use rand_mt::Mt64;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::engine::card::Card;
use crate::engine::game::Context;
use crate::engine::state;
use crate::proto::{ErrorResponse, TakoyakiResponse, Board, BoardCell, ManmenmiResponse, TakoyakiRequest, ErrorCode};

use super::AContext;
use super::connection::Connection;

/// An object represents a session of a game
#[derive(Debug)]
pub struct GameSession {
    context: AContext,
    client_south: Arc<Mutex<Connection>>,
    client_north: Arc<Mutex<Connection>>,

    rng: Mt64,
}

impl GameSession {
    pub fn new(context: AContext, client_south: Connection, client_north: Connection, rng: Mt64) -> Self {
        Self {
            context,
            client_south: Arc::new(Mutex::new(client_south)),
            client_north: Arc::new(Mutex::new(client_north)),
            rng,
        }
    }

    pub async fn start(self: Arc<Self>) {
        info!("New game session is started.");

        let south = self.client_south.clone();
        let north = self.client_north.clone();
        let deck_ids_s = tokio::spawn(async move {
            Self::get_deck(south).await.unwrap()
        });
        let deck_ids_n = tokio::spawn(async move {
            Self::get_deck(north).await.unwrap()
        });

        let deck_s: Vec<&Card> = deck_ids_s.await.unwrap().iter().map(|id| self.context.card_ref(*id)).collect();
        let deck_n: Vec<&Card> = deck_ids_n.await.unwrap().iter().map(|id| self.context.card_ref(*id)).collect();
        info!("Decks:\n{:?}\n{:?}", Card::format_cards(&deck_s), Card::format_cards(&deck_n));

        /*
        let mut player_state = deal_hands(rng, player_deck, player);
        let mut opponent_state = deal_hands(rng, opponent_deck, opponent);
        let mut state = State::new(context.board.clone(), 0, 0, 0, vec![], vec![]);

        loop {
            if state.is_end() {
                break;
            }
            let player_action = player.get_action(&state, player_state.get_hands());
            let opponent_action = opponent.get_action(&state, opponent_state.get_hands());

            state::update_state(&mut state, &player_action, &opponent_action);
            state::update_player_state(&mut player_state, &player_action);
            state::update_player_state(&mut opponent_state, &opponent_action);
        }
        state.board.get_scores()
         */
    }

    //async fn get_deck<'c>(self: Arc<Self>, context: &'c Context, client: &mut Connection) -> Result<Vec<&'c Card>, ErrorResponse> {
    async fn get_deck<'c>(client: Arc<Mutex<Connection>>) -> Result<Vec<u32>, ErrorResponse> {
        let board = Board {
            board_name: "test".into(),
            cells: vec![
                vec![BoardCell::Wall, BoardCell::Wall],
                vec![BoardCell::Wall, BoardCell::Wall],
            ],
        };
        let mut client = client.lock().await;
        client.send_response(&TakoyakiResponse::Manmenmi(
            ManmenmiResponse {
                board
            }
        )).await;

        let req : TakoyakiRequest = client.recv_request().await.unwrap();
        if let TakoyakiRequest::SetDeck(set_deck) = req {
            Ok(set_deck.deck)
        } else {
            Err(ErrorResponse{
                code: ErrorCode::BadRequest,
                message: "Expected request type: SetDeckRequest".into()
            })
        }
    }
}
