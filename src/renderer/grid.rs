use std::collections::VecDeque;
use crate::renderer::colors::TerminalColor;

#[derive(Debug, Clone, Default)]
pub struct Cell {
    pub ch: char,
    pub fg_color: TerminalColor,
    pub bg_color: TerminalColor,
    pub attrs: CellAttributes,
}

impl Cell {
    pub fn new(ch: char) -> Self {
        Self {
            ch,
            fg_color: TerminalColor::White,
            bg_color: TerminalColor::Black,
            attrs: CellAttributes::default(),
        }
    }
    
    pub fn empty() -> Self {
        Self::default()
    }
    
    pub fn is_empty(&self) -> bool {
        self.ch == '\0' || self.ch == ' '
    }
}

#[derive(Debug, Clone, Default)]
pub struct CellAttributes {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub blink: bool,
    pub reverse: bool,
}

#[derive(Debug, Clone)]
pub struct Region {
    pub row: u16,
    pub col: u16,
    pub width: u16,
    pub height: u16,
}

pub struct TextGrid {
    pub rows: u16,
    pub cols: u16,
    cells: Vec<Vec<Cell>>,
    scrollback: VecDeque<Vec<Cell>>,
    cursor_row: u16,
    cursor_col: u16,
    cursor_visible: bool,
    dirty_regions: Vec<Region>,
    current_attrs: CellAttributes,
    current_fg: TerminalColor,
    current_bg: TerminalColor,
    scroll_region_top: u16,
    scroll_region_bottom: u16,
}

impl TextGrid {
    pub fn new(rows: u16, cols: u16) -> Self {
        let mut cells = Vec::with_capacity(rows as usize);
        for _ in 0..rows {
            cells.push(vec![Cell::empty(); cols as usize]);
        }
        
        Self {
            rows,
            cols,
            cells,
            scrollback: VecDeque::new(),
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            dirty_regions: Vec::new(),
            current_attrs: CellAttributes::default(),
            current_fg: TerminalColor::White,
            current_bg: TerminalColor::Black,
            scroll_region_top: 0,
            scroll_region_bottom: rows - 1,
        }
    }
    
    pub fn resize(&mut self, new_rows: u16, new_cols: u16) {
        if new_rows == self.rows && new_cols == self.cols {
            return;
        }
        
        // Resize existing rows
        for row in &mut self.cells {
            if new_cols > self.cols {
                // Add cells to the right
                row.extend(vec![Cell::empty(); (new_cols - self.cols) as usize]);
            } else if new_cols < self.cols {
                // Remove cells from the right
                row.truncate(new_cols as usize);
            }
        }
        
        // Add or remove rows
        if new_rows > self.rows {
            // Add rows at the bottom
            for _ in self.rows..new_rows {
                self.cells.push(vec![Cell::empty(); new_cols as usize]);
            }
        } else if new_rows < self.rows {
            // Remove rows from the bottom, move to scrollback if needed
            while self.cells.len() > new_rows as usize {
                if let Some(row) = self.cells.pop() {
                    self.scrollback.push_back(row);
                }
            }
        }
        
        self.rows = new_rows;
        self.cols = new_cols;
        self.scroll_region_bottom = new_rows - 1;
        
        // Clamp cursor position
        self.cursor_row = self.cursor_row.min(new_rows - 1);
        self.cursor_col = self.cursor_col.min(new_cols - 1);
        
        self.mark_all_dirty();
    }
    
