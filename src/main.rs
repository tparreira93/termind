//! Termind - Phase A Week 3 Entry Point
//! 
//! Clean architecture with only Phase A components:
//! - PTY Host for real terminal spawning
//! - Terminal Parser for ANSI/VT100 sequences  
//! - Text Grid for screen state
//! - Block Detection for command boundaries

use clap::Parser;
use tokio::time::{sleep, Duration};
use tracing::{info, error};

// Use termind library components
use termind::{
    Result,
    TextGrid, TerminalParser,
    BlockDetector,
};

#[derive(Parser)]
#[command(name = "termind", version = "0.3.0", author, about = "Privacy-first, AI-powered terminal")]
struct Cli {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Terminal width (default: 80)
    #[arg(short = 'w', long, default_value = "80")]
    width: u16,
    
    /// Terminal height (default: 24)
    #[arg(short = 't', long, default_value = "24")]
    height: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    info!("🚀 Starting Termind v0.3.0 - Phase A Week 3");
    
    // Run the terminal application
    let result = run_terminal(&cli).await;
    
    match result {
        Ok(()) => {
            info!("✅ Termind terminated successfully");
            Ok(())
        }
        Err(e) => {
            error!("❌ Termind terminated with error: {}", e);
            Err(e)
        }
    }
}

async fn run_terminal(cli: &Cli) -> Result<()> {
    info!("📋 Initializing Phase A components...");
    
    // Initialize core components
    let _text_grid = TextGrid::new(cli.height, cli.width);
    let _parser = TerminalParser::new(cli.height, cli.width);
    let _block_detector = BlockDetector::new().await?;
    
    info!("🔧 Components initialized successfully");
    info!("📏 Terminal size: {}x{}", cli.width, cli.height);
    
    // Main event loop stub - will be implemented with full PTY integration
    info!("🔄 Starting main event loop (stub)...");
    
    // For now, just run for a few seconds to demonstrate
    for i in 1..=3 {
        info!("📊 Status check #{}: All Phase A components active", i);
        sleep(Duration::from_secs(1)).await;
    }
    
    info!("🧹 Phase A Week 3 preparation complete");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_main_components() {
        // Test that we can create all Phase A components
        let text_grid = TextGrid::new(24, 80);
        assert_eq!(text_grid.rows, 24);
        assert_eq!(text_grid.cols, 80);
        
        let _parser = TerminalParser::new(24, 80);
        // Parser creation should always succeed
        
        let block_detector = BlockDetector::new().await;
        assert!(block_detector.is_ok());
    }
}
