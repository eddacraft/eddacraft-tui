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
    /// or None if this zone doesn't affect the row.
    pub fn occupied_cols_at_row(&self, row: u16) -> Option<(u16, u16)> {
        let range = match &self.shape {
            ExclusionShape::Rect(rect) => {
                if rect.contains_row(row) {
                    Some((rect.x, rect.right()))
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
                // Circle equation: x² + y² = r²  →  x = sqrt(r² - y²)
                let dx = (r * r - dy * dy).sqrt();
                let left = (center.col as f64 - dx).floor().max(0.0) as u16;
                let right = (center.col as f64 + dx).ceil() as u16;
                Some((left, right))
            }
        }?;
        // Zero-width ranges are no-ops for layout; treating them as a right
        // boundary would incorrectly shrink available width.
        let (left, right) = range;
        if left >= right {
            None
        } else {
            Some((left, right))
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
///
/// Returns a [`RowBand`] describing the usable text region for this row.
/// The algorithm:
///
/// 1. Iteratively extend `left_edge` by absorbing any exclusion whose
///    `occ_start <= left_edge` — this handles overlapping/contained
///    exclusions correctly (e.g. a zone 0..30 plus another zone 10..20
///    still yields `left_edge=30`, not a spurious right boundary at 10).
/// 2. Compute `right_edge` as the min `occ_start` of exclusions whose
///    `occ_start > left_edge` — those are the zones that actually bound
///    the right side of the usable region.
/// 3. If `right_edge <= left_edge`, the row is fully blocked (width 0).
pub fn compute_row_band(container_width: u16, row: u16, exclusions: &[ExclusionZone]) -> RowBand {
    // Pre-fetch the occupied column ranges for this row once.
    let ranges: Vec<(u16, u16)> = exclusions
        .iter()
        .filter_map(|z| z.occupied_cols_at_row(row))
        .collect();

    // Iteratively absorb any exclusion that starts at or before the current
    // left_edge. This handles overlapping/contained exclusions: e.g. given
    // zones (0..30) and (10..20), the first pass sets left_edge=30; the
    // 10..20 zone is then (correctly) ignored since it's entirely inside
    // the established left edge.
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

    // Right edge: leftmost start of any exclusion that lies strictly past
    // left_edge. Exclusions inside or at/before left_edge were already
    // absorbed in step 1 and must not contribute a spurious right boundary.
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
///
/// For each row, computes the gap between the rightmost left-anchored exclusion
/// and the leftmost right-side exclusion. Rows where these edges touch or cross
/// are marked blocked (width 0) so layout can skip them entirely rather than
/// rendering text into an excluded region.
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
///
/// Convenience wrapper around `compute_row_bands` for callers that only need
/// widths. Blocked rows (fully covered by exclusions) report width 0.
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
        assert_eq!(zone.occupied_cols_at_row(0), None);
        assert_eq!(zone.occupied_cols_at_row(2), Some((60, 80)));
        assert_eq!(zone.occupied_cols_at_row(6), Some((60, 80)));
        assert_eq!(zone.occupied_cols_at_row(7), None);
    }

    #[test]
    fn test_circle_exclusion() {
        let zone = ExclusionZone::circle(40, 10, 5);
        assert_eq!(zone.occupied_cols_at_row(3), None);
        assert!(zone.occupied_cols_at_row(10).is_some());
        // At center row, should span full diameter
        let (left, right) = zone.occupied_cols_at_row(10).unwrap();
        assert_eq!(right - left, 10); // diameter = 2*5
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
        assert_eq!(widths[0], 80); // no exclusion on row 0
        assert_eq!(widths[1], 60); // exclusion starts at col 60
        assert_eq!(widths[2], 60); // still excluded
        assert_eq!(widths[3], 80); // no exclusion
    }

    #[test]
    fn test_compute_line_widths_fully_blocked() {
        // A single exclusion covering the entire container blocks the row.
        let zones = vec![ExclusionZone::rect(0, 0, 100, 3)];
        let widths = compute_line_widths(100, 5, &zones);
        assert_eq!(widths[0], 0);
        assert_eq!(widths[1], 0);
        assert_eq!(widths[2], 0);
        assert_eq!(widths[3], 100);
    }

    #[test]
    fn test_compute_row_band_overlapping_absorbs_into_left() {
        // Regression: exclusion A covers cols 0..30 (left-anchored), exclusion B
        // covers cols 10..20 (entirely inside A). Previously the B zone was
        // misclassified as a right boundary, collapsing width to 0. It should
        // instead be absorbed into the left edge, leaving the row usable from
        // col 30 onward.
        let zones = vec![
            ExclusionZone::rect(0, 0, 30, 3),  // 0..30 left-anchored
            ExclusionZone::rect(10, 0, 10, 3), // 10..20 contained inside A
        ];
        let band = compute_row_band(100, 0, &zones);
        assert_eq!(band.left, 30);
        assert_eq!(band.width, 70);
        assert!(!band.is_blocked());
    }

    #[test]
    fn test_compute_row_band_left_extended_by_overlapping_chain() {
        // A chain of overlapping exclusions should iteratively extend left_edge:
        //   0..20 → left_edge=20
        //   15..40 → 15 <= 20, absorbed → left_edge=40
        //   35..60 → 35 <= 40, absorbed → left_edge=60
        //   80..90 → 80 > 60, becomes right boundary → right_edge=80
        // Usable region: [60, 80), width=20.
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
        // A chain extending all the way to container_width leaves no space.
        //   0..50  → left_edge=50
        //   40..100 → 40 <= 50, absorbed → left_edge=100
        //   No zones past 100 → right_edge=100 → width=0.
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
        // Left exclusion 0-10 shifts text right
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
        // Left exclusion 0-10, right exclusion 80-100 on a 100-wide container
        let zones = vec![
            ExclusionZone::rect(0, 0, 10, 3),  // left block cols 0-10
            ExclusionZone::rect(80, 0, 20, 3), // right block cols 80-100
        ];
        let widths = compute_line_widths(100, 5, &zones);
        // Rows 0-2: text fits between col 10 and col 80 = 70 columns
        assert_eq!(widths[0], 70);
        assert_eq!(widths[1], 70);
        assert_eq!(widths[2], 70);
        // Rows 3+: full width
        assert_eq!(widths[3], 100);
    }
    #[test]
    fn test_circle_radius_zero() {
        let zone = ExclusionZone::circle(40, 10, 0);
        assert_eq!(zone.occupied_cols_at_row(10), None);
    }

    #[test]
    fn test_zero_width_rect_filtered() {
        let zone = ExclusionZone::rect(40, 0, 0, 2);
        assert_eq!(zone.occupied_cols_at_row(0), None);
    }
}
