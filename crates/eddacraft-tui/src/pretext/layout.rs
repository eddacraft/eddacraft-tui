use ratatui::style::Style;
use textwrap::wrap_algorithms::wrap_first_fit;

use super::exclusion::{ExclusionZone, RowBand, compute_row_band, compute_row_bands};
use super::prepare::PreparedText;

/// A positioned word ready for rendering.
#[derive(Debug, Clone)]
pub struct PositionedWord {
    /// The word text.
    pub text: String,
    /// Column position of the first character.
    pub x: u16,
    /// Row position.
    pub y: u16,
    /// Display width.
    pub width: u16,
    /// Style runs: `(byte_offset_into_text, style)` ordered by offset.
    pub style_runs: Vec<(usize, Style)>,
}

impl PositionedWord {
    /// The primary (first) style of this word.
    pub fn primary_style(&self) -> Style {
        self.style_runs.first().map(|r| r.1).unwrap_or_default()
    }

    /// Iterate over styled segments of this word as `(&str, Style)` pairs.
    pub fn segments(&self) -> impl Iterator<Item = (&str, Style)> {
        let text = self.text.as_str();
        let runs = &self.style_runs;
        (0..runs.len()).filter_map(move |i| {
            let start = runs[i].0;
            let end = runs.get(i + 1).map_or(text.len(), |r| r.0);
            if start >= end {
                None
            } else {
                Some((&text[start..end], runs[i].1))
            }
        })
    }
}

/// A line of positioned words.
#[derive(Debug, Clone)]
pub struct LayoutLine {
    pub words: Vec<PositionedWord>,
    pub y: u16,
}

/// The complete result of a layout computation.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub lines: Vec<LayoutLine>,
    pub total_height: u16,
}

/// Generous upper bound on pre-computed row bands.
const MAX_ROWS_CAP: u16 = 16_384;

/// Compute layout from prepared text and constraints.
pub fn layout(
    prepared: &PreparedText,
    container_width: u16,
    exclusions: &[ExclusionZone],
) -> LayoutResult {
    layout_with_cap(prepared, container_width, exclusions, MAX_ROWS_CAP)
}

