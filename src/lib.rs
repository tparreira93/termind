//! Termind - Privacy-first, AI-powered terminal
//! 
//! This library provides the core components for Termind, a terminal emulator
//! that runs entirely on your machine with AI-powered command assistance.
//!
//! ## Phase A Components
//! 
//! - **PTY Host**: Real terminal spawning with async I/O
//! - **Terminal Parser**: VT100/ANSI escape sequence parsing
//! - **Text Grid**: Terminal screen state representation
//! - **Block Detection**: Command block identification and storage (Week 3)
//! - **GPU Renderer**: Hardware-accelerated terminal rendering (stub)

pub mod error;
pub mod pty;
pub mod renderer;
pub mod blocks;

// Re-export commonly used types
pub use error::{Result, TermindError};
pub use pty::{PtyHost, SignalHandler, ProcessManager};
pub use renderer::{TextGrid, TerminalParser, colors};
pub use blocks::BlockDetector;
