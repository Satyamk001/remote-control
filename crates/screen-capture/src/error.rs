use thiserror::Error;

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("Platform error: {0}")]
    PlatformError(String),
    
    #[error("Access Denied (Requires higher privileges)")]
    AccessDenied,
    
    #[error("Timeout waiting for frame")]
    Timeout,
    
    #[error("Display configuration changed")]
    DisplayChanged,
    
    #[error("Initialization failed: {0}")]
    InitFailed(String),
}
