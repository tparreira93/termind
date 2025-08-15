use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, warn, error};
use winit::{
    event::{Event, WindowEvent, KeyEvent, ElementState},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{PhysicalKey, KeyCode},
    window::WindowBuilder,
};
use pixels::{Pixels, SurfaceTexture};

use termind::pty::PtyHost;
use termind::renderer::{TextGrid, TerminalParser};
use termind::renderer::software::SoftwareRenderer;
use termind::storage::BlockDetector;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("termind=info,debug")
        .init();

    info!("🚀 Starting Termind v0.3.0 - Phase A Week 3 (Software Rendering)");
    info!("📋 Initializing Phase A components...");

    // Initialize storage first
    let _block_detector = BlockDetector::new().await
        .map_err(|e| format!("Failed to initialize block detector: {}", e))?;

    info!("🔧 Components initialized successfully");

    // Set up terminal dimensions
    let terminal_cols = 80;
    let terminal_rows = 24;
    info!("📏 Terminal size: {}x{}", terminal_cols, terminal_rows);

    // Initialize PTY host
    info!("🐚 Spawning shell...");
    let pty_host = Arc::new(Mutex::new(
        PtyHost::new(terminal_rows, terminal_cols).await
            .map_err(|e| format!("Failed to create PTY host: {}", e))?
    ));

    info!("✅ Shell spawned successfully: /bin/zsh");

    // Initialize text grid and parser
    let text_grid = Arc::new(Mutex::new(TextGrid::new(terminal_rows, terminal_cols)));
    let parser = Arc::new(Mutex::new(TerminalParser::new(terminal_rows, terminal_cols)));

    // Create the window and event loop
    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;
    let window = WindowBuilder::new()
        .with_title("Termind - Software Rendered Terminal")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .map_err(|e| format!("Failed to create window: {}", e))?;

    info!("🪟 Opening terminal window...");

    // Initialize software renderer
    let size = window.inner_size();
    let mut software_renderer = SoftwareRenderer::new(size)?;

    // Initialize pixels for displaying the software-rendered buffer
    let surface_texture = SurfaceTexture::new(size.width, size.height, &window);
    let mut pixels = Pixels::new(size.width, size.height, surface_texture)?;

    info!("✅ Terminal window opened successfully");
    info!("🔄 Starting GUI event loop - terminal is now interactive!");
    info!("💡 Type commands or press Escape to quit");

    // Clone Arc references for the background PTY reader task
    let pty_host_reader = pty_host.clone();
    let parser_reader = parser.clone();
    let text_grid_reader = text_grid.clone();

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
                // Debug: Show what data we received from the PTY
                let data_str = String::from_utf8_lossy(&data);
                if !data_str.trim().is_empty() && data_str.len() < 100 {
                    info!("📝 PTY data: {:?}", data_str);
                } else if !data.is_empty() {
                    info!("📝 PTY data: {} bytes", data.len());
                }

                // Parse the data and update grid
                {
                    let mut parser = parser_reader.lock().await;
                    parser.parse(&data);

                    // Copy updated grid from parser to our shared grid
                    let parser_grid = parser.grid();
                    let mut text_grid = text_grid_reader.lock().await;

                    let mut cells_copied = 0;
                    // Update the shared grid with parser data
                    for row in 0..parser_grid.rows.min(text_grid.rows) {
                        if let Some(parser_row) = parser_grid.row(row) {
                            for col in 0..parser_row.len().min(text_grid.cols as usize) {
                                // Copy cell data from parser to display grid
                                if let Some(parser_cell) = parser_grid.cell_at(row, col as u16) {
                                    // Update the text grid with the parser's cell data
                                    text_grid.set_cell(row, col as u16, parser_cell);
                                    if parser_cell.ch != '\0' && parser_cell.ch != ' ' {
                                        cells_copied += 1;
                                    }
                                }
                            }
                        }
                    }
                    if cells_copied > 0 {
                        info!("🔄 Copied {} non-empty cells to display grid", cells_copied);
                    }
                }
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

    info!("🖥️  Software renderer initialized successfully");

    // Run the GUI event loop (blocking, synchronous)
    run_event_loop(event_loop, window, pty_host, text_grid, software_renderer, pixels)
}

