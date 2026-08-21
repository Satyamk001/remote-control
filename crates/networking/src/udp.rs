use async_trait::async_trait;
use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::error::NetworkError;
use crate::protocol::ProtocolMessage;
use crate::transport::Transport;

pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    buffer: Vec<u8>,
}

impl UdpTransport {
    pub fn new(socket: Arc<UdpSocket>, peer_addr: SocketAddr) -> Self {
        Self {
            socket,
            peer_addr,
            buffer: vec![0; 65535], // Max UDP payload
        }
    }
}

#[async_trait]
impl Transport for UdpTransport {
    async fn send(&mut self, message: &ProtocolMessage) -> Result<(), NetworkError> {
        let serialized = bincode::serialize(message)?;
        self.socket.send_to(&serialized, &self.peer_addr).await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<ProtocolMessage, NetworkError> {
        loop {
            let (len, src) = self.socket.recv_from(&mut self.buffer).await?;
            if src == self.peer_addr {
                let msg: ProtocolMessage = bincode::deserialize(&self.buffer[..len])?;
                return Ok(msg);
            }
            // Ignore packets from other addresses in this session
        }
    }

    async fn close(&mut self) -> Result<(), NetworkError> {
        // UDP doesn't have a formal close mechanism
        Ok(())
    }
}
