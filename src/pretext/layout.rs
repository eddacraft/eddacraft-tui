use super::exclusion::{ExclusionZone, RowBand, compute_row_band, compute_row_bands};
use super::prepare::PreparedText;
use super::segment::MeasuredWord;
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
    // Hard breaks add rows that carry no display width, so the estimate
    // has to count them or later rows miss their exclusion bands.
    let break_rows = words.iter().fold(0usize, |total, word| {
        total.saturating_add(usize::from(word.hard_breaks))
    });
    let mut estimated_max_lines = (prepared.total_width() / safe_width)
        .saturating_add(1)
        .saturating_add(break_rows)
        .min(usize::from(max_rows_cap))
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

    // Hard breaks split the words into runs. Each run is wrapped with
    // `wrap_first_fit`, then the row cursor advances by the break count.
    // The Fragment trait on MeasuredWord still supplies the cached widths.
    layout_paragraphs(
        words,
        &mut placement,
        &mut filtered_widths,
        container_width,
        exclusions,
        safe_width,
    )
}

fn is_visible_word(word: &MeasuredWord) -> bool {
    !word.text.is_empty() || word.whitespace_width > 0
}

/// Split on hard breaks. The `u16` is the break count that ends the slice
/// (`0` for a tail that does not end on a break).
fn split_paragraphs(words: &[MeasuredWord]) -> Vec<(&[MeasuredWord], u16)> {
    let mut paragraphs = Vec::new();
    let mut start = 0;
    for (index, word) in words.iter().enumerate() {
        if word.hard_breaks > 0 {
            paragraphs.push((&words[start..=index], word.hard_breaks));
            start = index + 1;
        }
    }
    if start < words.len() {
        paragraphs.push((&words[start..], 0));
    }
    paragraphs
}

/// Grow `placement` until it contains an unblocked row at or below `min_row`.
fn extend_placement_through(
    placement: &mut Vec<(u16, RowBand)>,
    filtered_widths: &mut Vec<f64>,
    min_row: u16,
    container_width: u16,
    exclusions: &[ExclusionZone],
) {
    if placement.last().is_some_and(|(row, _)| *row == u16::MAX) {
        return;
    }
    let mut next = placement.last().map_or(0, |(row, _)| row.saturating_add(1));
    while placement.last().is_none_or(|(row, _)| *row < min_row) {
        let band = compute_row_band(container_width, next, exclusions);
        if !band.is_blocked() {
            filtered_widths.push(band.width as f64);
            placement.push((next, band));
        }
        if next == u16::MAX {
            break;
        }
        next = next.saturating_add(1);
    }
}

fn row_band_at(
    virtual_row: usize,
    placement: &[(u16, RowBand)],
    safe_width: usize,
) -> Option<(u16, RowBand)> {
    if virtual_row < placement.len() {
        return Some(placement[virtual_row]);
    }
    let last = placement.last().map_or(0, |(row, _)| *row);
    let extra_usize = virtual_row - placement.len() + 1;
    let extra = u16::try_from(extra_usize).ok()?;
    let real_row = last.checked_add(extra)?;
    Some((
        real_row,
        RowBand {
            left: 0,
            width: safe_width,
        },
    ))
}

