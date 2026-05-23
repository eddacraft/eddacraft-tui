use super::types::{CellPos, CellRect};

/// A shape that text should flow around.
#[derive(Debug, Clone)]
pub enum ExclusionShape {
    Rect(CellRect),
    Circle { center: CellPos, radius: u16 },
}

/// A region that text should avoid during layout.
#[derive(Debug, Clone)]
pub struct ExclusionZone {
    pub shape: ExclusionShape,
}

impl ExclusionZone {
    pub fn rect(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            shape: ExclusionShape::Rect(CellRect {
                x,
                y,
                width,
                height,
            }),
        }
    }

    pub fn circle(col: u16, row: u16, radius: u16) -> Self {
        Self {
            shape: ExclusionShape::Circle {
                center: CellPos { col, row },
                radius,
            },
        }
    }

    /// For a given row, compute how many columns this zone occupies
    /// and where. Returns (`occupied_start`, `occupied_end`) in column space,
    /// clamped to `container_width`, or None if this zone doesn't affect the row.
    pub fn occupied_cols_at_row(&self, row: u16, container_width: u16) -> Option<(u16, u16)> {
        match &self.shape {
            ExclusionShape::Rect(rect) => {
                if rect.contains_row(row) {
                    Some((
                        rect.x.min(container_width),
                        rect.right().min(container_width),
                    ))
                } else {
                    None
                }
            }
            ExclusionShape::Circle { center, radius } => {
                if *radius == 0 {
                    return None;
                }
                let r = *radius as f64;
                let dy = (row as f64 - center.row as f64).abs();
                if dy > r {
                    return None;
                }
                let dx = (r * r - dy * dy).sqrt();
                let left = (center.col as f64 - dx).floor().max(0.0) as u16;
                let right_f = (center.col as f64 + dx).ceil().min(container_width as f64);
                let right = (right_f.max(0.0) as u32).min(u16::MAX as u32) as u16;
                if left >= right {
                    return None;
                }
                Some((left, right))
            }
        }
    }
}

/// Per-row layout info computed from exclusion zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBand {
    /// Leftmost column available for text (past any left-anchored exclusions).
    pub left: u16,
    /// Available width in columns (0 means the row is fully blocked).
    pub width: usize,
}

impl RowBand {
    pub fn is_blocked(&self) -> bool {
        self.width == 0
    }
}

/// Compute the layout band for a single row, accounting for exclusion zones.
pub fn compute_row_band(container_width: u16, row: u16, exclusions: &[ExclusionZone]) -> RowBand {
    let ranges: Vec<(u16, u16)> = exclusions
        .iter()
        .filter_map(|z| z.occupied_cols_at_row(row, container_width))
        .collect();

    let mut left_edge: u16 = 0;
    loop {
        let new_left = ranges
            .iter()
            .filter(|(start, _)| *start <= left_edge)
            .map(|(_, end)| *end)
            .fold(left_edge, u16::max);
        if new_left == left_edge {
            break;
        }
        left_edge = new_left;
    }

    let right_edge = ranges
        .iter()
        .filter(|(start, _)| *start > left_edge)
        .map(|(start, _)| *start)
        .fold(container_width, u16::min);

    let width = right_edge.saturating_sub(left_edge) as usize;
    RowBand {
        left: left_edge,
        width,
    }
}

/// Compute per-row layout bands accounting for exclusion zones.
pub fn compute_row_bands(
    container_width: u16,
    max_lines: u16,
    exclusions: &[ExclusionZone],
) -> Vec<RowBand> {
    (0..max_lines)
        .map(|row| compute_row_band(container_width, row, exclusions))
        .collect()
}