    pub fn write_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.newline();
        }
        
        self.cells[self.cursor_row as usize][self.cursor_col as usize] = Cell {
            ch,
            fg_color: self.current_fg,
            bg_color: self.current_bg,
            attrs: self.current_attrs.clone(),
        };
        
        self.mark_dirty(self.cursor_row, self.cursor_col, 1, 1);
        self.cursor_col += 1;
        
        if self.cursor_col >= self.cols {
            self.cursor_col = self.cols - 1;
        }
    }
    
    pub fn set_char(&mut self, row: u16, col: u16, ch: char) {
        if row < self.rows && col < self.cols {
            self.cells[row as usize][col as usize] = Cell {
                ch,
                fg_color: TerminalColor::White,
                bg_color: TerminalColor::Black,
                attrs: CellAttributes::default(),
            };
            self.mark_dirty(row, col, 1, 1);
        }
    }
    
    pub fn newline(&mut self) {
        self.cursor_col = 0;
        if self.cursor_row >= self.scroll_region_bottom {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
        }
    }
    
    pub fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }
    
    pub fn tab(&mut self) {
        // Move to next tab stop (every 8 characters)
        let next_tab = ((self.cursor_col / 8) + 1) * 8;
        self.cursor_col = next_tab.min(self.cols - 1);
    }
    
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }
    
    pub fn scroll_up(&mut self, lines: u16) {
        for _ in 0..lines {
            if self.scroll_region_top < self.cells.len() as u16 {
                let top_line = self.cells.remove(self.scroll_region_top as usize);
                self.scrollback.push_back(top_line);
                
                // Insert empty line at scroll region bottom
                self.cells.insert(
                    self.scroll_region_bottom as usize,
                    vec![Cell::empty(); self.cols as usize]
                );
            }
            
            // Limit scrollback size
            if self.scrollback.len() > 10000 {
                self.scrollback.pop_front();
            }
        }
        
        self.mark_dirty(self.scroll_region_top, 0, self.cols, 
                       self.scroll_region_bottom - self.scroll_region_top + 1);
    }
    
    pub fn scroll_down(&mut self, lines: u16) {
        for _ in 0..lines {
            if self.scroll_region_bottom < self.cells.len() as u16 {
                self.cells.remove(self.scroll_region_bottom as usize);
                self.cells.insert(
                    self.scroll_region_top as usize,
                    vec![Cell::empty(); self.cols as usize]
                );
            }
        }
        
        self.mark_dirty(self.scroll_region_top, 0, self.cols,
                       self.scroll_region_bottom - self.scroll_region_top + 1);
    }
    
    // Cursor movement methods
    pub fn cursor_up(&mut self, lines: u16) {
        self.cursor_row = self.cursor_row.saturating_sub(lines).max(self.scroll_region_top);
    }
    
    pub fn cursor_down(&mut self, lines: u16) {
        self.cursor_row = (self.cursor_row + lines).min(self.scroll_region_bottom);
    }
    
    pub fn cursor_left(&mut self, cols: u16) {
        self.cursor_col = self.cursor_col.saturating_sub(cols);
    }
    
    pub fn cursor_right(&mut self, cols: u16) {
        self.cursor_col = (self.cursor_col + cols).min(self.cols - 1);
    }
    
    pub fn set_cursor(&mut self, row: u16, col: u16) {
        self.cursor_row = row.min(self.rows - 1);
        self.cursor_col = col.min(self.cols - 1);
    }
    
    pub fn cursor_position(&self) -> (u16, u16) {
        (self.cursor_row, self.cursor_col)
    }
    
    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }
    
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }
    
    // Screen clearing methods
    pub fn clear_screen(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = Cell::empty();
            }
        }
        self.mark_all_dirty();
    }
    
    pub fn clear_line(&mut self) {
        let row = &mut self.cells[self.cursor_row as usize];
        for cell in row {
            *cell = Cell::empty();
        }
        self.mark_dirty(self.cursor_row, 0, self.cols, 1);
    }
    
    pub fn clear_line_from_cursor(&mut self) {
        let row = &mut self.cells[self.cursor_row as usize];
        for i in self.cursor_col as usize..row.len() {
            row[i] = Cell::empty();
        }
        self.mark_dirty(self.cursor_row, self.cursor_col, self.cols - self.cursor_col, 1);
    }
    
    pub fn clear_line_to_cursor(&mut self) {
        let row = &mut self.cells[self.cursor_row as usize];
        for i in 0..=self.cursor_col as usize {
            if i < row.len() {
                row[i] = Cell::empty();
            }
        }
        self.mark_dirty(self.cursor_row, 0, self.cursor_col + 1, 1);
    }
    
    // Attribute and color methods
    pub fn set_attrs(&mut self, attrs: CellAttributes) {
        self.current_attrs = attrs;
    }
    
    pub fn set_fg_color(&mut self, color: TerminalColor) {
        self.current_fg = color;
    }
    
    pub fn set_bg_color(&mut self, color: TerminalColor) {
        self.current_bg = color;
    }
    
    pub fn reset_attrs(&mut self) {
        self.current_attrs = CellAttributes::default();
        self.current_fg = TerminalColor::White;
        self.current_bg = TerminalColor::Black;
    }
    
    // Scroll region methods
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        self.scroll_region_top = top.min(self.rows - 1);
        self.scroll_region_bottom = bottom.min(self.rows - 1);
        
        if self.scroll_region_top > self.scroll_region_bottom {
            std::mem::swap(&mut self.scroll_region_top, &mut self.scroll_region_bottom);
        }
    }
    
    // Dirty region tracking
    fn mark_dirty(&mut self, row: u16, col: u16, width: u16, height: u16) {
        self.dirty_regions.push(Region { row, col, width, height });
    }
    
    fn mark_all_dirty(&mut self) {
        self.dirty_regions.clear();
        self.dirty_regions.push(Region {
            row: 0,
            col: 0,
            width: self.cols,
            height: self.rows,
        });
    }
    
    pub fn take_dirty_regions(&mut self) -> Vec<Region> {
        std::mem::take(&mut self.dirty_regions)
    }
    
    pub fn is_dirty(&self) -> bool {
        !self.dirty_regions.is_empty()
    }
    
    // Access methods
    pub fn cell_at(&self, row: u16, col: u16) -> Option<&Cell> {
        self.cells
            .get(row as usize)?
            .get(col as usize)
    }
    
    pub fn set_cell(&mut self, row: u16, col: u16, cell: &Cell) {
        if let Some(row_cells) = self.cells.get_mut(row as usize) {
            if let Some(target_cell) = row_cells.get_mut(col as usize) {
                *target_cell = cell.clone();
                self.mark_dirty(row, col, 1, 1);
            }
        }
    }
    
    pub fn row(&self, index: u16) -> Option<&Vec<Cell>> {
        self.cells.get(index as usize)
    }
    
    pub fn scrollback(&self) -> &VecDeque<Vec<Cell>> {
        &self.scrollback
    }
    
    pub fn scrollback_line(&self, index: usize) -> Option<&Vec<Cell>> {
        self.scrollback.get(index)
    }
    
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_grid_creation() {
        let grid = TextGrid::new(24, 80);
        assert_eq!(grid.rows, 24);
        assert_eq!(grid.cols, 80);
        assert_eq!(grid.cursor_position(), (0, 0));
    }
    
    #[test]
    fn test_write_char() {
        let mut grid = TextGrid::new(24, 80);
        grid.write_char('H');
        grid.write_char('i');
        
        assert_eq!(grid.cell_at(0, 0).unwrap().ch, 'H');
        assert_eq!(grid.cell_at(0, 1).unwrap().ch, 'i');
        assert_eq!(grid.cursor_position(), (0, 2));
    }
    
    #[test]
    fn test_newline() {
        let mut grid = TextGrid::new(24, 80);
        grid.write_char('A');
        grid.newline();
        grid.write_char('B');
        
        assert_eq!(grid.cell_at(0, 0).unwrap().ch, 'A');
        assert_eq!(grid.cell_at(1, 0).unwrap().ch, 'B');
    }
    
    #[test]
    fn test_resize() {
        let mut grid = TextGrid::new(24, 80);
        grid.write_char('X');
        grid.resize(30, 100);
        
        assert_eq!(grid.rows, 30);
        assert_eq!(grid.cols, 100);
        assert_eq!(grid.cell_at(0, 0).unwrap().ch, 'X');
    }
}
