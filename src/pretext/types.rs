/// A cell position in terminal space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellPos {
    pub col: u16,
    pub row: u16,
}

/// A rectangular region in cell coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl CellRect {
    pub fn contains_row(&self, row: u16) -> bool {
        row >= self.y && row < self.y.saturating_add(self.height)
    }

    pub fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }
}