/// Compute available line widths for each row, accounting for exclusion zones.
pub fn compute_line_widths(
    container_width: u16,
    max_lines: u16,
    exclusions: &[ExclusionZone],
) -> Vec<usize> {
    compute_row_bands(container_width, max_lines, exclusions)
        .into_iter()
        .map(|band| band.width)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_exclusion() {
        let zone = ExclusionZone::rect(60, 2, 20, 5);
        assert_eq!(zone.occupied_cols_at_row(0, 100), None);
        assert_eq!(zone.occupied_cols_at_row(2, 100), Some((60, 80)));
        assert_eq!(zone.occupied_cols_at_row(6, 100), Some((60, 80)));
        assert_eq!(zone.occupied_cols_at_row(7, 100), None);
    }

    #[test]
    fn test_circle_exclusion() {
        let zone = ExclusionZone::circle(40, 10, 5);
        assert_eq!(zone.occupied_cols_at_row(3, 100), None);
        assert!(zone.occupied_cols_at_row(10, 100).is_some());
        let (left, right) = zone.occupied_cols_at_row(10, 100).unwrap();
        assert_eq!(right - left, 10);
    }

    #[test]
    fn test_circle_radius_zero() {
        let zone = ExclusionZone::circle(40, 10, 0);
        assert_eq!(zone.occupied_cols_at_row(10, 100), None);
    }

    #[test]
    fn test_rect_clamped_to_container() {
        let zone = ExclusionZone::rect(40, 0, 30, 2);
        assert_eq!(zone.occupied_cols_at_row(0, 50), Some((40, 50)));
    }

    #[test]
    fn test_circle_clamped_to_container() {
        let zone = ExclusionZone::circle(48, 5, 5);
        let result = zone.occupied_cols_at_row(5, 50);
        assert!(result.is_some());
        let (_, right) = result.unwrap();
        assert!(
            right <= 50,
            "right {right} should be clamped to container 50"
        );
    }

    #[test]
    fn test_compute_line_widths_no_exclusions() {
        let widths = compute_line_widths(80, 5, &[]);
        assert_eq!(widths, vec![80, 80, 80, 80, 80]);
    }

    #[test]
    fn test_compute_line_widths_with_rect() {
        let zones = vec![ExclusionZone::rect(60, 1, 20, 2)];
        let widths = compute_line_widths(80, 5, &zones);
        assert_eq!(widths[0], 80);
        assert_eq!(widths[1], 60);
        assert_eq!(widths[2], 60);
        assert_eq!(widths[3], 80);
    }

    #[test]
    fn test_compute_line_widths_fully_blocked() {
        let zones = vec![ExclusionZone::rect(0, 0, 100, 3)];
        let widths = compute_line_widths(100, 5, &zones);
        assert_eq!(widths[0], 0);
        assert_eq!(widths[1], 0);
        assert_eq!(widths[2], 0);
        assert_eq!(widths[3], 100);
    }

    #[test]
    fn test_compute_row_band_overlapping_absorbs_into_left() {
        let zones = vec![
            ExclusionZone::rect(0, 0, 30, 3),
            ExclusionZone::rect(10, 0, 10, 3),
        ];
        let band = compute_row_band(100, 0, &zones);
        assert_eq!(band.left, 30);
        assert_eq!(band.width, 70);
        assert!(!band.is_blocked());
    }

    #[test]
    fn test_compute_row_band_left_extended_by_overlapping_chain() {
        let zones = vec![
            ExclusionZone::rect(0, 0, 20, 2),
            ExclusionZone::rect(15, 0, 25, 2),
            ExclusionZone::rect(35, 0, 25, 2),
            ExclusionZone::rect(80, 0, 10, 2),
        ];
        let band = compute_row_band(100, 0, &zones);
        assert_eq!(band.left, 60);
        assert_eq!(band.width, 20);
    }

    #[test]
    fn test_compute_row_band_truly_blocked_by_overlapping_chain() {
        let zones = vec![
            ExclusionZone::rect(0, 0, 50, 2),
            ExclusionZone::rect(40, 0, 60, 2),
        ];
        let band = compute_row_band(100, 0, &zones);
        assert_eq!(band.left, 100);
        assert_eq!(band.width, 0);
        assert!(band.is_blocked());
    }

    #[test]
    fn test_row_band_left_offset() {
        let zones = vec![ExclusionZone::rect(0, 0, 10, 2)];
        let bands = compute_row_bands(80, 3, &zones);
        assert_eq!(bands[0].left, 10);
        assert_eq!(bands[0].width, 70);
        assert!(!bands[0].is_blocked());
        assert_eq!(bands[2].left, 0);
        assert_eq!(bands[2].width, 80);
    }

    #[test]
    fn test_compute_line_widths_both_sides() {
        let zones = vec![
            ExclusionZone::rect(0, 0, 10, 3),
            ExclusionZone::rect(80, 0, 20, 3),
        ];
        let widths = compute_line_widths(100, 5, &zones);
        assert_eq!(widths[0], 70);
        assert_eq!(widths[1], 70);
        assert_eq!(widths[2], 70);
        assert_eq!(widths[3], 100);
    }
}
