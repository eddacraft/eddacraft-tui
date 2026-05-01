use ratatui::style::Style;
use textwrap::wrap_algorithms::wrap_first_fit;

use crate::pretext::exclusion::{RowBand, compute_row_band, compute_row_bands};
use crate::pretext::{ExclusionZone, PreparedText};

#[derive(Debug, Clone)]
pub struct PositionedWord {
    pub text: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub style_runs: Vec<(usize, Style)>,
}

impl PositionedWord {
    pub fn primary_style(&self) -> Style {
        self.style_runs.first().map(|run| run.1).unwrap_or_default()
    }

    pub fn segments(&self) -> impl Iterator<Item = (&str, Style)> {
        let text = self.text.as_str();
        let runs = &self.style_runs;
        (0..runs.len()).filter_map(move |index| {
            let start = runs[index].0;
            let end = runs.get(index + 1).map_or(text.len(), |run| run.0);
            (start < end).then_some((&text[start..end], runs[index].1))
        })
    }
}

#[derive(Debug, Clone)]
pub struct LayoutLine {
    pub words: Vec<PositionedWord>,
    pub y: u16,
}

#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub lines: Vec<LayoutLine>,
    pub total_height: u16,
}

const MAX_ROWS_CAP: u16 = 16_384;

pub fn layout(
    prepared: &PreparedText,
    container_width: u16,
    exclusions: &[ExclusionZone],
) -> LayoutResult {
    layout_with_cap(prepared, container_width, exclusions, MAX_ROWS_CAP)
}

fn layout_with_cap(
    prepared: &PreparedText,
    container_width: u16,
    exclusions: &[ExclusionZone],
    max_rows_cap: u16,
) -> LayoutResult {
    if prepared.words().is_empty() {
        return LayoutResult {
            lines: Vec::new(),
            total_height: 0,
        };
    }

    let safe_width = usize::from(container_width.max(1));
    let required_capacity = prepared.total_width().saturating_add(safe_width);
    let mut estimated_max_lines = usize_to_u16(
        (prepared.total_width() / safe_width + 1)
            .min(usize::from(max_rows_cap))
            .max(50),
    );
    estimated_max_lines = estimated_max_lines.min(max_rows_cap);

    let mut row_bands = build_bands(container_width, estimated_max_lines, exclusions, safe_width);

    while unblocked_capacity(&row_bands) < required_capacity
        && row_bands.len() < usize::from(max_rows_cap)
    {
        let current_len = u32::try_from(row_bands.len()).unwrap_or(u32::MAX);
        let next = current_len
            .saturating_mul(2)
            .max(current_len.saturating_add(64))
            .min(u32::from(max_rows_cap));
        let next = u16::try_from(next).unwrap_or(max_rows_cap);
        row_bands = build_bands(container_width, next, exclusions, safe_width);
    }

    let mut placement = Vec::with_capacity(row_bands.len());
    let mut widths = Vec::with_capacity(row_bands.len());
    for (index, band) in row_bands.iter().enumerate() {
        if !band.is_blocked() {
            placement.push((usize_to_u16(index), *band));
            widths.push(usize_to_f64(band.width));
        }
    }

    add_overflow_bands(
        container_width,
        exclusions,
        required_capacity,
        &mut placement,
        &mut widths,
        usize_to_u16(row_bands.len()),
        max_rows_cap,
    );

    if widths.is_empty() {
        return LayoutResult {
            lines: Vec::new(),
            total_height: 0,
        };
    }

    let wrapped = wrap_first_fit(prepared.words(), &widths);
    positioned_lines(&wrapped, &placement, safe_width)
}

fn build_bands(
    container_width: u16,
    max_lines: u16,
    exclusions: &[ExclusionZone],
    safe_width: usize,
) -> Vec<RowBand> {
    if exclusions.is_empty() {
        vec![
            RowBand {
                left: 0,
                width: safe_width,
            };
            usize::from(max_lines)
        ]
    } else {
        compute_row_bands(container_width, max_lines, exclusions)
    }
}

