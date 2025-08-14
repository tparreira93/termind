// VT100/ANSI Terminal Parser - Phase A Week 2
// This will implement VTE parsing for terminal escape sequences

use vte::{Parser, Perform};
use crate::renderer::{TextGrid, CellAttributes, TerminalColor};

// Separate performer to avoid borrowing issues with the parser
struct ParserPerformer<'a> {
    grid: &'a mut TextGrid,
    current_attrs: &'a mut CellAttributes,
    current_fg: &'a mut TerminalColor,
    current_bg: &'a mut TerminalColor,
}

pub struct TerminalParser {
    parser: Parser,
    grid: TextGrid,
    current_attrs: CellAttributes,
    current_fg: TerminalColor,
    current_bg: TerminalColor,
}

impl TerminalParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: Parser::new(),
            grid: TextGrid::new(rows, cols),
            current_attrs: CellAttributes::default(),
            current_fg: TerminalColor::White,
            current_bg: TerminalColor::Black,
        }
    }
    
    pub fn parse(&mut self, data: &[u8]) {
        for &byte in data {
            // Create a temporary performer to avoid borrowing issues
            let mut performer = ParserPerformer {
                grid: &mut self.grid,
                current_attrs: &mut self.current_attrs,
                current_fg: &mut self.current_fg,
                current_bg: &mut self.current_bg,
            };
            self.parser.advance(&mut performer, byte);
        }
    }
    
    pub fn grid(&self) -> &TextGrid {
        &self.grid
    }
    
    pub fn grid_mut(&mut self) -> &mut TextGrid {
        &mut self.grid
    }
    
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.grid.resize(rows, cols);
    }
}

impl<'a> Perform for ParserPerformer<'a> {
    fn print(&mut self, c: char) {
        // Set current attributes and colors before writing
        self.grid.set_attrs(self.current_attrs.clone());
        self.grid.set_fg_color(*self.current_fg);
        self.grid.set_bg_color(*self.current_bg);
        
        self.grid.write_char(c);
    }
    
    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.grid.newline(),
            b'\r' => self.grid.carriage_return(),
            b'\t' => self.grid.tab(),
            b'\x08' => self.grid.backspace(), // Backspace
            _ => {} // Ignore other control characters for now
        }
    }
    
    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        // TODO: Implement hook sequences (DCS)
    }
    
    fn put(&mut self, _byte: u8) {
        // TODO: Implement put for DCS sequences
    }
    
    fn unhook(&mut self) {
        // TODO: Implement unhook for DCS sequences
    }
    
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        // TODO: Implement OSC sequences (titles, colors, etc.)
    }
    
    fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, c: char) {
        match c {
            // Cursor movement
            'A' => {
                let lines = params.iter().next().map_or(1, |p| p[0] as u16);
                self.grid.cursor_up(lines);
            }
            'B' => {
                let lines = params.iter().next().map_or(1, |p| p[0] as u16);
                self.grid.cursor_down(lines);
            }
            'C' => {
                let cols = params.iter().next().map_or(1, |p| p[0] as u16);
                self.grid.cursor_right(cols);
            }
            'D' => {
                let cols = params.iter().next().map_or(1, |p| p[0] as u16);
                self.grid.cursor_left(cols);
            }
            'H' | 'f' => {
                // Cursor position
                let mut iter = params.iter();
                let row = iter.next().map_or(1, |p| p[0] as u16).saturating_sub(1);
                let col = iter.next().map_or(1, |p| p[0] as u16).saturating_sub(1);
                self.grid.set_cursor(row, col);
            }
            
            // Screen clearing
            'J' => {
                let mode = params.iter().next().map_or(0, |p| p[0]);
                match mode {
                    0 => {} // Clear from cursor to end of screen
                    1 => {} // Clear from beginning of screen to cursor
                    2 => self.grid.clear_screen(),
                    _ => {}
                }
            }
            'K' => {
                let mode = params.iter().next().map_or(0, |p| p[0]);
                match mode {
                    0 => self.grid.clear_line_from_cursor(),
                    1 => self.grid.clear_line_to_cursor(),
                    2 => self.grid.clear_line(),
                    _ => {}
                }
            }
            
            // Scrolling
            'S' => {
                let lines = params.iter().next().map_or(1, |p| p[0] as u16);
                self.grid.scroll_up(lines);
            }
            'T' => {
                let lines = params.iter().next().map_or(1, |p| p[0] as u16);
                self.grid.scroll_down(lines);
            }
            
            // Character attributes (SGR)
            'm' => {
                self.handle_sgr(params);
            }
            
            // Cursor visibility
            'h' => {
                if let Some(param) = params.iter().next() {
                    if param[0] == 25 {
                        self.grid.set_cursor_visible(true);
                    }
                }
            }
            'l' => {
                if let Some(param) = params.iter().next() {
                    if param[0] == 25 {
                        self.grid.set_cursor_visible(false);
                    }
                }
            }
            
            _ => {
                // Ignore unhandled sequences for now
            }
        }
    }
    
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        // TODO: Implement escape sequences
    }
}

