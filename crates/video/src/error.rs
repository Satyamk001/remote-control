use thiserror::Error;

#[derive(Error, Debug)]
pub enum VideoError {
    #[error("Encoding failed: {0}")]
    EncodeError(String),
    
    #[error("Decoding failed: {0}")]
    DecodeError(String),
    
    #[error("Packetization error: {0}")]
    PacketizationError(String),
    
    #[error("Rendering error: {0}")]
    RenderError(String),
}
