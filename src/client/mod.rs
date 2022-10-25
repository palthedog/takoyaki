use tokio::net::TcpStream;

use crate::{players::Player, proto::GameResult};

pub struct Client<P>
where P: Player {
    player: P,
}

impl<P> Client<P>
where P: Player {
    pub fn new(player: P) -> Self {
        Self {
            player,
        }
    }

    pub async fn connect(host: &str) -> Result<GameResult, String> {
        let stream = TcpStream::connect(host).await;
        let _stream = match stream {
            Ok(v) => v,
            Err(e) => {
                return Err(format!("Connection failed: {}", e));
            },
        };

        //stream.
        Ok(GameResult{
            score: 0,
            opponent_score: 0,
        })
    }
}