impl<'a> ParserPerformer<'a> {
    fn handle_sgr(&mut self, params: &vte::Params) {
        if params.is_empty() {
            // Reset all attributes
            *self.current_attrs = CellAttributes::default();
            *self.current_fg = TerminalColor::DefaultFg;
            *self.current_bg = TerminalColor::DefaultBg;
            return;
        }
        
        for param in params.iter() {
            match param[0] {
                // Reset
                0 => {
                    *self.current_attrs = CellAttributes::default();
                    *self.current_fg = TerminalColor::DefaultFg;
                    *self.current_bg = TerminalColor::DefaultBg;
                }
                
                // Attributes
                1 => self.current_attrs.bold = true,
                3 => self.current_attrs.italic = true,
                4 => self.current_attrs.underline = true,
                5 => self.current_attrs.blink = true,
                7 => self.current_attrs.reverse = true,
                9 => self.current_attrs.strikethrough = true,
                
                // Reset attributes
                22 => self.current_attrs.bold = false,
                23 => self.current_attrs.italic = false,
                24 => self.current_attrs.underline = false,
                25 => self.current_attrs.blink = false,
                27 => self.current_attrs.reverse = false,
                29 => self.current_attrs.strikethrough = false,
                
                // Foreground colors
                30..=37 | 90..=97 => {
                    *self.current_fg = TerminalColor::from_ansi_code(param[0] as u8);
                }
                39 => *self.current_fg = TerminalColor::DefaultFg,
                
                // Background colors
                40..=47 | 100..=107 => {
                    *self.current_bg = TerminalColor::from_ansi_code(param[0] as u8 + 10);
                }
                49 => *self.current_bg = TerminalColor::DefaultBg,
                
                // 256-color and RGB color modes
                38 => {
                    // Foreground 256-color or RGB
                    // TODO: Parse subsequent parameters
                }
                48 => {
                    // Background 256-color or RGB
                    // TODO: Parse subsequent parameters
                }
                
                _ => {
                    // Ignore unknown parameters
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parser_creation() {
        let parser = TerminalParser::new(24, 80);
        assert_eq!(parser.grid().rows, 24);
        assert_eq!(parser.grid().cols, 80);
    }
    
    #[test]
    fn test_simple_text() {
        let mut parser = TerminalParser::new(24, 80);
        parser.parse(b"Hello");
        
        assert_eq!(parser.grid().cell_at(0, 0).unwrap().ch, 'H');
        assert_eq!(parser.grid().cell_at(0, 4).unwrap().ch, 'o');
    }
    
    #[test]
    fn test_newline() {
        let mut parser = TerminalParser::new(24, 80);
        parser.parse(b"Line1\nLine2");
        
        assert_eq!(parser.grid().cell_at(0, 0).unwrap().ch, 'L');
        assert_eq!(parser.grid().cell_at(1, 0).unwrap().ch, 'L');
    }
}
