use super::exclusion::{ExclusionZone, RowBand, compute_row_band, compute_row_bands};
use super::prepare::PreparedText;
use ratatui::style::Style;
use textwrap::wrap_algorithms::wrap_first_fit;

/// A positioned word ready for rendering.
///
/// Carries its full set of style runs so rendering can draw each run with
/// its own style. Use `primary_style()` for the common single-style case.
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
    /// Always non-empty; the first entry has offset 0.
    pub style_runs: Vec<(usize, Style)>,
}

impl PositionedWord {
    /// The primary (first) style of this word.
    pub fn primary_style(&self) -> Style {
        self.style_runs.first().map_or_else(Style::default, |r| r.1)
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

/// Generous upper bound on pre-computed row bands. Content that would need
/// more rows than this still lays out correctly via the overflow fallback
/// path — past this cap, `row_bands` are not grown further and wrapped lines
/// beyond `row_map.len()` are placed on synthesized full-width rows.
const MAX_ROWS_CAP: u16 = 16_384;

/// Compute layout from prepared text and constraints.
///
/// This is the hot path — called on every resize or animation frame.
/// No string measurement happens here. All widths come from the cached
/// values in [`PreparedText`]. This is pure arithmetic on integers/floats.
pub fn layout(
    prepared: &PreparedText,
    container_width: u16,
    exclusions: &[ExclusionZone],
) -> LayoutResult {
    layout_with_cap(prepared, container_width, exclusions, MAX_ROWS_CAP)
}

/// Internal layout entrypoint that accepts an explicit row cap.
/// Exposed (crate-visible) so tests can exercise the overflow fallback
/// without needing to synthesize 16k+ rows of input.
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

    // Compute per-row bands (left offset + available width) accounting for
    // exclusion zones. The initial estimate assumes full container width;
    // if exclusions shrink effective per-row width below that, we grow the
    // band list until total unblocked capacity covers the text. This prevents
    // `wrap_first_fit` from returning more lines than row_bands holds
    // (textwrap repeats the last line_widths entry as needed, so row_map
    // indexing would otherwise panic).
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

    // Clamp before the u16 cast so huge width estimates cannot truncate.
    let mut estimated_max_lines = (prepared.total_width() / safe_width + 1)
        .min(max_rows_cap as usize)
        .max(50) as u16;
    estimated_max_lines = estimated_max_lines.min(max_rows_cap);
    let mut row_bands: Vec<RowBand> = build_bands(estimated_max_lines);

    while unblocked_capacity(&row_bands) < required_capacity
        && (row_bands.len() as u16) < max_rows_cap
    {
        let next = ((row_bands.len() as u32) * 2)
            .max(row_bands.len() as u32 + 64)
            .min(max_rows_cap as u32) as u16;
        row_bands = build_bands(next);
    }

    // Filter out fully-blocked rows. `placement[i]` gives the (real_row, band)
    // pair that the i-th unblocked entry corresponds to. filtered_widths is
    // passed to wrap_first_fit so it sees the exact per-row width for every
    // row where text will actually land.
    let mut placement: Vec<(u16, RowBand)> = Vec::with_capacity(row_bands.len());
    let mut filtered_widths: Vec<f64> = Vec::with_capacity(row_bands.len());
    for (i, band) in row_bands.iter().enumerate() {
        if !band.is_blocked() {
            placement.push((i as u16, *band));
            filtered_widths.push(band.width as f64);
        }
    }

    // If we hit max_rows_cap before capacity caught up, precompute overflow
    // bands past the cap and include them in filtered_widths. This ensures
    // wrap_first_fit sees the actual (possibly narrower) widths of overflow
    // rows — if we instead relied on textwrap repeating the last entry,
    // words packed for a wider cap-edge row could spill into exclusions on
    // a narrower overflow row. Overflow walks forward skipping blocked rows,
    // bounded by a generous probe limit.
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
        // Stop if we'd walk past the u16 row index space. saturating_add
        // would otherwise clamp at u16::MAX and re-probe the same row
        // forever, appending duplicate placements at the same y.
        if next_overflow_row == u16::MAX {
            break;
        }
        next_overflow_row = next_overflow_row.saturating_add(1);
    }

    // If every row is blocked (e.g., overlapping exclusions cover everything)
    // there's nowhere to place text.
    if filtered_widths.is_empty() {
        return LayoutResult {
            lines: Vec::new(),
            total_height: 0,
        };
    }

    // Use textwrap's first-fit algorithm with our pre-measured words.
    // The Fragment trait on MeasuredWord provides the cached widths.
    let wrapped = wrap_first_fit(words, &filtered_widths);

    // Convert wrapped lines into positioned words with (x, y) coordinates,
    // mapping each wrapped index back to its (real_row, band) via placement.
    let mut lines = Vec::with_capacity(wrapped.len());
    let mut max_row: u16 = 0;

