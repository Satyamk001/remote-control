use async_trait::async_trait;
use crate::protocol::ProtocolMessage;
use crate::error::NetworkError;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&mut self, message: &ProtocolMessage) -> Result<(), NetworkError>;
    async fn receive(&mut self) -> Result<ProtocolMessage, NetworkError>;
    async fn close(&mut self) -> Result<(), NetworkError>;
}
