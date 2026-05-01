use crate::pretext::{CellPos, CellRect};

#[derive(Debug, Clone)]
pub enum ExclusionShape {
    Rect(CellRect),
    Circle { center: CellPos, radius: u16 },
}

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

    pub fn occupied_cols_at_row(&self, row: u16, container_width: u16) -> Option<(u16, u16)> {
        match self.shape {
            ExclusionShape::Rect(rect) => rect.contains_row(row).then(|| {
                (
                    rect.x.min(container_width),
                    rect.right().min(container_width),
                )
            }),
            ExclusionShape::Circle { center, radius } => {
                if radius == 0 {
                    return None;
                }

                let radius = f64::from(radius);
                let dy = (f64::from(row) - f64::from(center.row)).abs();
                if dy > radius {
                    return None;
                }

                let dx = (radius.mul_add(radius, -(dy * dy))).sqrt();
                let left = f64_to_u16((f64::from(center.col) - dx).floor().max(0.0));
                let right = f64_to_u16(
                    (f64::from(center.col) + dx)
                        .ceil()
                        .clamp(0.0, f64::from(container_width)),
                );

                (left < right).then_some((left, right))
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn f64_to_u16(value: f64) -> u16 {
    value.clamp(0.0, f64::from(u16::MAX)) as u16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RowBand {
    pub left: u16,
    pub width: usize,
}

impl RowBand {
    pub fn is_blocked(self) -> bool {
        self.width == 0
    }
}

pub(crate) fn compute_row_band(
    container_width: u16,
    row: u16,
    exclusions: &[ExclusionZone],
) -> RowBand {
    let ranges: Vec<(u16, u16)> = exclusions
        .iter()
        .filter_map(|zone| zone.occupied_cols_at_row(row, container_width))
        .collect();

    let mut left_edge = 0;
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

    RowBand {
        left: left_edge,
        width: usize::from(right_edge.saturating_sub(left_edge)),
    }
}

pub(crate) fn compute_row_bands(
    container_width: u16,
    max_lines: u16,
    exclusions: &[ExclusionZone],
) -> Vec<RowBand> {
    (0..max_lines)
        .map(|row| compute_row_band(container_width, row, exclusions))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_exclusion_reports_occupied_rows() {
        let zone = ExclusionZone::rect(60, 2, 20, 5);

        assert_eq!(zone.occupied_cols_at_row(0, 100), None);
        assert_eq!(zone.occupied_cols_at_row(2, 100), Some((60, 80)));
        assert_eq!(zone.occupied_cols_at_row(6, 100), Some((60, 80)));
        assert_eq!(zone.occupied_cols_at_row(7, 100), None);
    }

    #[test]
    fn circle_exclusion_reports_center_row() {
        let zone = ExclusionZone::circle(40, 10, 5);
        let (left, right) = zone.occupied_cols_at_row(10, 100).unwrap();

        assert_eq!(right - left, 10);
    }

    #[test]
    fn row_band_absorbs_overlapping_left_exclusions() {
        let zones = vec![
            ExclusionZone::rect(0, 0, 30, 3),
            ExclusionZone::rect(10, 0, 10, 3),
        ];

        let band = compute_row_band(100, 0, &zones);

        assert_eq!(band.left, 30);
        assert_eq!(band.width, 70);
    }
}
