/// A cell position in terminal space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPos {
    pub col: u16,
    pub row: u16,
}

/// A rectangular region in cell coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl CellRect {
    pub fn contains_row(&self, row: u16) -> bool {
        let row = row as u32;
        let top = self.y as u32;
        let bottom = top + self.height as u32;
        row >= top && row < bottom
    }

    pub fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_row_includes_final_representable_row() {
        let rect = CellRect {
            x: 0,
            y: u16::MAX,
            width: 1,
            height: 1,
        };

        assert!(rect.contains_row(u16::MAX));
    }

    #[test]
    fn contains_row_uses_half_open_bounds() {
        let rect = CellRect {
            x: 0,
            y: 10,
            width: 1,
            height: 3,
        };

        assert!(!rect.contains_row(9));
        assert!(rect.contains_row(10));
        assert!(rect.contains_row(12));
        assert!(!rect.contains_row(13));
    }
}
