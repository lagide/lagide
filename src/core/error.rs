use thiserror::Error;

#[derive(Error, Debug)]
pub enum LagideError {
    #[error("SSH error: {0}")]
    Ssh(String),

    #[error("WSL error: {0}")]
    Wsl(String),

    #[error("SFTP error: {0}")]
    Sftp(String),

    #[error("Tunnel error: {0}")]
    Tunnel(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),
}

pub type Result<T> = std::result::Result<T, LagideError>;
