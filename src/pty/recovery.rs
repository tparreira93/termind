// PTY Recovery and Error Handling - Phase A Enhancement
// Provides resilient PTY operations with automatic recovery

use crate::pty::{PtyHost, PtyError};
use crate::renderer::TerminalParser;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{warn, error, info, debug};

pub struct ResilientPtyHost {
    pty: Option<PtyHost>,
    parser: TerminalParser,
    retry_config: RetryConfig,
    last_failure: Option<Instant>,
    consecutive_failures: u32,
}

#[derive(Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub failure_threshold: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            failure_threshold: 3,
        }
    }
}

#[derive(Debug)]
pub enum RecoveryAction {
    Retry,
    Recreate,
    Fail,
}

impl ResilientPtyHost {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            pty: None,
            parser: TerminalParser::new(rows, cols),
            retry_config: RetryConfig::default(),
            last_failure: None,
            consecutive_failures: 0,
        }
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Initialize or reinitialize the PTY with automatic retry
    pub async fn ensure_connected(&mut self) -> Result<(), PtyError> {
        if self.pty.is_some() {
            return Ok(());
        }

        let mut attempt = 0;
        let mut delay = self.retry_config.base_delay;

        while attempt < self.retry_config.max_retries {
            match PtyHost::spawn_shell().await {
                Ok(pty) => {
                    self.pty = Some(pty);
                    self.consecutive_failures = 0;
                    info!("PTY successfully initialized on attempt {}", attempt + 1);
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    self.consecutive_failures += 1;
                    warn!("PTY initialization failed (attempt {}): {}", attempt, e);

                    if attempt < self.retry_config.max_retries {
                        info!("Retrying in {:?}...", delay);
                        sleep(delay).await;
                        delay = Duration::from_millis(
                            (delay.as_millis() as f64 * self.retry_config.backoff_multiplier) as u64
                        ).min(self.retry_config.max_delay);
                    } else {
                        error!("Failed to initialize PTY after {} attempts", attempt);
                        return Err(e);
                    }
                }
            }
        }

        Err(PtyError::EnvironmentSetup)
    }

    /// Write with automatic recovery
    pub async fn write_resilient(&mut self, data: &[u8]) -> Result<(), PtyError> {
        let mut attempt = 0;

        while attempt < self.retry_config.max_retries {
            // Ensure we have a connection
            if let Err(e) = self.ensure_connected().await {
                return Err(e);
            }

            if let Some(ref mut pty) = self.pty {
                match pty.write(data).await {
                    Ok(()) => {
                        if attempt > 0 {
                            info!("Write succeeded after {} retries", attempt);
                        }
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Write failed (attempt {}): {}", attempt + 1, e);
                        
                        match self.determine_recovery_action(&e) {
                            RecoveryAction::Retry => {
                                attempt += 1;
                                if attempt < self.retry_config.max_retries {
                                    sleep(self.calculate_delay(attempt)).await;
                                }
                            }
                            RecoveryAction::Recreate => {
                                warn!("Recreating PTY connection due to unrecoverable error");
                                self.pty = None;
                                attempt += 1;
                                if attempt < self.retry_config.max_retries {
                                    sleep(self.calculate_delay(attempt)).await;
                                }
                            }
                            RecoveryAction::Fail => {
                                error!("Unrecoverable write error: {}", e);
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }

        Err(PtyError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Write failed after all retry attempts"
        )))
    }

    /// Read with automatic recovery and buffering
    pub async fn read_resilient(&mut self) -> Result<Vec<u8>, PtyError> {
        if let Err(e) = self.ensure_connected().await {
            return Err(e);
        }

        if let Some(ref mut pty) = self.pty {
            match pty.try_read().await {
                Ok(data) => {
                    if !data.is_empty() {
                        // Process through parser for validation
                        self.parser.parse(&data);
                        debug!("Read {} bytes from PTY", data.len());
                    }
                    Ok(data)
                }
                Err(e) => {
                    warn!("Read error: {}", e);
                    
                    match self.determine_recovery_action(&e) {
                        RecoveryAction::Recreate => {
                            warn!("Recreating PTY connection due to read error");
                            self.pty = None;
                            return Ok(Vec::new()); // Return empty data for this read
                        }
                        _ => return Err(e),
                    }
                }
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Resize with automatic recovery
    pub async fn resize_resilient(&mut self, rows: u16, cols: u16) -> Result<(), PtyError> {
        // Update parser size immediately
        self.parser.resize(rows, cols);

        if let Err(e) = self.ensure_connected().await {
            return Err(e);
        }

        if let Some(ref mut pty) = self.pty {
            match pty.resize(rows, cols) {
                Ok(()) => {
                    info!("PTY resized to {}x{}", rows, cols);
                    Ok(())
                }
                Err(e) => {
                    warn!("Resize failed: {}", e);
                    // Don't recreate PTY for resize failures, just log the error
                    Err(e)
                }
            }
        } else {
            Ok(()) // Parser was resized, PTY will be resized when reconnected
        }
    }

    /// Get the current parser state
    pub fn parser(&self) -> &TerminalParser {
        &self.parser
    }

    /// Get mutable parser access
    pub fn parser_mut(&mut self) -> &mut TerminalParser {
        &mut self.parser
    }

    /// Check if PTY is currently connected
    pub fn is_connected(&self) -> bool {
        self.pty.is_some()
    }

    /// Get connection statistics
    pub fn connection_stats(&self) -> ConnectionStats {
        ConnectionStats {
            is_connected: self.is_connected(),
            consecutive_failures: self.consecutive_failures,
            last_failure: self.last_failure,
        }
    }

    /// Force disconnect (useful for testing recovery)
    pub fn disconnect(&mut self) {
        if self.pty.is_some() {
            info!("Forcing PTY disconnection");
            self.pty = None;
        }
    }

    fn determine_recovery_action(&self, error: &PtyError) -> RecoveryAction {
        match error {
            PtyError::Io(io_err) => {
                match io_err.kind() {
                    std::io::ErrorKind::BrokenPipe |
                    std::io::ErrorKind::ConnectionAborted |
                    std::io::ErrorKind::UnexpectedEof => RecoveryAction::Recreate,
                    
                    std::io::ErrorKind::TimedOut |
                    std::io::ErrorKind::Interrupted |
                    std::io::ErrorKind::WouldBlock => RecoveryAction::Retry,
                    
                    std::io::ErrorKind::PermissionDenied |
                    std::io::ErrorKind::NotFound => RecoveryAction::Fail,
                    
                    _ => {
                        if self.consecutive_failures >= self.retry_config.failure_threshold {
                            RecoveryAction::Recreate
                        } else {
                            RecoveryAction::Retry
                        }
                    }
                }
            }
            PtyError::PtyCreation(_) => RecoveryAction::Retry,
            PtyError::ShellNotFound(_) => RecoveryAction::Fail,
            _ => RecoveryAction::Retry,
        }
    }

    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = (self.retry_config.base_delay.as_millis() as f64 * 
                       self.retry_config.backoff_multiplier.powi(attempt as i32)) as u64;
        Duration::from_millis(delay_ms).min(self.retry_config.max_delay)
    }
}

impl Drop for ResilientPtyHost {
    fn drop(&mut self) {
        if let Some(pty) = self.pty.take() {
            debug!("Dropping resilient PTY host");
            drop(pty);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub is_connected: bool,
    pub consecutive_failures: u32,
    pub last_failure: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_resilient_pty_creation() {
        let mut resilient_pty = ResilientPtyHost::new(24, 80);
        assert!(!resilient_pty.is_connected());
        
        // Test connection
        let result = resilient_pty.ensure_connected().await;
        assert!(result.is_ok(), "Should be able to connect to PTY");
        assert!(resilient_pty.is_connected());
    }
    
    #[tokio::test]
    async fn test_recovery_after_disconnect() {
        let mut resilient_pty = ResilientPtyHost::new(24, 80);
        
        // Connect
        resilient_pty.ensure_connected().await.unwrap();
        assert!(resilient_pty.is_connected());
        
        // Force disconnect
        resilient_pty.disconnect();
        assert!(!resilient_pty.is_connected());
        
        // Should automatically reconnect on next operation
        let result = resilient_pty.write_resilient(b"echo test\n").await;
        assert!(result.is_ok(), "Should reconnect automatically");
        assert!(resilient_pty.is_connected());
    }
}