fn unblocked_capacity(bands: &[RowBand]) -> usize {
    bands
        .iter()
        .filter(|band| !band.is_blocked())
        .map(|band| band.width)
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn add_overflow_bands(
    container_width: u16,
    exclusions: &[ExclusionZone],
    required_capacity: usize,
    placement: &mut Vec<(u16, RowBand)>,
    widths: &mut Vec<f64>,
    mut next_row: u16,
    max_rows_cap: u16,
) {
    let mut accumulated = placement.iter().map(|(_, band)| band.width).sum::<usize>();
    let overflow_probe_limit = u32::from(max_rows_cap).saturating_mul(4);
    let mut probed = 0;

    while accumulated < required_capacity && probed < overflow_probe_limit {
        let band = compute_row_band(container_width, next_row, exclusions);
        if !band.is_blocked() {
            accumulated += band.width;
            placement.push((next_row, band));
            widths.push(usize_to_f64(band.width));
        }

        probed += 1;
        if next_row == u16::MAX {
            break;
        }
        next_row = next_row.saturating_add(1);
    }
}

fn positioned_lines(
    wrapped: &[&[crate::pretext::MeasuredWord]],
    placement: &[(u16, RowBand)],
    safe_width: usize,
) -> LayoutResult {
    let mut lines = Vec::with_capacity(wrapped.len());
    let mut max_row = 0;

    for (virtual_row, line_words) in wrapped.iter().enumerate() {
        let Some((real_row, band)) = placement_for_row(virtual_row, placement, safe_width) else {
            break;
        };

        let mut positioned = Vec::with_capacity(line_words.len());
        let mut x = usize::from(band.left);

        for word in *line_words {
            positioned.push(PositionedWord {
                text: word.text.clone(),
                x: usize_to_u16(x),
                y: real_row,
                width: usize_to_u16(word.width),
                style_runs: word.style_runs.clone(),
            });
            x = x
                .saturating_add(word.width)
                .saturating_add(word.whitespace_width);
        }

        lines.push(LayoutLine {
            words: positioned,
            y: real_row,
        });
        max_row = max_row.max(real_row);
    }

    LayoutResult {
        total_height: if lines.is_empty() {
            0
        } else {
            max_row.saturating_add(1)
        },
        lines,
    }
}

fn placement_for_row(
    virtual_row: usize,
    placement: &[(u16, RowBand)],
    safe_width: usize,
) -> Option<(u16, RowBand)> {
    if let Some(entry) = placement.get(virtual_row) {
        return Some(*entry);
    }

    let last = placement.last().map(|(row, _)| *row).unwrap_or_default();
    let extra = virtual_row.checked_sub(placement.len())?.checked_add(1)?;
    let extra = u16::try_from(extra).ok()?;
    Some((
        last.checked_add(extra)?,
        RowBand {
            left: 0,
            width: safe_width,
        },
    ))
}

fn usize_to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_wraps_words() {
        let prepared = PreparedText::new("hello world foo");
        let result = layout(&prepared, 10, &[]);

        assert!(result.lines.len() >= 2);
        assert_eq!(result.lines[0].words[0].text, "hello");
        assert_eq!(result.lines[1].words[0].text, "world");
    }

    #[test]
    fn layout_respects_right_side_exclusion() {
        let prepared = PreparedText::new("hello world foo bar baz");
        let exclusion = ExclusionZone::rect(15, 0, 5, 3);
        let result = layout(&prepared, 20, &[exclusion]);

        for line in &result.lines {
            for word in &line.words {
                if word.y < 3 {
                    assert!(word.x + word.width <= 15);
                }
            }
        }
    }

    #[test]
    fn layout_skips_fully_blocked_rows() {
        let prepared = PreparedText::new("hello world foo bar");
        let zones = vec![
            ExclusionZone::rect(0, 0, 30, 3),
            ExclusionZone::rect(20, 0, 80, 3),
        ];
        let result = layout(&prepared, 100, &zones);

        assert!(!result.lines.is_empty());
        assert!(result.lines.iter().all(|line| line.y >= 3));
    }
}
