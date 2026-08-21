use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebRtcError {
    #[error("WebRTC SDK error: {0}")]
    SdkError(String),
    
    #[error("Signaling error: {0}")]
    SignalingError(String),
    
    #[error("Connection failed")]
    ConnectionFailed,
}
