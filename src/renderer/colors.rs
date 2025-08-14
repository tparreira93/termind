#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminalColor {
    // Standard 16 colors
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    
    // 256-color mode
    Indexed(u8),
    
    // True color (RGB)
    Rgb { r: u8, g: u8, b: u8 },
    
    // Default terminal colors
    DefaultFg,
    DefaultBg,
}

impl Default for TerminalColor {
    fn default() -> Self {
        TerminalColor::DefaultFg
    }
}

impl TerminalColor {
    /// Convert ANSI color code to TerminalColor
    pub fn from_ansi_code(code: u8) -> Self {
        match code {
            30 => TerminalColor::Black,
            31 => TerminalColor::Red,
            32 => TerminalColor::Green,
            33 => TerminalColor::Yellow,
            34 => TerminalColor::Blue,
            35 => TerminalColor::Magenta,
            36 => TerminalColor::Cyan,
            37 => TerminalColor::White,
            90 => TerminalColor::BrightBlack,
            91 => TerminalColor::BrightRed,
            92 => TerminalColor::BrightGreen,
            93 => TerminalColor::BrightYellow,
            94 => TerminalColor::BrightBlue,
            95 => TerminalColor::BrightMagenta,
            96 => TerminalColor::BrightCyan,
            97 => TerminalColor::BrightWhite,
            39 => TerminalColor::DefaultFg,
            49 => TerminalColor::DefaultBg,
            _ => TerminalColor::DefaultFg,
        }
    }
    
    /// Convert to RGB values for rendering
    pub fn to_rgb(self) -> [f32; 4] { // RGBA
        match self {
            TerminalColor::Black => [0.0, 0.0, 0.0, 1.0],
            TerminalColor::Red => [0.8, 0.0, 0.0, 1.0],
            TerminalColor::Green => [0.0, 0.8, 0.0, 1.0],
            TerminalColor::Yellow => [0.8, 0.8, 0.0, 1.0],
            TerminalColor::Blue => [0.0, 0.0, 0.8, 1.0],
            TerminalColor::Magenta => [0.8, 0.0, 0.8, 1.0],
            TerminalColor::Cyan => [0.0, 0.8, 0.8, 1.0],
            TerminalColor::White => [0.8, 0.8, 0.8, 1.0],
            TerminalColor::BrightBlack => [0.4, 0.4, 0.4, 1.0],
            TerminalColor::BrightRed => [1.0, 0.4, 0.4, 1.0],
            TerminalColor::BrightGreen => [0.4, 1.0, 0.4, 1.0],
            TerminalColor::BrightYellow => [1.0, 1.0, 0.4, 1.0],
            TerminalColor::BrightBlue => [0.4, 0.4, 1.0, 1.0],
            TerminalColor::BrightMagenta => [1.0, 0.4, 1.0, 1.0],
            TerminalColor::BrightCyan => [0.4, 1.0, 1.0, 1.0],
            TerminalColor::BrightWhite => [1.0, 1.0, 1.0, 1.0],
            TerminalColor::Indexed(idx) => {
                // 256-color palette
                Self::indexed_to_rgb(idx)
            }
            TerminalColor::Rgb { r, g, b } => {
                [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
            }
            TerminalColor::DefaultFg => [0.9, 0.9, 0.9, 1.0], // Light gray
            TerminalColor::DefaultBg => [0.0, 0.0, 0.0, 1.0], // Black
        }
    }
    
    fn indexed_to_rgb(index: u8) -> [f32; 4] {
        match index {
            // Standard 16 colors (0-15)
            0..=15 => {
                let colors = [
                    [0.0, 0.0, 0.0], // Black
                    [0.8, 0.0, 0.0], // Red
                    [0.0, 0.8, 0.0], // Green
                    [0.8, 0.8, 0.0], // Yellow
                    [0.0, 0.0, 0.8], // Blue
                    [0.8, 0.0, 0.8], // Magenta
                    [0.0, 0.8, 0.8], // Cyan
                    [0.8, 0.8, 0.8], // White
                    [0.4, 0.4, 0.4], // Bright Black
                    [1.0, 0.4, 0.4], // Bright Red
                    [0.4, 1.0, 0.4], // Bright Green
                    [1.0, 1.0, 0.4], // Bright Yellow
                    [0.4, 0.4, 1.0], // Bright Blue
                    [1.0, 0.4, 1.0], // Bright Magenta
                    [0.4, 1.0, 1.0], // Bright Cyan
                    [1.0, 1.0, 1.0], // Bright White
                ];
                let [r, g, b] = colors[index as usize];
                [r, g, b, 1.0]
            }
            
            // 216 color cube (16-231)
            16..=231 => {
                let index = index - 16;
                let r = (index / 36) as f32 / 5.0;
                let g = ((index % 36) / 6) as f32 / 5.0;
                let b = (index % 6) as f32 / 5.0;
                [r, g, b, 1.0]
            }
            
            // Grayscale ramp (232-255)
            232..=255 => {
                let gray = (index - 232) as f32 / 23.0;
                [gray, gray, gray, 1.0]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ansi_color_conversion() {
        assert_eq!(TerminalColor::from_ansi_code(31), TerminalColor::Red);
        assert_eq!(TerminalColor::from_ansi_code(32), TerminalColor::Green);
        assert_eq!(TerminalColor::from_ansi_code(94), TerminalColor::BrightBlue);
    }
    
    #[test]
    fn test_rgb_conversion() {
        let red = TerminalColor::Red.to_rgb();
        assert_eq!(red, [0.8, 0.0, 0.0, 1.0]);
        
        let custom = TerminalColor::Rgb { r: 255, g: 128, b: 0 }.to_rgb();
        assert_eq!(custom, [1.0, 0.5019608, 0.0, 1.0]);
    }
}