fn run_event_loop(
    event_loop: EventLoop<()>,
    window: winit::window::Window,
    pty_host: Arc<Mutex<PtyHost>>,
    text_grid: Arc<Mutex<TextGrid>>,
    mut software_renderer: SoftwareRenderer,
    mut pixels: Pixels,
) -> Result<()> {
    // Store window ID for comparison in event loop
    let window_id = window.id();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::AboutToWait => {
                // Render the terminal using software renderer
                if let Ok(text_grid_locked) = text_grid.try_lock() {
                    if let Ok(pixel_buffer) = software_renderer.render_frame(&*text_grid_locked) {
                        // Copy the software-rendered buffer to pixels
                        let frame = pixels.frame_mut();
                        for (i, pixel) in pixel_buffer.iter().enumerate() {
                            if i * 4 + 3 < frame.len() {
                                let rgba = pixel.to_le_bytes();
                                frame[i * 4] = rgba[2];     // R
                                frame[i * 4 + 1] = rgba[1]; // G
                                frame[i * 4 + 2] = rgba[0]; // B
                                frame[i * 4 + 3] = rgba[3]; // A
                            }
                        }

                        if let Err(e) = pixels.render() {
                            warn!("Failed to render pixels: {}", e);
                        }
                    }
                } else {
                    // If we can't lock the text grid, render empty
                    let simple_grid = TextGrid::new(24, 80);
                    if let Ok(pixel_buffer) = software_renderer.render_frame(&simple_grid) {
                        let frame = pixels.frame_mut();
                        for (i, pixel) in pixel_buffer.iter().enumerate() {
                            if i * 4 + 3 < frame.len() {
                                let rgba = pixel.to_le_bytes();
                                frame[i * 4] = rgba[2];     // R
                                frame[i * 4 + 1] = rgba[1]; // G
                                frame[i * 4 + 2] = rgba[0]; // B
                                frame[i * 4 + 3] = rgba[3]; // A
                            }
                        }

                        if let Err(e) = pixels.render() {
                            warn!("Failed to render pixels: {}", e);
                        }
                    }
                }
                return;
            }
            _ => {} // Continue to normal event processing
        }

        match event {
            Event::WindowEvent {
                window_id: event_window_id,
                event: WindowEvent::CloseRequested,
            } if event_window_id == window_id => {
                info!("🪟 Window close requested");
                elwt.exit();
            }

            Event::WindowEvent {
                window_id: event_window_id,
                event: WindowEvent::KeyboardInput {
                    event: KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state: ElementState::Pressed,
                        text,
                        ..
                    },
                    ..
                },
            } if event_window_id == window_id => {
                // Handle keyboard input
                match keycode {
                    KeyCode::Escape => {
                        info!("🚪 Escape pressed, exiting...");
                        elwt.exit();
                    }
                    KeyCode::Enter => {
                        // Send carriage return + line feed
                        let pty_host = pty_host.clone();
                        tokio::task::spawn(async move {
                            let mut pty = pty_host.lock().await;
                            if let Err(e) = pty.write(b"\r").await {
                                warn!("⚠️ Failed to write to PTY: {}", e);
                            }
                        });
                        elwt.set_control_flow(ControlFlow::Poll);
                    }
                    _ => {
                        // Forward other keys to the PTY
                        if let Some(text) = text {
                            let pty_host = pty_host.clone();
                            let text = text.to_string();
                            tokio::task::spawn(async move {
                                let mut pty = pty_host.lock().await;
                                if let Err(e) = pty.write(text.as_bytes()).await {
                                    warn!("⚠️ Failed to write to PTY: {}", e);
                                }
                            });
                            elwt.set_control_flow(ControlFlow::Poll);
                        }
                    }
                }
            }

            Event::WindowEvent {
                window_id: event_window_id,
                event: WindowEvent::Resized(size),
            } if event_window_id == window_id => {
                info!("📏 Window resized to {:?}", size);

                // Resize the software renderer and pixels
                if let Err(e) = software_renderer.resize(size) {
                    warn!("Failed to resize software renderer: {}", e);
                }

                if let Err(e) = pixels.resize_surface(size.width, size.height) {
                    warn!("Failed to resize pixels: {}", e);
                }

                elwt.set_control_flow(ControlFlow::Poll);
            }

            Event::WindowEvent {
                window_id: event_window_id,
                event: WindowEvent::RedrawRequested,
            } if event_window_id == window_id => {
                // This will be handled by AboutToWait
                window.request_redraw();
            }

            _ => {}
        }
    })
    .map_err(|e| format!("Event loop error: {}", e).into())
}
