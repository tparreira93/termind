use fontdue::{Font, FontSettings};
use winit::dpi::PhysicalSize;

use crate::renderer::{TextGrid, RenderError};

pub struct SoftwareRenderer {
    font: Font,
    font_size: f32,
    char_width: u32,
    char_height: u32,
    size: PhysicalSize<u32>,
    pixel_buffer: Vec<u32>, // RGBA pixels
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
        
        // Use advance width for character width (includes spacing)
        let char_width = metrics.advance_width.ceil() as u32;
        
        // Use proper line height (ascent + descent + line gap)
        let char_height = (line_metrics.ascent - line_metrics.descent + line_metrics.line_gap).ceil() as u32;
        
        tracing::info!("📊 Font metrics - advance_width: {}, ascent: {}, descent: {}, line_gap: {}", 
                      metrics.advance_width, line_metrics.ascent, line_metrics.descent, line_metrics.line_gap);
        
        tracing::info!("🔤 Character dimensions: {}x{}", char_width, char_height);
        
        let pixel_buffer = vec![0xFF000000u32; (size.width * size.height) as usize]; // Black background
        
        Ok(Self {
            font,
            font_size,
            char_width,
            char_height,
            size,
            pixel_buffer,
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
        
        // Render each character in the grid
        let max_rows = (self.size.height / self.char_height) as u16;
        let max_cols = (self.size.width / self.char_width) as u16;
        
        tracing::debug!("📐 Grid render area: max_rows={}, max_cols={}, char_size={}x{}", 
                       max_rows, max_cols, self.char_width, self.char_height);
        
        for row in 0..grid.rows.min(max_rows) {
            if let Some(row_data) = grid.row(row) {
                for col in 0..row_data.len().min(max_cols as usize) {
                    if let Some(cell) = grid.cell_at(row, col as u16) {
                        // Render all characters, including spaces (for proper spacing)
                        if cell.ch != '\0' {
                            let x = col as u32 * self.char_width;
                            let y = row as u32 * self.char_height;
                            
                            // Add padding from window edges
                            let padded_x = x + 4; // 4px left padding
                            let padded_y = y + 4; // 4px top padding
                            
                            if cell.ch != ' ' { // Don't render actual space characters
                                self.render_char(
                                    cell.ch,
                                    padded_x,
                                    padded_y,
                                    0xFFFFFFFFu32, // White text
                                );
                                chars_rendered += 1;
                            }
                        }
                    }
                }
            }
        }
        
        if chars_rendered > 0 {
            tracing::debug!("🔤 Software rendered {} characters", chars_rendered);
        } else {
            tracing::debug!("⚠️  No characters to render (grid may be empty)");
        }
        
        Ok(&self.pixel_buffer)
    }
    
    fn render_char(&mut self, ch: char, x: u32, y: u32, color: u32) {
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
        
        // Calculate proper baseline positioning
        // Fontdue gives us metrics relative to the baseline, but we need to position
        // the character correctly within our line height
        let baseline_offset = (self.char_height as f32 * 0.8) as u32; // Approximate baseline position
        
        // Handle negative ymin values safely to avoid overflow
        let char_y = if metrics.ymin < 0 {
            y + baseline_offset + (-metrics.ymin) as u32
        } else {
            y + baseline_offset - metrics.ymin as u32
        };
        
        // Reduced debug logging to avoid spam
        if ch == 'T' || ch == 'H' { // Only log certain characters
            tracing::debug!("Rendering '{}' at ({}, {}) -> baseline adjusted ({}, {}), metrics: {}x{}, ymin: {}", 
                           ch, x, y, x, char_y, metrics.width, metrics.height, metrics.ymin);
        }
        
        // Draw character bitmap to pixel buffer
        for bitmap_y in 0..metrics.height {
            for bitmap_x in 0..metrics.width {
                let pixel_x = x + bitmap_x as u32;
                let pixel_y = char_y + bitmap_y as u32;
                
                if pixel_x < self.size.width && pixel_y < self.size.height {
                    let bitmap_idx = bitmap_y * metrics.width + bitmap_x;
                    if bitmap_idx < bitmap.len() {
                        let alpha = bitmap[bitmap_idx];
                        if alpha > 0 {
                            // Better alpha blending: use actual alpha value
                            if alpha > 64 { // Lower threshold for better antialiasing
                                let buffer_idx = (pixel_y * self.size.width + pixel_x) as usize;
                                if buffer_idx < self.pixel_buffer.len() {
                                    // Simple alpha blend with existing pixel
                                    let alpha_f = alpha as f32 / 255.0;
                                    let existing = self.pixel_buffer[buffer_idx];
                                    let existing_r = ((existing >> 16) & 0xFF) as f32;
                                    let existing_g = ((existing >> 8) & 0xFF) as f32;
                                    let existing_b = (existing & 0xFF) as f32;
                                    
                                    let new_r = ((color >> 16) & 0xFF) as f32;
                                    let new_g = ((color >> 8) & 0xFF) as f32;
                                    let new_b = (color & 0xFF) as f32;
                                    
                                    let blended_r = (existing_r * (1.0 - alpha_f) + new_r * alpha_f) as u32;
                                    let blended_g = (existing_g * (1.0 - alpha_f) + new_g * alpha_f) as u32;
                                    let blended_b = (existing_b * (1.0 - alpha_f) + new_b * alpha_f) as u32;
                                    
                                    self.pixel_buffer[buffer_idx] = 0xFF000000 | (blended_r << 16) | (blended_g << 8) | blended_b;
                                }
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
            tracing::info!("📏 Software renderer resized to {}x{}", new_size.width, new_size.height);
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