/// Internal layout entrypoint that accepts an explicit row cap.
#[allow(clippy::too_many_lines)]
pub(crate) fn layout_with_cap(
    prepared: &PreparedText,
    container_width: u16,
    exclusions: &[ExclusionZone],
    max_rows_cap: u16,
) -> LayoutResult {
    let words = prepared.words();
    if words.is_empty() {
        return LayoutResult {
            lines: Vec::new(),
            total_height: 0,
        };
    }

    let safe_width = container_width.max(1) as usize;
    let required_capacity = prepared.total_width().saturating_add(safe_width);

    let build_bands = |max_lines: u16| -> Vec<RowBand> {
        if exclusions.is_empty() {
            vec![
                RowBand {
                    left: 0,
                    width: safe_width,
                };
                max_lines as usize
            ]
        } else {
            compute_row_bands(container_width, max_lines, exclusions)
        }
    };

    let unblocked_capacity = |bands: &[RowBand]| -> usize {
        bands
            .iter()
            .filter(|b| !b.is_blocked())
            .map(|b| b.width)
            .sum()
    };

    let mut estimated_max_lines = (prepared.total_width() / safe_width + 1)
        .min(max_rows_cap as usize)
        .max(50) as u16;
    estimated_max_lines = estimated_max_lines.min(max_rows_cap);
    let mut row_bands: Vec<RowBand> = build_bands(estimated_max_lines);

    while unblocked_capacity(&row_bands) < required_capacity
        && row_bands.len() < max_rows_cap as usize
    {
        let next = ((row_bands.len() as u32) * 2)
            .max(row_bands.len() as u32 + 64)
            .min(max_rows_cap as u32) as u16;
        row_bands = build_bands(next);
    }

    let mut placement: Vec<(u16, RowBand)> = Vec::with_capacity(row_bands.len());
    let mut filtered_widths: Vec<f64> = Vec::with_capacity(row_bands.len());
    for (i, band) in row_bands.iter().enumerate() {
        if !band.is_blocked() {
            placement.push((i as u16, *band));
            filtered_widths.push(band.width as f64);
        }
    }

    let mut overflow_capacity_accum: usize =
        filtered_widths.iter().map(|w| *w as usize).sum::<usize>();
    let mut next_overflow_row: u16 = row_bands.len() as u16;
    let overflow_probe_limit: u32 = (max_rows_cap as u32).saturating_mul(4);
    let mut probed: u32 = 0;
    while overflow_capacity_accum < required_capacity && probed < overflow_probe_limit {
        let band = compute_row_band(container_width, next_overflow_row, exclusions);
        if !band.is_blocked() {
            overflow_capacity_accum += band.width;
            placement.push((next_overflow_row, band));
            filtered_widths.push(band.width as f64);
        }
        probed += 1;
        if next_overflow_row == u16::MAX {
            break;
        }
        next_overflow_row = next_overflow_row.saturating_add(1);
    }

    if filtered_widths.is_empty() {
        return LayoutResult {
            lines: Vec::new(),
            total_height: 0,
        };
    }

    let wrapped = wrap_first_fit(words, &filtered_widths);

    let mut lines = Vec::with_capacity(wrapped.len());
    let mut max_row: u16 = 0;

    for (virtual_row, line_words) in wrapped.iter().enumerate() {
        let (real_row, band) = if virtual_row < placement.len() {
            placement[virtual_row]
        } else {
            let last = placement.last().map_or(0, |(r, _)| *r);
            let extra_usize = virtual_row - placement.len() + 1;
            let extra = if extra_usize > u16::MAX as usize {
                break;
            } else {
                extra_usize as u16
            };
            let Some(real_row) = last.checked_add(extra) else {
                break;
            };
            (
                real_row,
                RowBand {
                    left: 0,
                    width: safe_width,
                },
            )
        };
        let mut positioned = Vec::with_capacity(line_words.len());
        let mut x: usize = band.left as usize;

        for word in *line_words {
            let w = word.width.min(u16::MAX as usize);
            positioned.push(PositionedWord {
                text: word.text.clone(),
                x: (x.min(u16::MAX as usize)) as u16,
                y: real_row,
                width: w as u16,
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

    let total_height = if lines.is_empty() {
        0
    } else {
        max_row.saturating_add(1)
    };
    LayoutResult {
        lines,
        total_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn repeat_words(count: u32, prefix: &str) -> String {
        let mut s = String::new();
        for i in 0..count {
            let _ = write!(s, "{prefix}{i} ");
        }
        s
    }

    #[test]
    fn test_total_height_saturates_at_u16_max() {
        let max_row: u16 = u16::MAX;
        let total_height = max_row.saturating_add(1);
        assert_eq!(total_height, u16::MAX);
    }

    #[test]
    fn test_overflow_probe_terminates_at_u16_max() {
        let text = repeat_words(50, "w");
        let prepared = PreparedText::new(&text);
        let zones = vec![ExclusionZone::rect(3, 0, 17, 100)];
        let result = layout_with_cap(&prepared, 20, &zones, 5);

        let mut seen_rows = std::collections::HashSet::new();
        for line in &result.lines {
            assert!(
                seen_rows.insert(line.y),
                "duplicate row {} found — overflow probe collapsed lines onto same y",
                line.y
            );
        }
    }

    #[test]
    fn test_layout_single_line() {
        let prepared = PreparedText::new("hello world");
        let result = layout(&prepared, 80, &[]);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].words.len(), 2);
        assert_eq!(result.lines[0].words[0].text, "hello");
        assert_eq!(result.lines[0].words[0].x, 0);
        assert_eq!(result.lines[0].words[1].text, "world");
        assert_eq!(result.lines[0].words[1].x, 6);
    }

    #[test]
    fn test_layout_wraps() {
        let prepared = PreparedText::new("hello world foo");
        let result = layout(&prepared, 10, &[]);
        assert!(result.lines.len() >= 2);
        assert_eq!(result.lines[0].words[0].text, "hello");
        assert_eq!(result.lines[1].words[0].text, "world");
        assert_eq!(result.lines[1].words[0].y, 1);
    }

    #[test]
    fn test_layout_with_exclusion() {
        let prepared = PreparedText::new("hello world foo bar baz");
        let exclusion = ExclusionZone::rect(15, 0, 5, 3);
        let result = layout(&prepared, 20, &[exclusion]);
        for line in &result.lines {
            for word in &line.words {
                if word.y < 3 {
                    assert!(
                        word.x + word.width <= 15,
                        "word '{}' at ({},{}) width {} exceeds exclusion",
                        word.text,
                        word.x,
                        word.y,
                        word.width
                    );
                }
            }
        }
    }

    #[test]
    fn test_layout_empty() {
        let prepared = PreparedText::new("");
        let result = layout(&prepared, 80, &[]);
        assert_eq!(result.lines.len(), 0);
        assert_eq!(result.total_height, 0);
    }

    #[test]
    fn test_layout_skips_blocked_rows() {
        let prepared = PreparedText::new("hello world foo bar");
        let zones = vec![
            ExclusionZone::rect(0, 0, 30, 3),
            ExclusionZone::rect(20, 0, 80, 3),
        ];
        let result = layout(&prepared, 100, &zones);
        assert!(!result.lines.is_empty());
        for line in &result.lines {
            assert!(
                line.y >= 3,
                "line at row {} should have been skipped (blocked)",
                line.y
            );
            for word in &line.words {
                assert!(word.y >= 3);
            }
        }
    }

    #[test]
    fn test_layout_overflow_narrowing_second_exclusion() {
        let text = repeat_words(150, "w");
        let prepared = PreparedText::new(&text);
        let zones = vec![
            ExclusionZone::rect(10, 0, 10, 10),
            ExclusionZone::rect(5, 10, 15, 190),
        ];
        let result = layout_with_cap(&prepared, 20, &zones, 10);

        let total_words: usize = result.lines.iter().map(|l| l.words.len()).sum();
        assert_eq!(total_words, prepared.word_count());

        for line in &result.lines {
            for word in &line.words {
                let boundary = if word.y < 10 {
                    10
                } else if word.y < 200 {
                    5
                } else {
                    20
                };
                assert!(
                    word.x + word.width <= boundary,
                    "word '{}' at x={} width={} y={} intrudes past boundary {}",
                    word.text,
                    word.x,
                    word.width,
                    word.y,
                    boundary,
                );
            }
        }
    }

    #[test]
    fn test_layout_overflow_preserves_exclusions_past_cap() {
        let text = repeat_words(200, "w");
        let prepared = PreparedText::new(&text);
        let zones = vec![ExclusionZone::rect(10, 0, 10, 100)];
        let result = layout_with_cap(&prepared, 20, &zones, 10);

        for line in &result.lines {
            for word in &line.words {
                if word.y < 100 {
                    assert!(
                        word.x + word.width <= 10,
                        "word '{}' at x={} width={} y={} intrudes into exclusion",
                        word.text,
                        word.x,
                        word.width,
                        word.y,
                    );
                }
            }
        }
        let total_words: usize = result.lines.iter().map(|l| l.words.len()).sum();
        assert_eq!(total_words, prepared.word_count());
    }

    #[test]
    fn test_layout_overflow_beyond_max_rows_cap_does_not_panic() {
        let text = repeat_words(500, "word");
        let prepared = PreparedText::new(&text);
        let zones = vec![ExclusionZone::rect(5, 0, 15, 20)];
        let result = layout_with_cap(&prepared, 20, &zones, 10);

        assert!(!result.lines.is_empty());
        let total_words: usize = result.lines.iter().map(|l| l.words.len()).sum();
        assert_eq!(total_words, prepared.word_count());
    }

    #[test]
    fn test_layout_does_not_panic_when_exclusion_narrows_rows_below_estimate() {
        let long_text = "a b c d e f g h i j k l m n o p q r s t u v w x y z \
                         A B C D E F G H I J K L M N O P Q R S T U V W X Y Z \
                         0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9";
        let prepared = PreparedText::new(long_text);
        let zones = vec![ExclusionZone::rect(2, 0, 38, 100)];
        let result = layout(&prepared, 40, &zones);

        for line in &result.lines {
            for word in &line.words {
                if word.y < 100 {
                    assert!(
                        word.x + word.width <= 2,
                        "word '{}' at x={} width={} overflows narrow band on row {}",
                        word.text,
                        word.x,
                        word.width,
                        word.y
                    );
                }
            }
        }
    }

    #[test]
    fn test_layout_respects_both_side_exclusions() {
        let prepared = PreparedText::new("one two three four five six seven eight nine ten");
        let zones = vec![
            ExclusionZone::rect(0, 0, 10, 3),
            ExclusionZone::rect(80, 0, 20, 3),
        ];
        let result = layout(&prepared, 100, &zones);
        for line in &result.lines {
            if line.y < 3 {
                for word in &line.words {
                    assert!(
                        word.x >= 10,
                        "word '{}' at x={} should be right of left exclusion",
                        word.text,
                        word.x
                    );
                    assert!(
                        word.x + word.width <= 80,
                        "word '{}' at x={} width={} extends into right exclusion",
                        word.text,
                        word.x,
                        word.width
                    );
                }
            }
        }
    }
}