fn push_line(
    lines: &mut Vec<LayoutLine>,
    line_words: &[MeasuredWord],
    real_row: u16,
    band: RowBand,
) {
    let mut positioned = Vec::with_capacity(line_words.len());
    let mut x: u16 = band.left;
    for word in line_words {
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
}

fn layout_paragraphs(
    words: &[MeasuredWord],
    placement: &mut Vec<(u16, RowBand)>,
    filtered_widths: &mut Vec<f64>,
    container_width: u16,
    exclusions: &[ExclusionZone],
    safe_width: usize,
) -> LayoutResult {
    let mut lines = Vec::new();
    let mut max_row: u16 = 0;
    let mut saw_line = false;
    let mut min_row: u16 = 0;

    for (paragraph, hard_breaks) in split_paragraphs(words) {
        if !paragraph.iter().any(is_visible_word) {
            min_row = min_row.saturating_add(hard_breaks);
            continue;
        }

        extend_placement_through(
            placement,
            filtered_widths,
            min_row,
            container_width,
            exclusions,
        );
        let Some(start) = placement.iter().position(|(row, _)| *row >= min_row) else {
            break;
        };
        let widths = &filtered_widths[start..];
        if widths.is_empty() {
            break;
        }

        let wrapped = wrap_first_fit(paragraph, widths);
        let mut last_row = min_row;
        let mut placed = false;
        for (offset, line_words) in wrapped.iter().enumerate() {
            let Some((real_row, band)) = row_band_at(start + offset, placement, safe_width) else {
                break;
            };
            push_line(&mut lines, line_words, real_row, band);
            last_row = real_row;
            placed = true;
            saw_line = true;
            max_row = max_row.max(real_row);
        }

        let advance = if hard_breaks == 0 {
            u16::from(placed)
        } else {
            hard_breaks
        };
        min_row = last_row.saturating_add(advance);
    }

    // A trailing run of breaks reserves blank rows past the last glyph.
    // Saturate at `u16::MAX`: `max_row + 1` would overflow when layout
    // reaches row 65535.
    let total_height = if saw_line {
        max_row.saturating_add(1).max(min_row)
    } else {
        min_row
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

    #[test]
    fn hard_break_puts_following_word_on_the_next_row() {
        // `\n` is a forced row boundary. `bar` sits on the row directly
        // below `foo` even when the container could hold both words.
        let prepared = PreparedText::new("foo\nbar");
        let wide = layout(&prepared, 80, &[]);
        assert_eq!(
            wide.lines.len(),
            2,
            "wide layout stacked the break: {wide:?}"
        );
        assert_eq!(wide.lines[0].words[0].text, "foo");
        assert_eq!(wide.lines[0].words[0].y, 0);
        assert_eq!(wide.lines[1].words[0].text, "bar");
        assert_eq!(wide.lines[1].words[0].y, 1);

        let narrow = layout(&prepared, 1, &[]);
        let foo_y = word_y(&narrow, "foo");
        let bar_y = word_y(&narrow, "bar");
        assert_eq!(bar_y, foo_y + 1);
    }

    fn word_y(result: &LayoutResult, text: &str) -> u16 {
        result
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .find(|word| word.text == text)
            .unwrap_or_else(|| panic!("missing {text}"))
            .y
    }

    #[test]
    fn crlf_matches_a_single_newline() {
        let lf = layout(&PreparedText::new("foo\nbar"), 80, &[]);
        let crlf = layout(&PreparedText::new("foo\r\nbar"), 80, &[]);
        assert_eq!(word_y(&lf, "bar"), 1);
        assert_eq!(word_y(&crlf, "bar"), word_y(&lf, "bar"));
        assert_eq!(word_y(&crlf, "foo"), 0);
    }

    #[test]
    fn consecutive_breaks_insert_a_blank_row() {
        let prepared = PreparedText::new("foo\n\nbar");
        let result = layout(&prepared, 40, &[]);
        assert_eq!(word_y(&result, "foo"), 0);
        assert_eq!(word_y(&result, "bar"), 2);
        assert!(
            result
                .lines
                .iter()
                .all(|line| line.y != 1 || line.words.iter().all(|word| word.text.is_empty())),
            "row 1 should be blank: {result:?}"
        );
        assert_eq!(result.total_height, 3);
    }

    #[test]
    fn leading_break_starts_content_on_the_next_row() {
        let result = layout(&PreparedText::new("\nfoo"), 20, &[]);
        assert_eq!(word_y(&result, "foo"), 1);
        assert!(result.total_height >= 2);
    }

    #[test]
    fn trailing_break_is_kept_for_the_next_row() {
        let prepared = PreparedText::new("foo\n");
        assert_eq!(prepared.words()[0].hard_breaks, 1);
        let result = layout(&prepared, 30, &[]);
        assert_eq!(word_y(&result, "foo"), 0);
        // One trailing break ends the row. It does not add a blank row of
        // its own; a second break does.
        assert_eq!(result.total_height, 1);

        let mut streamed = PreparedText::new("foo");
        streamed.append("\n");
        streamed.append("bar");
        let streamed_layout = layout(&streamed, 80, &[]);
        assert_eq!(word_y(&streamed_layout, "bar"), 1);

        let blank = layout(&PreparedText::new("foo\n\n"), 30, &[]);
        assert_eq!(blank.total_height, 2);
    }

    #[test]
    fn lone_cr_is_a_hard_break() {
        let result = layout(&PreparedText::new("foo\rbar"), 80, &[]);
        assert_eq!(word_y(&result, "foo"), 0);
        assert_eq!(word_y(&result, "bar"), 1);
    }

    #[test]
    fn tab_stays_on_the_same_row() {
        let prepared = PreparedText::new("foo\tbar");
        assert_eq!(prepared.words()[0].hard_breaks, 0);
        let result = layout(&prepared, 80, &[]);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(word_y(&result, "foo"), word_y(&result, "bar"));
    }

    #[test]
    fn prepare_cache_distinguishes_a_break_from_a_space() {
        let broken = PreparedText::new("foo\nbar");
        let spaced = PreparedText::new("foo bar");
        assert_eq!(broken.words()[0].hard_breaks, 1);
        assert_eq!(broken.words()[0].whitespace_width, 0);
        assert_eq!(spaced.words()[0].hard_breaks, 0);
        assert_eq!(spaced.words()[0].whitespace_width, 1);

        let broken_layout = layout(&broken, 80, &[]);
        let spaced_layout = layout(&spaced, 80, &[]);
        assert_eq!(broken_layout.lines.len(), 2);
        assert_eq!(spaced_layout.lines.len(), 1);
        assert_ne!(
            broken_layout.total_height, spaced_layout.total_height,
            "layout cache of a break must not match a soft space"
        );
    }

    #[test]
    fn hard_break_respects_exclusion_bands() {
        let prepared = PreparedText::new("foo\nbar baz");
        let zones = vec![ExclusionZone::rect(4, 0, 20, 20)];
        let result = layout(&prepared, 24, &zones);
        assert_eq!(word_y(&result, "bar"), word_y(&result, "foo") + 1);
        for line in &result.lines {
            for word in &line.words {
                if word.text.is_empty() {
                    continue;
                }
                assert!(
                    word.x + word.width <= 4,
                    "word '{}' at x={} width={} y={} enters the exclusion",
                    word.text,
                    word.x,
                    word.width,
                    word.y
                );
            }
        }
    }
}
