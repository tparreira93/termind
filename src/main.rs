//! Termind - Phase A Week 3 Entry Point
//! 
//! Clean architecture with only Phase A components:
//! - PTY Host for real terminal spawning
//! - Terminal Parser for ANSI/VT100 sequences  
//! - Text Grid for screen state
//! - Block Detection for command boundaries

use clap::Parser;
use tokio::time::{sleep, Duration};
use tracing::{info, error, warn};
use std::sync::Arc;
use tokio::sync::Mutex;

use winit::{
    event::{Event, WindowEvent, KeyEvent, ElementState},
    event_loop::{EventLoop, ControlFlow},
    window::WindowBuilder,
    keyboard::{KeyCode, PhysicalKey},
};


// Use termind library components
use termind::{
    Result,
    TextGrid, TerminalParser,
    BlockDetector, PtyHost,
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
    let text_grid = TextGrid::new(cli.height, cli.width);
    let parser = TerminalParser::new(cli.height, cli.width);
    let _block_detector = BlockDetector::new().await?;
    
    info!("🔧 Components initialized successfully");
    info!("📏 Terminal size: {}x{}", cli.width, cli.height);
    
    // Spawn the shell with PTY
    info!("🐚 Spawning shell...");
    let mut pty_host = PtyHost::spawn_shell().await
        .map_err(|e| termind::TermindError::Pty(format!("Failed to spawn shell: {}", e)))?;
    
    info!("✅ Shell spawned successfully: {}", pty_host.shell_path());
    
    // Set up terminal size (non-fatal if it fails)
    if let Err(e) = pty_host.resize(cli.height, cli.width) {
        info!("⚠️  Could not resize PTY (continuing anyway): {}", e);
    }
    
    // Wrap components in Arc<Mutex<>> for sharing between async tasks and GUI
    let pty_host = Arc::new(Mutex::new(pty_host));
    let parser = Arc::new(Mutex::new(parser));
    let text_grid = Arc::new(Mutex::new(text_grid));
    
    // Start GUI window
    info!("🪟 Opening terminal window...");
    run_gui_terminal(cli, pty_host, parser, text_grid).await
}

async fn run_gui_terminal(
    cli: &Cli,
    pty_host: Arc<Mutex<PtyHost>>,
    parser: Arc<Mutex<TerminalParser>>,
    text_grid: Arc<Mutex<TextGrid>>,
) -> Result<()> {
    let event_loop = EventLoop::new()
        .map_err(|e| termind::TermindError::Configuration(format!("Failed to create event loop: {}", e)))?;
    
    let window = WindowBuilder::new()
        .with_title("Termind - Privacy-first AI Terminal")
        .with_inner_size(winit::dpi::LogicalSize::new(
            (cli.width as f64) * 8.0, // 8px per char width (rough estimate)
            (cli.height as f64) * 16.0, // 16px per char height  
        ))
        .build(&event_loop)
        .map_err(|e| termind::TermindError::Configuration(format!("Failed to create window: {}", e)))?;
    
    info!("✅ Terminal window opened successfully");
    info!("🔄 Starting GUI event loop - terminal is now interactive!");
    info!("💡 Type commands or press Escape to quit");
    
    // Clone Arc references for the background PTY reader task
    let pty_host_reader = pty_host.clone();
    let parser_reader = parser.clone();
    let _text_grid_reader = text_grid.clone();
    
    // Spawn background task to continuously read from PTY
    let _reader_handle = tokio::spawn(async move {
        let mut status_counter = 0;
        loop {
            let data = {
                let mut pty = pty_host_reader.lock().await;
                match pty.try_read().await {
                    Ok(data) => data,
                    Err(e) => {
                        error!("❌ Error reading from PTY: {}", e);
                        break;
                    }
                }
            };
            
            if !data.is_empty() {
                // Parse the data and update grid
                {
                    let mut parser = parser_reader.lock().await;
                    parser.parse(&data);
                }
                
                // For now, also print to stdout for debugging
                let text = String::from_utf8_lossy(&data);
                print!("{}", text);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            } else {
                // No data available, sleep a bit
                sleep(Duration::from_millis(10)).await;
                
                // Periodic status updates
                status_counter += 1;
                if status_counter % 500 == 0 { // Every ~5 seconds
                    let pty = pty_host_reader.lock().await;
                    info!("📊 Terminal active - shell PID: {}", pty.child_pid());
                }
            }
        }
    });
    
    // Run the GUI event loop (blocking)
    let result = run_event_loop(event_loop, window, pty_host);
    
    info!("🧹 Terminal session ended");
    result
}

fn run_event_loop(
    event_loop: EventLoop<()>,
    window: winit::window::Window,
    pty_host: Arc<Mutex<PtyHost>>,
) -> Result<()> {
    // Create a tokio runtime handle for async operations within the event loop
    let rt = tokio::runtime::Handle::current();
    
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if window_id == window.id() => {
                info!("🪟 Window close requested");
                elwt.exit();
            }
            
            Event::WindowEvent {
                window_id,
                event: WindowEvent::KeyboardInput {
                    event: KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state: ElementState::Pressed,
                        text,
                        ..
                    },
                    ..
                },
            } if window_id == window.id() => {
                // Handle keyboard input
                match keycode {
                    KeyCode::Escape => {
                        info!("🚪 Escape pressed, exiting...");
                        elwt.exit();
                    }
                    _ => {
                        // Forward other keys to the PTY
                        if let Some(text) = text {
                            let pty_host = pty_host.clone();
                            let text = text.to_string();
                            rt.spawn(async move {
                                let mut pty = pty_host.lock().await;
                                if let Err(e) = pty.write(text.as_bytes()).await {
                                    warn!("⚠️ Failed to write to PTY: {}", e);
                                }
                            });
                        }
                    }
                }
            }
            
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if window_id == window.id() => {
                info!("📏 Window resized to {:?}", size);
                // TODO: Update terminal size based on window size
            }
            
            Event::WindowEvent {
                window_id,
                event: WindowEvent::RedrawRequested,
            } if window_id == window.id() => {
                // TODO: Render the terminal grid to the window
                // For now, we just validate the window
                window.pre_present_notify();
            }
            
            _ => {}
        }
    })
    .map_err(|e| termind::TermindError::Configuration(format!("Event loop error: {}", e)))?;
    
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
