use fontdue::{Font, FontSettings};
use winit::dpi::PhysicalSize;

use crate::renderer::{TextGrid, RenderError};

/// Represents a rectangular cell in the terminal grid
#[derive(Debug, Clone, Copy)]
struct CellRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

pub struct SoftwareRenderer {
    font: Font,
    font_size: f32,
    char_width: u32,
    char_height: u32,
    size: PhysicalSize<u32>,
    pixel_buffer: Vec<u32>, // RGBA pixels
    // Grid properties
    grid_cols: u32,
    grid_rows: u32,
    cell_width: u32,
    cell_height: u32,
    // Font baseline info for consistent positioning
    baseline_offset: u32,
    ascent: f32,
    descent: f32,
}

impl SoftwareRenderer {
    pub fn new(size: PhysicalSize<u32>) -> Result<Self, RenderError> {
        tracing::info!("🖥️  Initializing software renderer");
        
        // Load system font
        let font_data = Self::load_system_font()?
            .ok_or_else(|| RenderError::Font("No suitable font found".to_string()))?;
            
        tracing::info!("📝 Loaded font data: {} bytes", font_data.len());
        
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| RenderError::Font(format!("Failed to load font: {}", e)))?;
            
        tracing::info!("✅ Font parsed successfully");
        
        // Calculate character dimensions using proper metrics
        let font_size = 16.0;
        let (metrics, _) = font.rasterize('M', font_size); // Use 'M' for measuring
        let line_metrics = font.horizontal_line_metrics(font_size).unwrap();
        
        // Store font metrics for consistent baseline positioning
        let ascent = line_metrics.ascent;
        let descent = line_metrics.descent;
        
        // Calculate ideal cell dimensions (make them slightly larger than character bounds)
        let char_width = metrics.advance_width.ceil() as u32;
        let char_height = (ascent - descent + line_metrics.line_gap).ceil() as u32;
        
        // Make cells uniform rectangles with some padding
        let cell_width = char_width.max(12); // Minimum cell width
        let cell_height = char_height.max(20); // Minimum cell height
        
        // Calculate grid dimensions based on window size
        let padding = 8; // 8px padding around the entire grid
        let usable_width = size.width.saturating_sub(padding * 2);
        let usable_height = size.height.saturating_sub(padding * 2);
        
        let grid_cols = usable_width / cell_width;
        let grid_rows = usable_height / cell_height;
        
        // Calculate baseline offset within each cell (where characters sit)
        let baseline_offset = (cell_height as f32 * 0.8) as u32;
        
        tracing::info!("📊 Font metrics - advance_width: {}, ascent: {}, descent: {}, line_gap: {}", 
                      metrics.advance_width, ascent, descent, line_metrics.line_gap);
        
        tracing::info!("🔤 Character dimensions: {}x{}", char_width, char_height);
        tracing::info!("📐 Cell dimensions: {}x{}", cell_width, cell_height);
        tracing::info!("📋 Grid dimensions: {}x{} cells", grid_cols, grid_rows);
        
        let pixel_buffer = vec![0xFF000000u32; (size.width * size.height) as usize]; // Black background
        
