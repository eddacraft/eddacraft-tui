//! Responsive layout adaptation — width breakpoints shared by components.
//!
//! Specs are authored once and rendered at any terminal size. Rather than each
//! component inventing its own width thresholds, they consult a single
//! [`Breakpoint`] derived from the available width, so a spec degrades
//! consistently: grids collapse to a single column when narrow, tables shed
//! overflow columns, and so on. Thresholds are chosen around the common test
//! sizes (80×24, 120×40, 200×60).

use ratatui::layout::Rect;

/// Coarse width class an area falls into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    /// Below [`Breakpoint::NARROW_WIDTH`] — phone-ish / split panes. Multi-column
    /// layouts collapse to a single column here.
    Narrow,
    /// Standard terminal width (roughly an 80–160 column window).
    Medium,
    /// Wide window — full multi-column layouts with room to spare.
    Wide,
}

impl Breakpoint {
    /// Below this width an area is [`Narrow`](Breakpoint::Narrow): two columns
    /// either side of a gap leave too few cells to be readable, so layouts
    /// collapse to a single column. Matches the web `md` breakpoint intent.
    pub const NARROW_WIDTH: u16 = 100;
    /// At or above this width an area is [`Wide`](Breakpoint::Wide).
    pub const WIDE_WIDTH: u16 = 160;
    /// Minimum readable column width used by [`max_table_columns`].
    pub const MIN_COLUMN_WIDTH: u16 = 12;

    /// Classify a width in cells.
    #[must_use]
    pub fn for_width(width: u16) -> Self {
        if width < Self::NARROW_WIDTH {
            Self::Narrow
        } else if width < Self::WIDE_WIDTH {
            Self::Medium
        } else {
            Self::Wide
        }
    }

    /// Classify an area by its width.
    #[must_use]
    pub fn for_area(area: Rect) -> Self {
        Self::for_width(area.width)
    }

    /// Whether multi-column layouts should collapse to one column.
    #[must_use]
    pub fn is_narrow(self) -> bool {
        self == Self::Narrow
    }
}

/// How many of a table's `total` columns fit in `width` before columns must be
/// dropped, assuming [`Breakpoint::MIN_COLUMN_WIDTH`] per column.
///
/// Always keeps at least one column (so a table never vanishes) and never
/// reports more than `total`. Progressive column hiding (TUIDASH-011) drops the
/// trailing columns beyond this count.
#[must_use]
pub fn max_table_columns(width: u16, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    let fit = (width / Breakpoint::MIN_COLUMN_WIDTH).max(1) as usize;
    fit.min(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_the_common_terminal_sizes() {
        assert_eq!(Breakpoint::for_width(80), Breakpoint::Narrow);
        assert_eq!(Breakpoint::for_width(120), Breakpoint::Medium);
        assert_eq!(Breakpoint::for_width(200), Breakpoint::Wide);
        assert!(Breakpoint::for_width(80).is_narrow());
        assert!(!Breakpoint::for_width(120).is_narrow());
    }

    #[test]
    fn boundaries_are_inclusive_as_documented() {
        assert_eq!(Breakpoint::for_width(99), Breakpoint::Narrow);
        assert_eq!(Breakpoint::for_width(100), Breakpoint::Medium);
        assert_eq!(Breakpoint::for_width(159), Breakpoint::Medium);
        assert_eq!(Breakpoint::for_width(160), Breakpoint::Wide);
    }

    #[test]
    fn table_columns_fit_and_clamp() {
        // 80 cols / 12 = 6 columns fit; a 4-column table keeps all 4.
        assert_eq!(max_table_columns(80, 4), 4);
        // A very wide table is capped by what fits.
        assert_eq!(max_table_columns(80, 20), 6);
        // Always at least one column, never zero for a non-empty table.
        assert_eq!(max_table_columns(4, 5), 1);
        assert_eq!(max_table_columns(200, 0), 0);
    }
}
