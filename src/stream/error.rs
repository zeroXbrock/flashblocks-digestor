use thiserror::Error;

#[derive(Error, Debug)]
pub enum StreamError {
    #[error("Send error: {0}")]
    SendError(String),
}
