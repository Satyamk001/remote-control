use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization Error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Connection Closed")]
    ConnectionClosed,

    #[error("Timeout")]
    Timeout,

    #[error("Invalid Message Format")]
    InvalidFormat,
}
