use log::*;
use tokio::{
    self,
    io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt},
    sync::mpsc::Sender, time::timeout
};
use tokio::net::TcpStream;
use std::{
    io::Error, time::Duration, fmt::Display,
};

use crate::proto::*;

#[derive(Debug)]
pub struct Connection {
    stream: tokio::io::BufReader<TcpStream>,

    preferred_format: Format,
    name: String,

    buffer: Vec<u8>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: tokio::io::BufReader::new(stream),
            preferred_format: Format::Json,
            name: "<new comer>".into(),
            buffer: vec![],
        }
    }

    fn init(&mut self, manmenmi: ManmenmiRequest) {
        self.name = manmenmi.name;
        self.preferred_format = manmenmi.preferred_format;
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub async fn recv_request(&mut self) -> Result<TakoyakiRequest, ErrorResponse> {
        match self.preferred_format {
            Format::Json => self.recv_json_request().await,
            Format::Flexbuffers => self.recv_flexbuffers_request().await,
        }
    }

    pub async fn send_response(&mut self, response: &TakoyakiResponse) -> Result<(), Error> {
        let f = match self.preferred_format {
            Format::Json => self.send_json_response(response),
            _ => todo!(),
        };

        f.await
    }

    pub async fn try_establish_connection(stream: TcpStream, client_sender: Sender<Connection>) {
        let mut client = Connection::new(stream);
        match timeout(Duration::from_secs(10), client.recv_request()).await {
            Ok(Ok(TakoyakiRequest::Manmenmi(m))) => {
                client.init(m);
                client_sender.send(client).await.unwrap();
            },
            Ok(Ok(_)) => {
                client.send_response(
                    &TakoyakiResponse::Error(ErrorResponse{
                    code: ErrorCode::BadRequest,
                    message: "Expected request type: SetDeckRequest".into()
                    })
                ).await.unwrap_or_default();
            }
            Ok(Err(err_res)) => {
                client.send_response(&TakoyakiResponse::Error(err_res)).await.unwrap_or_default();
            }
            Err(_elapsed) => {
                client.send_response(&TakoyakiResponse::Error(ErrorResponse::new_timeout())).await.unwrap_or_default();
            }
        }
    }

    async fn recv_json_request(&mut self) -> Result<TakoyakiRequest, ErrorResponse> {
        let mut line = String::new();
        if let Err(e) = self.stream
            .read_line(&mut line)
            .await {
                return Err(ErrorResponse {
                    code: ErrorCode::MalformedPayload,  // network error?
                    message: e.to_string(),
                });
            }
        info!("Read line: {}", line.trim_end());
        match serde_json::from_str::<TakoyakiRequest>(&line) {
            Ok(req) => Ok(req),
            Err(e) =>  Err(ErrorResponse {
                code: ErrorCode::MalformedPayload,
                message: e.to_string(),
            }),
        }
    }

    async fn recv_flexbuffers_request(&mut self) -> Result<TakoyakiRequest, ErrorResponse> {
        let size: u32 = match self.stream.read_u32().await {
            Ok(v) => v,
            Err(e) => return Err(ErrorResponse{
                code: ErrorCode::MalformedPayload,
                message: format!("The first 4 bytes of payload must be a size of following message. This unsigned 32bit integer must be encoded as big-endian: {}", e.to_string()),
            }),
        };
        if let Err(e) = self.stream.get_mut().take(size.into()).read_to_end(&mut self.buffer).await {
            return Err(ErrorResponse{
                code: ErrorCode::MalformedPayload,
                message: format!("Malformed body: {}", e.to_string()),
            });
        };

        match flexbuffers::from_slice(&self.buffer) {
            Ok(req) => Ok(req),
            Err(e) => Err(ErrorResponse{
                code: ErrorCode::MalformedPayload,
                message: format!("Malformed body: {}", e.to_string()),
            }),
        }
    }

    async fn send_json_response(&mut self, response: &TakoyakiResponse) -> Result<(), Error> {
        let serialized = serde_json::to_vec(&response)?;
        trace!("Serialized data: {:?}", serialized);
        self.stream.get_mut().write_all(&serialized).await?;
        self.stream.get_mut().write_u8(b'\n').await?;
        Ok(())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        info!("Client is going to be dropped: {:?}", self.stream.get_mut().peer_addr());
    }
}

impl Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Connection(name: {}, addr: {:?})", self.name, self.stream.get_ref().peer_addr())?;
        Ok(())
    }
}
