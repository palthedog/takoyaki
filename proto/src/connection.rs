use log::*;
use serde::{
    Deserialize,
    Serialize,
};
use std::fmt::Display;
use tokio::{
    self,
    io::{
        AsyncBufReadExt,
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpStream,
};

use crate::{
    ErrorCode,
    WireFormat,
};

#[derive(Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug)]
pub struct Connection {
    stream: tokio::io::BufReader<TcpStream>,

    preferred_format: WireFormat,
    buffer: Vec<u8>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        stream.set_nodelay(true).unwrap();
        Self {
            stream: tokio::io::BufReader::new(stream),
            preferred_format: WireFormat::Json,
            buffer: vec![],
        }
    }

    pub fn set_preferred_format(&mut self, format: WireFormat) {
        self.preferred_format = format;
    }

    pub async fn recv<P>(&mut self) -> Result<P, Error>
    where
        P: for<'de> Deserialize<'de>,
    {
        match self.preferred_format {
            WireFormat::Json => self.recv_json().await,
            WireFormat::Flexbuffers => self.recv_flexbuffers().await,
        }
    }

    async fn recv_json<P>(&mut self) -> Result<P, Error>
    where
        P: for<'de> Deserialize<'de>,
    {
        let mut line = String::new();
        if let Err(e) = self.stream.read_line(&mut line).await {
            return Err(Error {
                code: ErrorCode::MalformedPayload, // network error?
                message: e.to_string(),
            });
        }
        debug!("Read line: {}", line.trim_end());
        match serde_json::from_str::<P>(&line) {
            Ok(req) => Ok(req),
            Err(e) => Err(Error {
                code: ErrorCode::MalformedPayload,
                message: e.to_string(),
            }),
        }
    }

    async fn recv_flexbuffers<P>(&mut self) -> Result<P, Error>
    where
        P: for<'de> Deserialize<'de>,
    {
        let size: u32 = match self.stream.read_u32().await {
            Ok(v) => v,
            Err(e) => return Err(Error{
                code: ErrorCode::MalformedPayload,
                message: format!("The first 4 bytes of payload must be a size of following message. This unsigned 32bit integer must be encoded as big-endian: {}", e),
            }),
        };

        if let Err(e) = (&mut self.stream)
            .take(size.into())
            .read_to_end(&mut self.buffer)
            .await
        {
            return Err(Error {
                code: ErrorCode::MalformedPayload,
                message: format!("Failed to read data from the stream: {}", e),
            });
        };

        match flexbuffers::from_slice(&self.buffer) {
            Ok(req) => Ok(req),
            Err(e) => Err(Error {
                code: ErrorCode::MalformedPayload,
                message: format!("Failed to parse flexbuffers: {}", e),
            }),
        }
    }

    pub async fn send<P>(&mut self, response: &P) -> Result<(), Error>
    where
        P: Serialize,
    {
        match self.preferred_format {
            WireFormat::Json => self.send_json(response).await,
            WireFormat::Flexbuffers => self.send_flexbuffers(response).await,
        }
    }

    async fn send_json<P>(&mut self, response: &P) -> Result<(), Error>
    where
        P: Serialize,
    {
        let serialized = match serde_json::to_vec(&response) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error {
                    code: ErrorCode::SerializationFailure,
                    message: format!("{}", e),
                });
            }
        };

        trace!(
            "Send: Serialized data: {:?}",
            String::from_utf8_lossy(&serialized)
        );
        if let Err(_e) = self.stream.write_all(&serialized).await {
            return Err(Error {
                code: ErrorCode::NetworkError,
                message: "Failed to write data into the network stream".into(),
            });
        }

        if let Err(_e) = self.stream.write_u8(b'\n').await {
            return Err(Error {
                code: ErrorCode::NetworkError,
                message: "Failed to write the delimiter into the network stream".into(),
            });
        }
        self.stream.flush().await.unwrap();

        Ok(())
    }

    async fn send_flexbuffers<P>(&mut self, response: &P) -> Result<(), Error>
    where
        P: Serialize,
    {
        let serialized = match flexbuffers::to_vec(&response) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error {
                    code: ErrorCode::SerializationFailure,
                    message: format!("{}", e),
                });
            }
        };

        let size = serialized.len();
        // Write the size first.
        if let Err(_e) = self.stream.write_u32(size as u32).await {
            return Err(Error {
                code: ErrorCode::NetworkError,
                message: "Failed to write the size delimiter into the network stream".into(),
            });
        }
        // Then, the body follows
        if let Err(e) = self.stream.write_all(&serialized).await {
            error!("Failed to send body: {}", e);
            return Err(Error {
                code: ErrorCode::NetworkError,
                message: "Failed to write data into the network stream".into(),
            });
        }
        self.stream.flush().await.unwrap();

        Ok(())
    }
}

impl Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Connection(addr: {:?})",
            self.stream.get_ref().peer_addr()
        )?;
        Ok(())
    }
}
