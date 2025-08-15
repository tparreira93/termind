use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use winit::{
    event::{Event, WindowEvent, KeyEvent, ElementState},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{PhysicalKey, KeyCode},
    window::WindowBuilder,
};
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;

use termind::renderer::{TextGrid, TerminalParser};
use termind::renderer::software::SoftwareRenderer;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().init();

    info!("🚀 Starting Termind v0.3.0 - Software Rendering");
    info!("📋 Initializing components...");

    // Set up terminal dimensions
    let terminal_cols = 80;
    let terminal_rows = 24;
    info!("📏 Terminal size: {}x{}", terminal_cols, terminal_rows);

    // Initialize PTY host (we'll create a mock for now)
    info!("🐚 Creating terminal components...");
    let pty_host = Arc::new(Mutex::new(MockPtyHost::new()));
    
    info!("✅ Shell components created");

    // Initialize text grid and parser
    let text_grid = Arc::new(Mutex::new(TextGrid::new(terminal_rows, terminal_cols)));
    let parser = Arc::new(Mutex::new(TerminalParser::new(terminal_rows, terminal_cols)));

    // Add some test data to the grid
    {
        let mut grid = text_grid.lock().await;
        // Create a simple test message
        let test_msg = "Termind Terminal Ready!";
        for (i, ch) in test_msg.chars().enumerate() {
            if i < 80 {
                grid.set_char(0, i as u16, ch);
            }
        }
        
        let prompt_msg = "Type 'hello' and press Enter ❯ ";
        for (i, ch) in prompt_msg.chars().enumerate() {
            if i < 80 {
                grid.set_char(1, i as u16, ch);
            }
        }
    }

    // Create the window and event loop
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Termind - Software Terminal")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)?;

    info!("🪟 Window created successfully");

    // Initialize software renderer
    let size = window.inner_size();
    let mut software_renderer = SoftwareRenderer::new(size)?;

    // Initialize softbuffer
    let context = Context::new(&window).unwrap();
    let mut surface = Surface::new(&context, &window).unwrap();
    
    // Store window ID for redraw requests
    let window_id = window.id();

    info!("✅ Software renderer initialized");
    info!("🔄 Starting event loop - press Escape to quit");

    // Run the event loop
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::AboutToWait => {
                // Render frame
                if let Ok(text_grid_locked) = text_grid.try_lock() {
                    if let Ok(pixel_buffer) = software_renderer.render_frame(&*text_grid_locked) {
                        // Get surface buffer and copy pixels
                        if let Ok(mut buffer) = surface.buffer_mut() {
                            // Convert our RGBA buffer to the format softbuffer expects
                            for (i, &pixel) in pixel_buffer.iter().enumerate() {
                                if i < buffer.len() {
                                    // Convert from RGBA to RGB format that softbuffer expects
                                    let r = ((pixel >> 16) & 0xFF) as u32;
                                    let g = ((pixel >> 8) & 0xFF) as u32;
                                    let b = (pixel & 0xFF) as u32;
                                    buffer[i] = (r << 16) | (g << 8) | b;
                                }
                            }
                            
                            // Present the buffer
                            if let Err(e) = buffer.present() {
                                warn!("Failed to present buffer: {}", e);
                            }
                        }
                    }
                }
                return;
            }
            _ => {}
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("🪟 Window close requested");
                elwt.exit();
            }

            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    event: KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state: ElementState::Pressed,
                        ..
                    },
                    ..
                },
                ..
            } => {
                match keycode {
                    KeyCode::Escape => {
                        info!("🚪 Escape pressed, exiting...");
                        elwt.exit();
                    }
                    KeyCode::Enter => {
                        // Add a response line
                        let grid = text_grid.clone();
                        tokio::spawn(async move {
                            let mut grid = grid.lock().await;
                            let response = "Hello! Software rendering works! 🎉";
                            for (i, ch) in response.chars().enumerate() {
                                if i < 80 {
                                    grid.set_char(2, i as u16, ch);
                                }
                            }
                        });
                    }
                    _ => {
                        info!("Key pressed: {:?}", keycode);
                    }
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                info!("📏 Window resized to {:?}", new_size);
                surface.resize(
                    NonZeroU32::new(new_size.width).unwrap(),
                    NonZeroU32::new(new_size.height).unwrap(),
                ).unwrap();
                
                if let Err(e) = software_renderer.resize(new_size) {
                    warn!("Failed to resize software renderer: {}", e);
                }
            }

            _ => {}
        }
    })?;

    Ok(())
}

// Mock PTY Host for testing
struct MockPtyHost;

impl MockPtyHost {
    fn new() -> Self {
        Self
    }
}