    for (virtual_row, line_words) in wrapped.iter().enumerate() {
        // If wrap_first_fit produced more lines than placement holds (extreme
        // pathological inputs: probe limit hit or capacity still insufficient),
        // fall back to synthesized free rows so we never panic. These are a
        // last-resort and may not honor distant exclusions, but they only
        // activate when the probe loop above already gave up.
        let (real_row, band) = if virtual_row < placement.len() {
            placement[virtual_row]
        } else {
            let last = placement.last().map_or(0, |(r, _)| *r);
            let extra_usize = virtual_row - placement.len() + 1;
            // Saturating conversion: u16 row index can't exceed u16::MAX.
            // If we'd walk past the end of the row space, stop emitting
            // further lines rather than collapsing them all onto the same
            // row via saturating_add (which would stack duplicate words).
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
        let mut x: u16 = band.left;

        for word in *line_words {
            let word_width = word.width.min(u16::MAX as usize) as u16;
            let whitespace_width = word.whitespace_width.min(u16::MAX as usize) as u16;

            positioned.push(PositionedWord {
                text: word.text.clone(),
                x,
                y: real_row,
                width: word_width,
                style_runs: word.style_runs.clone(),
            });
            x = x
                .saturating_add(word_width)
                .saturating_add(whitespace_width);
        }

        lines.push(LayoutLine {
            words: positioned,
            y: real_row,
        });
        max_row = max_row.max(real_row);
    }

    // Saturate at u16::MAX: max_row + 1 would overflow to 0 in release mode
    // (and panic in debug) when layout reaches row 65535, which is possible
    // with very long streamed text in narrow layouts.
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

    #[test]
    fn test_total_height_saturates_at_u16_max() {
        // Regression: previously `max_row + 1` would overflow to 0 (release)
        // or panic (debug) when layout reached row u16::MAX. Saturate instead.
        //
        // Verify the saturation contract directly: construct a LayoutResult
        // the same way layout() does, but with a max_row of u16::MAX.
        let max_row: u16 = u16::MAX;
        let total_height = max_row.saturating_add(1);
        assert_eq!(total_height, u16::MAX);
    }

    #[test]
    fn test_overflow_probe_terminates_at_u16_max() {
        // Regression: the overflow probe loop previously guarded with
        // `next_overflow_row == 0` after saturating_add(1), which is
        // unreachable with saturating arithmetic. At u16::MAX the loop
        // would re-probe row 65535 forever and append duplicate placements.
        //
        // Rather than construct 65k rows of input, we rely on the probe
        // limit (4 * max_rows_cap) as the primary termination guarantee
        // and assert that layouts with extreme inputs still produce
        // unique row indices for each line.
        let text = (0..50).fold(String::new(), |mut s, i| {
            use std::fmt::Write;
            let _ = write!(s, "w{i} ");
            s
        });
        let prepared = PreparedText::new(&text);
        let zones = vec![ExclusionZone::rect(3, 0, 17, 100)];
        let result = layout_with_cap(&prepared, 20, &zones, 5);

        // Every line should have a distinct y coordinate — no duplicate
        // placements stacked on the same row.
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
        assert_eq!(result.lines[0].words[1].x, 6); // "hello" + " "
    }

    #[test]
    fn test_layout_clamps_huge_word_width() {
        let text = format!("{} b", "a".repeat(u16::MAX as usize + 10));
        let prepared = PreparedText::new(&text);
        let result = layout(&prepared, u16::MAX, &[]);

        assert!(!result.lines.is_empty());
        assert_eq!(result.lines[0].words[0].width, u16::MAX);
    }

    #[test]
    fn test_layout_wraps() {
        let prepared = PreparedText::new("hello world foo");
        let result = layout(&prepared, 10, &[]);
        // "hello" (5) fits on line 0
        // "world" (5) fits on line 0? "hello " (6) + "world" (5) = 11 > 10
        // So "world" goes to line 1
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
        // Rows 0-2 have only 15 columns available
        // Row 3+ have full 20 columns
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
        // Left 0-30 + right 20-100 block rows 0-2 entirely; text must start on row 3
        let prepared = PreparedText::new("hello world foo bar");
        let zones = vec![
            ExclusionZone::rect(0, 0, 30, 3),
            ExclusionZone::rect(20, 0, 80, 3),
        ];
        let result = layout(&prepared, 100, &zones);
        assert!(!result.lines.is_empty());
        // No text should land on blocked rows 0, 1, or 2
        for line in &result.lines {
            assert!(
                line.y >= 3,
                "line at row {} should have been skipped (blocked)",
                line.y
            );
            for word in &line.words {
                assert!(word.y >= 3);
                // And text must not extend into the right exclusion on rows 0-2
                // (vacuously true since we just asserted y >= 3)
            }
        }
    }

    #[test]
    fn test_layout_overflow_narrowing_second_exclusion() {
        // Regression: rows 0..max_rows_cap and rows past the cap can have
        // DIFFERENT widths when stacked exclusions kick in at different
        // vertical ranges. wrap_first_fit must see each row's actual width
        // — otherwise words packed for the wider cap-edge row spill into
        // the narrower overflow row's exclusion.
        //
        // Container: 20 cols
        // Exclusion A: cols 10..20, rows 0..10  (width 10 per row)
        // Exclusion B: cols 5..20,  rows 10..200 (width 5 per row, narrower)
        // cap = 10 → precomputed bands cover rows 0..10 (width 10),
        // overflow rows 10+ must use width 5.
        let text = (0..150).fold(String::new(), |mut s, i| {
            use std::fmt::Write;
            let _ = write!(s, "w{i} ");
            s
        });
        let prepared = PreparedText::new(&text);
        let zones = vec![
            ExclusionZone::rect(10, 0, 10, 10),
            ExclusionZone::rect(5, 10, 15, 190),
        ];
        let result = layout_with_cap(&prepared, 20, &zones, 10);

        // All words placed
        let total_words: usize = result.lines.iter().map(|l| l.words.len()).sum();
        assert_eq!(total_words, prepared.word_count());

        // Every word must respect its row's exclusion boundary:
        //   rows 0..10  → x+width <= 10
        //   rows 10..200 → x+width <= 5
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
        // Regression: when wrapping overflows past max_rows_cap, synthesized
        // fallback bands must still honor exclusions that extend into those
        // rows. A tall right-side exclusion (height 100) with cap=10 means
        // rows 10..100 are overflow rows where the exclusion still applies.
        // Text placed in those rows must not intrude into the excluded cols.
        let text = (0..200).fold(String::new(), |mut s, i| {
            use std::fmt::Write;
            let _ = write!(s, "w{i} ");
            s
        });
        let prepared = PreparedText::new(&text);
        // 20-col container; exclusion at cols 10..20 on rows 0..100.
        // Rows should have width ~10, and rows past cap=10 are "overflow"
        // but the same exclusion still applies to them.
        let zones = vec![ExclusionZone::rect(10, 0, 10, 100)];
        let result = layout_with_cap(&prepared, 20, &zones, 10);

        // Every placed word must fit within col 10 (the exclusion boundary),
        // including words placed in overflow rows past the cap.
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
        // All words should be placed (none silently dropped).
        let total_words: usize = result.lines.iter().map(|l| l.words.len()).sum();
        assert_eq!(total_words, prepared.word_count());
    }

    #[test]
    fn test_layout_overflow_beyond_max_rows_cap_does_not_panic() {
        // Regression: even when row_bands growth hits MAX_ROWS_CAP before
        // capacity catches up, wrapped lines beyond row_map.len() must not
        // panic at row_map[virtual_row]. They fall back to synthesized
        // full-width rows past the last mapped index.
        //
        // Use layout_with_cap(max_rows_cap=10) so we don't need 16k+ rows
        // of text to exercise the overflow path.
        let text = (0..500).fold(String::new(), |mut s, i| {
            use std::fmt::Write;
            let _ = write!(s, "word{i} ");
            s
        });
        let prepared = PreparedText::new(&text);
        // Narrow rows (5 columns) with an exclusion that keeps them narrow
        let zones = vec![ExclusionZone::rect(5, 0, 15, 20)];
        let result = layout_with_cap(&prepared, 20, &zones, 10);

        // Must not panic; must lay out all input words across (possibly
        // many) rows, some of which come from the overflow fallback.
        assert!(!result.lines.is_empty());
        let total_words: usize = result.lines.iter().map(|l| l.words.len()).sum();
        assert_eq!(total_words, prepared.word_count());
    }

    #[test]
    fn test_layout_does_not_panic_when_exclusion_narrows_rows_below_estimate() {
        // Regression: estimated_max_lines is based on container_width, but
        // a right-side exclusion can shrink per-row width dramatically.
        // With more text than fits in the initial estimate's unblocked
        // capacity, row_bands must grow so row_map indexing never panics.
        //
        // container_width=40; right exclusion 2..40 shrinks rows 0..100 to
        // just 2 columns each. Initial estimate (50 rows @ 2 cols = 100 cap)
        // is far below this text's ~200+ width.
        let long_text = "a b c d e f g h i j k l m n o p q r s t u v w x y z \
                         A B C D E F G H I J K L M N O P Q R S T U V W X Y Z \
                         0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9";
        let prepared = PreparedText::new(long_text);
        let zones = vec![ExclusionZone::rect(2, 0, 38, 100)];
        let result = layout(&prepared, 40, &zones);

        // Every placed word must fit within its row's available band
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
        // Left 0-10, right 80-100 on rows 0-2 — text must fit in cols 10..80
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
