use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec, Decoder, Encoder};
use futures::{SinkExt, StreamExt};

use crate::error::NetworkError;
use crate::protocol::ProtocolMessage;
use crate::transport::Transport;

struct ProtocolCodec {
    length_codec: LengthDelimitedCodec,
}

impl ProtocolCodec {
    pub fn new() -> Self {
        Self {
            length_codec: LengthDelimitedCodec::new(),
        }
    }
}

impl Decoder for ProtocolCodec {
    type Item = ProtocolMessage;
    type Error = NetworkError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(frame) = self.length_codec.decode(src)? {
            let msg: ProtocolMessage = bincode::deserialize(&frame)?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<ProtocolMessage> for ProtocolCodec {
    type Error = NetworkError;

    fn encode(&mut self, item: ProtocolMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let serialized = bincode::serialize(&item)?;
        let mut frame_buf = BytesMut::new();
        frame_buf.put_slice(&serialized);
        self.length_codec.encode(frame_buf.freeze(), dst)?;
        Ok(())
    }
}

pub struct TcpTransport {
    framed: Framed<TcpStream, ProtocolCodec>,
}

impl TcpTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            framed: Framed::new(stream, ProtocolCodec::new()),
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, message: &ProtocolMessage) -> Result<(), NetworkError> {
        self.framed.send(message.clone()).await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<ProtocolMessage, NetworkError> {
        match self.framed.next().await {
            Some(Ok(msg)) => Ok(msg),
            Some(Err(e)) => Err(e),
            None => Err(NetworkError::ConnectionClosed),
        }
    }

    async fn close(&mut self) -> Result<(), NetworkError> {
        let _ = self.framed.close().await;
        Ok(())
    }
}