        Ok(Self {
            font,
            font_size,
            char_width,
            char_height,
            size,
            pixel_buffer,
            grid_cols,
            grid_rows,
            cell_width,
            cell_height,
            baseline_offset,
            ascent,
            descent,
        })
    }
    
    fn load_system_font() -> Result<Option<Vec<u8>>, RenderError> {
        let font_paths = [
            "/System/Library/Fonts/Monaco.ttf",
            "/System/Library/Fonts/Menlo.ttc", 
            "/Library/Fonts/SF Mono Regular.otf",
            "/System/Library/Fonts/Courier New.ttf",
        ];
        
        tracing::debug!("🔍 Searching for system fonts...");
        
        for path in &font_paths {
            tracing::debug!("  Trying: {}", path);
            if let Ok(data) = std::fs::read(path) {
                tracing::info!("✅ Found font: {} ({} bytes)", path, data.len());
                return Ok(Some(data));
            } else {
                tracing::debug!("  ❌ Not found: {}", path);
            }
        }
        
        tracing::warn!("⚠️  No system fonts found");
        Ok(None)
    }
    
    pub fn render_frame(&mut self, grid: &TextGrid) -> Result<&[u32], RenderError> {
        // Clear buffer to black
        self.pixel_buffer.fill(0xFF000000u32);
        
        tracing::debug!("🖥️  Software rendering frame {}x{}", self.size.width, self.size.height);
        
        let mut chars_rendered = 0;
        
        // First, optionally draw grid lines for debugging (remove in production)
        self.draw_debug_grid();
        
        // Calculate grid offset to center the terminal grid
        let padding = 8;
        let grid_start_x = padding;
        let grid_start_y = padding;
        
        // Render each character within its designated cell
        let max_rows = grid.rows.min(self.grid_rows as u16);
        let max_cols = self.grid_cols.min(80) as u16; // Cap at typical terminal width
        
        tracing::debug!("📐 Grid render area: {}x{} cells, cell_size={}x{}", 
                       max_rows, max_cols, self.cell_width, self.cell_height);
        
        for row in 0..max_rows {
            if let Some(row_data) = grid.row(row) {
                for col in 0..(row_data.len().min(max_cols as usize)) {
                    if let Some(cell) = grid.cell_at(row, col as u16) {
                        if cell.ch != '\0' && cell.ch != ' ' {
                            // Calculate the exact cell rectangle
                            let cell_rect = self.get_cell_rect(row as u32, col as u32, grid_start_x, grid_start_y);
                            
                            // Render character centered within its cell
                            self.render_char_in_cell(
                                cell.ch,
                                cell_rect,
                                0xFFFFFFFFu32, // White text
                            );
                            chars_rendered += 1;
                        }
                    }
                }
            }
        }
        
        if chars_rendered > 0 {
            tracing::debug!("🔤 Software rendered {} characters in grid cells", chars_rendered);
        } else {
            tracing::debug!("⚠️  No characters to render (grid may be empty)");
        }
        
        Ok(&self.pixel_buffer)
    }
    
    /// Calculate the exact rectangle for a grid cell
    fn get_cell_rect(&self, row: u32, col: u32, grid_start_x: u32, grid_start_y: u32) -> CellRect {
        let x = grid_start_x + col * self.cell_width;
        let y = grid_start_y + row * self.cell_height;
        
        CellRect {
            x,
            y,
            width: self.cell_width,
            height: self.cell_height,
        }
    }
    
    /// Draw debug grid lines (optional, for development)
    fn draw_debug_grid(&mut self) {
        // Enable to see grid lines for debugging
        let grid_color = 0xFF222222u32; // Very dark gray
        let padding = 8;
        
        // Draw vertical lines (every few cells to avoid clutter)
        for col in (0..=self.grid_cols).step_by(5) {
            let x = padding + col * self.cell_width;
            if x < self.size.width {
                for y in padding..(padding + self.grid_rows * self.cell_height).min(self.size.height) {
                    let idx = (y * self.size.width + x) as usize;
                    if idx < self.pixel_buffer.len() {
                        self.pixel_buffer[idx] = grid_color;
                    }
                }
            }
        }
        
        // Draw horizontal lines (every few cells to avoid clutter)
        for row in (0..=self.grid_rows).step_by(5) {
            let y = padding + row * self.cell_height;
            if y < self.size.height {
                for x in padding..(padding + self.grid_cols * self.cell_width).min(self.size.width) {
                    let idx = (y * self.size.width + x) as usize;
                    if idx < self.pixel_buffer.len() {
                        self.pixel_buffer[idx] = grid_color;
                    }
                }
            }
        }
    }
    
    /// Render a character within a specific cell rectangle
    fn render_char_in_cell(&mut self, ch: char, cell_rect: CellRect, color: u32) {
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
        
        // Calculate character position within the cell
        // Center horizontally, align to baseline vertically
        let char_x = cell_rect.x + (cell_rect.width.saturating_sub(metrics.width as u32)) / 2;
        let char_y = cell_rect.y + self.baseline_offset;
        
        // Adjust for font metrics (handle negative ymin safely)
        let final_char_y = if metrics.ymin < 0 {
            char_y + (-metrics.ymin) as u32
        } else {
            char_y.saturating_sub(metrics.ymin as u32)
        };
        
        // Debug logging for first few characters
        if ch == 'T' || ch == 'H' {
            tracing::debug!("Rendering '{}' in cell ({}, {}, {}x{}) -> char pos ({}, {}), metrics: {}x{}", 
                           ch, cell_rect.x, cell_rect.y, cell_rect.width, cell_rect.height,
                           char_x, final_char_y, metrics.width, metrics.height);
        }
        
        // Draw character bitmap within the cell bounds
        for bitmap_y in 0..metrics.height {
            for bitmap_x in 0..metrics.width {
                let pixel_x = char_x + bitmap_x as u32;
                let pixel_y = final_char_y + bitmap_y as u32;
                
                // Ensure we stay within cell boundaries and screen bounds
                if pixel_x >= cell_rect.x && pixel_x < cell_rect.x + cell_rect.width &&
                   pixel_y >= cell_rect.y && pixel_y < cell_rect.y + cell_rect.height &&
                   pixel_x < self.size.width && pixel_y < self.size.height {
                    
                    let bitmap_idx = bitmap_y * metrics.width + bitmap_x;
                    if bitmap_idx < bitmap.len() {
                        let alpha = bitmap[bitmap_idx];
                        if alpha > 64 { // Antialiasing threshold
                            let buffer_idx = (pixel_y * self.size.width + pixel_x) as usize;
                            if buffer_idx < self.pixel_buffer.len() {
                                // High-quality alpha blending
                                let alpha_f = alpha as f32 / 255.0;
                                let existing = self.pixel_buffer[buffer_idx];
                                
                                let existing_r = ((existing >> 16) & 0xFF) as f32;
                                let existing_g = ((existing >> 8) & 0xFF) as f32;
                                let existing_b = (existing & 0xFF) as f32;
                                
                                let new_r = ((color >> 16) & 0xFF) as f32;
                                let new_g = ((color >> 8) & 0xFF) as f32;
                                let new_b = (color & 0xFF) as f32;
                                
                                let blended_r = (existing_r * (1.0 - alpha_f) + new_r * alpha_f).clamp(0.0, 255.0) as u32;
                                let blended_g = (existing_g * (1.0 - alpha_f) + new_g * alpha_f).clamp(0.0, 255.0) as u32;
                                let blended_b = (existing_b * (1.0 - alpha_f) + new_b * alpha_f).clamp(0.0, 255.0) as u32;
                                
                                self.pixel_buffer[buffer_idx] = 0xFF000000 | (blended_r << 16) | (blended_g << 8) | blended_b;
                            }
                        }
                    }
                }
            }
        }
    }
    
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) -> Result<(), RenderError> {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.pixel_buffer = vec![0xFF000000u32; (new_size.width * new_size.height) as usize];
            
            // Recalculate grid dimensions for new window size
            let padding = 8;
            let usable_width = new_size.width.saturating_sub(padding * 2);
            let usable_height = new_size.height.saturating_sub(padding * 2);
            
            self.grid_cols = usable_width / self.cell_width;
            self.grid_rows = usable_height / self.cell_height;
            
            tracing::info!("📏 Software renderer resized to {}x{}", new_size.width, new_size.height);
            tracing::info!("📋 New grid dimensions: {}x{} cells", self.grid_cols, self.grid_rows);
        }
        Ok(())
    }
    
    pub fn char_width(&self) -> u32 {
        self.char_width
    }
    
    pub fn char_height(&self) -> u32 {
        self.char_height
    }
    
    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }
}
