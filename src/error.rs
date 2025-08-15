use thiserror::Error;

#[derive(Error, Debug)]
pub enum TermindError {
    #[error("Shell execution failed: {0}")]
    ShellExecution(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("PTY error: {0}")]
    Pty(String),
    
    #[error("AI provider error: {0}")]
    AiProvider(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("DateTime parsing error: {0}")]
    DateTime(#[from] chrono::ParseError),
    
}

pub type Result<T> = std::result::Result<T, TermindError>;
