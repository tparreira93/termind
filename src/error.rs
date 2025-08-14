use thiserror::Error;

#[derive(Error, Debug)]
pub enum TermindError {
    #[error("Shell execution failed: {0}")]
    ShellExecution(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("AI provider error: {0}")]
    AiProvider(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
}

pub type Result<T> = std::result::Result<T, TermindError>;
