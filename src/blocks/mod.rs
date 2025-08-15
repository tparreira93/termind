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
    
    pub async fn get_recent(&self, limit: i32) -> Result<Vec<Block>> {
        self.store.get_recent(limit).await
    }
    
    pub async fn get_failed(&self, limit: i32) -> Result<Vec<Block>> {
        self.store.get_failed(limit).await
    }
    
    pub fn current_block(&self) -> Option<&Block> {
        self.current_block.as_ref()
    }
}

// Block storage with SQLite backend (Phase A Week 3)
use sqlx::{sqlite::{SqlitePool, SqliteRow}, Pool, Sqlite, Row};
use std::path::PathBuf;

pub struct BlockStore {
    pool: Pool<Sqlite>,
}

impl BlockStore {
    pub async fn new() -> Result<Self> {
        let db_path = Self::get_database_path()?;
        
        // Ensure the directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let database_url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&database_url).await?;
        
        let store = Self { pool };
        store.initialize_schema().await?;
        
        Ok(store)
    }
    
    fn get_database_path() -> Result<PathBuf> {
        let mut path = dirs::data_dir()
            .ok_or_else(|| crate::error::TermindError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine data directory"
            )))?;
        path.push("termind");
        path.push("blocks.db");
        Ok(path)
    }
    
    async fn initialize_schema(&self) -> Result<()> {
        // Create the main blocks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS blocks (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                cwd TEXT NOT NULL,
                shell TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT NOT NULL, -- JSON array
                exit_code INTEGER,
                duration_ms INTEGER,
                stdout TEXT NOT NULL,
                stderr TEXT NOT NULL,
                tags TEXT NOT NULL  -- JSON array
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create FTS virtual table for full-text search
        sqlx::query(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS blocks_fts USING fts5(
                id UNINDEXED,
                command,
                stdout,
                stderr,
                tags,
                content='blocks',
                content_rowid='rowid'
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create triggers to keep FTS table in sync
        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS blocks_ai AFTER INSERT ON blocks BEGIN
              INSERT INTO blocks_fts(rowid, id, command, stdout, stderr, tags)
              VALUES (new.rowid, new.id, new.command, new.stdout, new.stderr, new.tags);
            END
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS blocks_ad AFTER DELETE ON blocks BEGIN
              INSERT INTO blocks_fts(blocks_fts, rowid, id, command, stdout, stderr, tags)
              VALUES('delete', old.rowid, old.id, old.command, old.stdout, old.stderr, old.tags);
            END
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS blocks_au AFTER UPDATE ON blocks BEGIN
              INSERT INTO blocks_fts(blocks_fts, rowid, id, command, stdout, stderr, tags)
              VALUES('delete', old.rowid, old.id, old.command, old.stdout, old.stderr, old.tags);
              INSERT INTO blocks_fts(rowid, id, command, stdout, stderr, tags)
              VALUES (new.rowid, new.id, new.command, new.stdout, new.stderr, new.tags);
            END
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn store(&self, block: Block) -> Result<()> {
        let args_json = serde_json::to_string(&block.args)?;
        let tags_json = serde_json::to_string(&block.tags)?;
        
        sqlx::query(
            r#"
            INSERT INTO blocks (
                id, timestamp, cwd, shell, command, args,
                exit_code, duration_ms, stdout, stderr, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&block.id)
        .bind(block.timestamp.to_rfc3339())
        .bind(&block.cwd)
        .bind(&block.shell)
        .bind(&block.command)
        .bind(args_json)
        .bind(block.exit_code)
        .bind(block.duration_ms.map(|d| d as i64))
        .bind(&block.stdout)
        .bind(&block.stderr)
        .bind(tags_json)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<Block>> {
        let rows = sqlx::query(
            r#"
            SELECT b.id, b.timestamp, b.cwd, b.shell, b.command, b.args,
                   b.exit_code, b.duration_ms, b.stdout, b.stderr, b.tags
            FROM blocks_fts fts
            JOIN blocks b ON b.rowid = fts.rowid
            WHERE blocks_fts MATCH ?
            ORDER BY rank
            LIMIT 50
            "#,
        )
        .bind(query)
        .fetch_all(&self.pool)
        .await?;
        
        let mut blocks = Vec::new();
        for row in rows {
            let block = Self::row_to_block(&row)?;
            blocks.push(block);
        }
        
        Ok(blocks)
    }
    
    pub async fn get_recent(&self, limit: i32) -> Result<Vec<Block>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, cwd, shell, command, args,
                   exit_code, duration_ms, stdout, stderr, tags
            FROM blocks
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut blocks = Vec::new();
        for row in rows {
            let block = Self::row_to_block(&row)?;
            blocks.push(block);
        }
        
        Ok(blocks)
    }
    
    pub async fn get_failed(&self, limit: i32) -> Result<Vec<Block>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, cwd, shell, command, args,
                   exit_code, duration_ms, stdout, stderr, tags
            FROM blocks
            WHERE exit_code IS NOT NULL AND exit_code != 0
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut blocks = Vec::new();
        for row in rows {
            let block = Self::row_to_block(&row)?;
            blocks.push(block);
        }
        
        Ok(blocks)
    }
    
    fn row_to_block(row: &SqliteRow) -> Result<Block> {
        let args_json: String = row.try_get("args")?;
        let tags_json: String = row.try_get("tags")?;
        let timestamp_str: String = row.try_get("timestamp")?;
        
        let args: Vec<String> = serde_json::from_str(&args_json)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json)?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)?
            .with_timezone(&chrono::Utc);
        
        Ok(Block {
            id: row.try_get("id")?,
            timestamp,
            cwd: row.try_get("cwd")?,
            shell: row.try_get("shell")?,
            command: row.try_get("command")?,
            args,
            exit_code: row.try_get("exit_code")?,
            duration_ms: row.try_get::<Option<i64>, _>("duration_ms")?.map(|d| d as u64),
            stdout: row.try_get("stdout")?,
            stderr: row.try_get("stderr")?,
            tags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_block_creation() {
        let block = Block::new(
            "ls -la".to_string(),
            "/home/user".to_string(),
            "bash".to_string(),
        );

        assert!(!block.id.is_empty());
        assert_eq!(block.command, "ls -la");
        assert_eq!(block.cwd, "/home/user");
        assert_eq!(block.shell, "bash");
        assert_eq!(block.exit_code, None);
        assert_eq!(block.duration_ms, None);
    }

    #[tokio::test]
    async fn test_block_with_output() {
        let block = Block::new(
            "echo test".to_string(),
            "/home/user".to_string(),
            "bash".to_string(),
        )
        .with_output("test\n".to_string(), "".to_string())
        .with_exit_code(0)
        .with_duration(100);

        assert_eq!(block.stdout, "test\n");
        assert_eq!(block.stderr, "");
        assert_eq!(block.exit_code, Some(0));
        assert_eq!(block.duration_ms, Some(100));
        assert!(block.success());
    }

    #[tokio::test]
    async fn test_block_store_creation() -> Result<()> {
        let _store = BlockStore::new().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_store_and_retrieve_block() -> Result<()> {
        let store = BlockStore::new().await?;
        
        let block = Block::new(
            "pwd".to_string(),
            "/home/user".to_string(),
            "zsh".to_string(),
        )
        .with_output("/home/user\n".to_string(), "".to_string())
        .with_exit_code(0)
        .with_duration(50);

        // Store the block
        store.store(block.clone()).await?;

        // Retrieve recent blocks
        let recent = store.get_recent(10).await?;
        assert!(!recent.is_empty());
        
        let retrieved = &recent[0];
        assert_eq!(retrieved.command, block.command);
        assert_eq!(retrieved.cwd, block.cwd);
        assert_eq!(retrieved.shell, block.shell);
        assert_eq!(retrieved.stdout, block.stdout);
        assert_eq!(retrieved.exit_code, block.exit_code);

        Ok(())
    }

    #[tokio::test]
    async fn test_block_detector() -> Result<()> {
        let mut detector = BlockDetector::new().await?;
        
        // Start a command
        detector.start_command(
            "cat file.txt".to_string(),
            "/home/user".to_string(),
            "bash".to_string(),
        );

        assert!(detector.current_block().is_some());
        
        // Add some output
        detector.add_output("line 1\n", false);
        detector.add_output("line 2\n", false);
        detector.add_output("warning: deprecated\n", true);
        
        let current = detector.current_block().unwrap();
        assert!(current.stdout.contains("line 1"));
        assert!(current.stdout.contains("line 2"));
        assert!(current.stderr.contains("warning"));

        // Finish the command
        detector.finish_command(0, 250).await?;
        assert!(detector.current_block().is_none());

        // Verify it was stored
        let recent = detector.get_recent(1).await?;
        assert!(!recent.is_empty());
        assert_eq!(recent[0].command, "cat file.txt");
        assert_eq!(recent[0].exit_code, Some(0));
        assert_eq!(recent[0].duration_ms, Some(250));

        Ok(())
    }
}
