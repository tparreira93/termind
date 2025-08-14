// Block-based data model for Termind (Phase A foundation)
// This will store command blocks with SQLite in Phase A Week 3

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub cwd: String,
    pub shell: String,
    pub command: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub stdout: String,
    pub stderr: String,
    pub tags: Vec<String>,
}

impl Block {
    pub fn new(command: String, cwd: String, shell: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            cwd,
            shell,
            command,
            args: Vec::new(),
            exit_code: None,
            duration_ms: None,
            stdout: String::new(),
            stderr: String::new(),
            tags: Vec::new(),
        }
    }
    
    pub fn with_output(mut self, stdout: String, stderr: String) -> Self {
        self.stdout = stdout;
        self.stderr = stderr;
        self
    }
    
    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self
    }
    
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
    
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

// Block detector for identifying command boundaries in terminal output
use crate::error::Result;

pub struct BlockDetector {
    store: BlockStore,
    current_block: Option<Block>,
}

impl BlockDetector {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            store: BlockStore::new().await?,
            current_block: None,
        })
    }
    
    pub fn start_command(&mut self, command: String, cwd: String, shell: String) {
        self.current_block = Some(Block::new(command, cwd, shell));
    }
    
    pub fn add_output(&mut self, output: &str, is_stderr: bool) {
        if let Some(ref mut block) = self.current_block {
            if is_stderr {
                block.stderr.push_str(output);
            } else {
                block.stdout.push_str(output);
            }
        }
    }
    
    pub async fn finish_command(&mut self, exit_code: i32, duration_ms: u64) -> Result<()> {
        if let Some(block) = self.current_block.take() {
            let finished_block = block
                .with_exit_code(exit_code)
                .with_duration(duration_ms);
            
            self.store.store(finished_block).await?;
        }
        Ok(())
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<Block>> {
        self.store.search(query).await
    }
}

// Block storage with SQLite backend (Phase A Week 3)
pub struct BlockStore;

impl BlockStore {
    pub async fn new() -> Result<Self> {
        // TODO: Initialize SQLite database
        Ok(Self)
    }
    
    pub async fn store(&self, _block: Block) -> Result<()> {
        // TODO: Implement SQLite storage
        Ok(())
    }
    
    pub async fn search(&self, _query: &str) -> Result<Vec<Block>> {
        // TODO: Implement FTS search
        Ok(Vec::new())
    }
}
