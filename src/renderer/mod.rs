pub mod gpu;
pub mod software;
pub mod grid;
pub mod parser;
pub mod colors;

pub use gpu::RenderError;
// Note: GpuRenderer is generic and needs to be used with lifetime parameter
pub use grid::{TextGrid, Cell, CellAttributes, Region};
pub use parser::TerminalParser;
pub use colors::TerminalColor;
