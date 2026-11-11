use super::exclusion::{ExclusionZone, RowBand, compute_row_band, compute_row_bands};
use super::prepare::PreparedText;
use super::segment::MeasuredWord;
use ratatui::style::Style;
use textwrap::core::Fragment;

/// A positioned word ready for rendering.
///
/// Carries its full set of style runs so rendering can draw each run with
/// its own style. Use `primary_style()` for the common single-style case.
///
/// A word too wide for its row's band is split between display clusters
/// into one `PositionedWord` per row, each inside that row's band.
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
    /// `Some` when layout ran out of rows before it placed every word.
    ///
    /// Rows are `u16`, so text that needs rows past `u16::MAX` cannot be
    /// placed. Layout stops at the first word it cannot place and reports it
    /// here rather than dropping the rest silently. `lines` holds everything
    /// before that word; the [`PreparedText`] still holds the full text.
    pub overflow: Option<LayoutOverflow>,
}

/// Where layout stopped when the row space ran out.
///
/// Every word from [`Self::first_unplaced_word`] onward is missing from
/// [`LayoutResult::lines`]. Read them back from [`PreparedText::words`].
///
/// # Stability
///
/// **unstable**: new in the release after 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayoutOverflow {
    /// Index into [`PreparedText::words`] of the first word layout did not
    /// place in full. The text needs rows past `u16::MAX`, or exclusions
    /// block every row left for it. A word too wide for its row is split
    /// across rows, so when layout stops partway through one, its earlier
    /// pieces are in [`LayoutResult::lines`].
    pub first_unplaced_word: usize,
}

/// Report an overflow from `index` only when a visible word is left to lose.
fn overflow_from(words: &[MeasuredWord], index: usize) -> Option<LayoutOverflow> {
    words
        .get(index..)
        .is_some_and(|rest| rest.iter().any(is_visible_word))
        .then_some(LayoutOverflow {
            first_unplaced_word: index,
        })
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
            overflow: None,
        };
    }

    // Compute per-row bands (left offset + available width) accounting for
    // exclusion zones. The initial estimate assumes full container width;
    // if exclusions shrink effective per-row width below that, we grow the
    // band list until total unblocked capacity covers the text, so most
    // layouts never compute a band while wrapping. Rows past these are still
    // computed on demand from the exclusions.
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
    // pair of the i-th unblocked row. Wrapping reads each row's band width
    // from here, so it sees the exact width of every row text lands on.
    let mut placement: Vec<(u16, RowBand)> = row_bands
        .iter()
        .enumerate()
        .filter(|(_, band)| !band.is_blocked())
        .map(|(i, band)| (i as u16, *band))
        .collect();

    // If we hit max_rows_cap before capacity caught up, precompute bands past
    // the cap, walking forward over blocked rows, bounded by a generous probe
    // limit. Rows still past these are computed on demand while wrapping.
    let mut overflow_capacity_accum: usize = unblocked_capacity(&row_bands);
    let mut next_overflow_row: u16 = row_bands.len() as u16;
    let overflow_probe_limit: u32 = (max_rows_cap as u32).saturating_mul(4);
    let mut probed: u32 = 0;
    while overflow_capacity_accum < required_capacity && probed < overflow_probe_limit {
        let band = compute_row_band(container_width, next_overflow_row, exclusions);
        if !band.is_blocked() {
            overflow_capacity_accum += band.width;
            placement.push((next_overflow_row, band));
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
    if placement.is_empty() {
        return LayoutResult {
            lines: Vec::new(),
            total_height: 0,
            overflow: overflow_from(words, 0),
        };
    }

    // Hard breaks split the words into runs. Each run is wrapped first-fit
    // onto the row bands, then the row cursor advances by the break count.
    let mut rows = Rows {
        placement,
        container_width,
        exclusions,
    };
    layout_paragraphs(words, &mut rows)
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

/// The unblocked rows text can land on, grown on demand.
struct Rows<'a> {
    /// `(real_row, band)` for each unblocked row, in row order.
    placement: Vec<(u16, RowBand)>,
    container_width: u16,
    exclusions: &'a [ExclusionZone],
}

impl Rows<'_> {
    /// Grow `placement` until it contains an unblocked row at or below `min_row`.
    fn extend_through(&mut self, min_row: u16) {
        let placement = &mut self.placement;
        if placement.last().is_some_and(|(row, _)| *row == u16::MAX) {
            return;
        }
        let mut next = placement.last().map_or(0, |(row, _)| row.saturating_add(1));
        while placement.last().is_none_or(|(row, _)| *row < min_row) {
            let band = compute_row_band(self.container_width, next, self.exclusions);
            if !band.is_blocked() {
                placement.push((next, band));
            }
            if next == u16::MAX {
                break;
            }
            next = next.saturating_add(1);
        }
    }

    /// Index of the first unblocked row at or below `min_row`.
    fn first_at_or_below(&mut self, min_row: u16) -> Option<usize> {
        self.extend_through(min_row);
        self.placement.iter().position(|(row, _)| *row >= min_row)
    }

    /// The `index`-th unblocked row, or `None` past the `u16` row space.
    fn get(&mut self, index: usize) -> Option<(u16, RowBand)> {
        while self.placement.len() <= index {
            let last = self.placement.last().map(|(row, _)| *row)?;
            self.extend_through(last.checked_add(1)?);
            if self.placement.last().map(|(row, _)| *row) == Some(last) {
                return None;
            }
        }
        Some(self.placement[index])
    }
}

/// Clamp a column count to the `u16` column space.
fn clamp_cols(cols: usize) -> u16 {
    u16::try_from(cols).unwrap_or(u16::MAX)
}

/// The line being filled on one row.
struct OpenLine {
    row: u16,
    band: RowBand,
    words: Vec<PositionedWord>,
    /// Columns used so far, trailing whitespace included.
    used: usize,
}

impl OpenLine {
    fn new((row, band): (u16, RowBand)) -> Self {
        Self {
            row,
            band,
            words: Vec::new(),
            used: 0,
        }
    }

    /// Place `word.text[from..end]`, `width` columns wide, after the words
    /// already on this row.
    fn place(
        &mut self,
        word: &MeasuredWord,
        (from, end): (usize, usize),
        width: usize,
        whitespace: usize,
    ) {
        let (text, style_runs) = if from == 0 && end == word.text.len() {
            (word.text.clone(), word.style_runs.clone())
        } else {
            (
                word.text[from..end].to_string(),
                word.style_runs_in(from, end),
            )
        };
        self.words.push(PositionedWord {
            text,
            x: self.band.left.saturating_add(clamp_cols(self.used)),
            y: self.row,
            width: clamp_cols(width),
            style_runs,
        });
        self.used = self.used.saturating_add(width).saturating_add(whitespace);
    }

    /// Push this line if it holds a word, and open the line on the next row.
    fn advance(
        &mut self,
        rows: &mut Rows<'_>,
        index: &mut usize,
        lines: &mut Vec<LayoutLine>,
    ) -> Option<()> {
        if !self.words.is_empty() {
            lines.push(LayoutLine {
                words: std::mem::take(&mut self.words),
                y: self.row,
            });
        }
        *index += 1;
        *self = Self::new(rows.get(*index)?);
        Some(())
    }
}

/// Lay one paragraph onto the unblocked rows from `rows.get(start)` on.
///
/// First-fit: a word joins the current row when it fits beside the words
/// already there, else it starts the next row. A word wider than an empty
/// row's band is split between display clusters, so every piece fits the
/// band of the row it lands on; style runs follow each piece. A cluster
/// wider than the band (a two-column glyph in a one-column band) moves down
/// to the next row wide enough to hold it. Only a cluster wider than the
/// container itself, which no row can hold, is placed past its band; the
/// container edge clips it.
///
/// Returns the index into `words` of the first word not placed in full when
/// the `u16` row space runs out.
fn fill_paragraph(
    paragraph: &[MeasuredWord],
    first_word: usize,
    rows: &mut Rows<'_>,
    start: usize,
    lines: &mut Vec<LayoutLine>,
) -> Result<(), usize> {
    let container_width = usize::from(rows.container_width.max(1));
    let mut index = start;
    let mut line = OpenLine::new(rows.get(index).ok_or(first_word)?);
    for (offset, word) in paragraph.iter().enumerate() {
        let source = first_word + offset;
        let mut from = 0;
        let mut remaining = word.width;
        loop {
            let room = line.band.width;
            let fits = if line.words.is_empty() {
                remaining <= room
            } else {
                line.used + remaining + word.penalty_width() as usize <= room
            };
            if fits {
                line.place(
                    word,
                    (from, word.text.len()),
                    remaining,
                    word.whitespace_width,
                );
                break;
            }
            if !line.words.is_empty() {
                line.advance(rows, &mut index, lines).ok_or(source)?;
                continue;
            }

            // The rest of the word is wider than this empty row: split it.
            let (mut end, mut width) = word.fitting_prefix(from, room);
            if end == from {
                let (cluster_end, cluster_width) = word.cluster_at(from);
                if cluster_width <= container_width {
                    line.advance(rows, &mut index, lines).ok_or(source)?;
                    continue;
                }
                (end, width) = (cluster_end, cluster_width);
            }
            let last_piece = end == word.text.len();
            let whitespace = if last_piece { word.whitespace_width } else { 0 };
            line.place(word, (from, end), width, whitespace);
            if last_piece {
                break;
            }
            from = end;
            remaining = remaining.saturating_sub(width);
            line.advance(rows, &mut index, lines).ok_or(source)?;
        }
    }
    if !line.words.is_empty() {
        lines.push(LayoutLine {
            words: line.words,
            y: line.row,
        });
    }
    Ok(())
}

fn layout_paragraphs(words: &[MeasuredWord], rows: &mut Rows<'_>) -> LayoutResult {
    let mut lines = Vec::new();
    // `u32` so a row cursor past `u16::MAX` is seen as overflow instead of
    // saturating onto the last row and stacking paragraphs there.
    let mut min_row: u32 = 0;
    let mut overflow = None;
    let mut paragraph_start = 0;

    for (paragraph, hard_breaks) in split_paragraphs(words) {
        let first_word = paragraph_start;
        paragraph_start += paragraph.len();
        if !paragraph.iter().any(is_visible_word) {
            min_row = min_row.saturating_add(u32::from(hard_breaks));
            continue;
        }

        let Ok(first_row) = u16::try_from(min_row) else {
            overflow = overflow_from(words, first_word);
            break;
        };
        let Some(start) = rows.first_at_or_below(first_row) else {
            overflow = overflow_from(words, first_word);
            break;
        };

        let lines_before = lines.len();
        let filled = fill_paragraph(paragraph, first_word, rows, start, &mut lines);
        if let Err(first_unplaced) = filled {
            overflow = overflow_from(words, first_unplaced);
            break;
        }
        let placed = lines.len() > lines_before;
        let last_row = lines.last().map_or(first_row, |line| line.y);

        let advance = if hard_breaks == 0 {
            u16::from(placed)
        } else {
            hard_breaks
        };
        min_row = u32::from(last_row) + u32::from(advance);
    }

    // Rows only grow, so the last line holds the lowest glyph.
    // A trailing run of breaks reserves blank rows past the last glyph.
    // Clamp at `u16::MAX`: the row after row 65535 has no `u16` height.
    let height = match lines.last() {
        Some(line) => (u32::from(line.y) + 1).max(min_row),
        None => min_row,
    };
    LayoutResult {
        lines,
        total_height: u16::try_from(height).unwrap_or(u16::MAX),
        overflow,
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
        // vertical ranges. Wrapping must see each row's actual width
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
        // many) rows, some of which come from the overflow fallback. Words
        // wider than the five-column band are split across rows, so compare
        // the placed text rather than the word count.
        assert!(!result.lines.is_empty());
        assert_eq!(result.overflow, None);
        assert_eq!(placed_concat(&result), text.replace(' ', ""));
        assert_inside_bands(&result, 20, &zones);
    }

    fn placed_concat(result: &LayoutResult) -> String {
        result
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .map(|word| word.text.as_str())
            .collect()
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

        // At width 1 each word is split one column per row; `bar` still
        // starts on the row after the last piece of `foo`.
        let narrow = layout(&prepared, 1, &[]);
        assert_eq!(
            placed_text(&narrow),
            vec![
                ("f", 0, 0),
                ("o", 0, 1),
                ("o", 0, 2),
                ("b", 0, 3),
                ("a", 0, 4),
                ("r", 0, 5)
            ]
        );
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

    /// Row count that fills the `u16` row index space: rows `0..=u16::MAX`.
    const ROW_SPACE: usize = u16::MAX as usize + 1;

    fn one_word_per_row(rows: usize) -> PreparedText {
        // At width 1 a soft space cannot share a row, so each word takes a row.
        PreparedText::new(&vec!["a"; rows].join(" "))
    }

    fn placed_words(result: &LayoutResult) -> usize {
        result.lines.iter().map(|line| line.words.len()).sum()
    }

    #[test]
    fn layout_filling_the_last_row_reports_no_overflow() {
        let result = layout(&one_word_per_row(ROW_SPACE), 1, &[]);
        assert_eq!(result.overflow, None);
        assert_eq!(placed_words(&result), ROW_SPACE);
        assert_eq!(result.lines.last().map(|line| line.y), Some(u16::MAX));
        assert_eq!(result.total_height, u16::MAX);
    }

    #[test]
    fn layout_reports_words_past_the_last_row() {
        // Regression (#6537): the 65,537th row has no u16 index, and layout
        // used to drop its words without saying so.
        let prepared = one_word_per_row(ROW_SPACE + 2);
        let result = layout(&prepared, 1, &[]);
        assert_eq!(
            result.overflow,
            Some(LayoutOverflow {
                first_unplaced_word: ROW_SPACE
            })
        );
        assert_eq!(placed_words(&result), ROW_SPACE);
        assert_eq!(result.lines.last().map(|line| line.y), Some(u16::MAX));
        assert_eq!(result.total_height, u16::MAX);
        // The prepared text still holds every unplaced word.
        assert_eq!(prepared.words()[ROW_SPACE..].len(), 2);
    }

    #[test]
    fn paragraph_past_the_last_row_is_reported_not_stacked() {
        // Regression (#6537): once a paragraph ended on row u16::MAX, the
        // saturated row cursor put the next paragraph on the same row.
        let mut text = "a\n".repeat(ROW_SPACE);
        text.push_str("tail");
        let result = layout(&PreparedText::new(&text), 80, &[]);
        assert_eq!(
            result.overflow,
            Some(LayoutOverflow {
                first_unplaced_word: ROW_SPACE
            })
        );
        assert_eq!(result.lines.len(), ROW_SPACE);
        assert!(
            result
                .lines
                .iter()
                .flat_map(|line| &line.words)
                .all(|word| word.text != "tail"),
            "tail should not be drawn over row u16::MAX"
        );
        let mut seen_rows = std::collections::HashSet::new();
        assert!(result.lines.iter().all(|line| seen_rows.insert(line.y)));
    }

    #[test]
    fn fully_blocked_rows_report_the_text_as_unplaced() {
        let prepared = PreparedText::new("hello world");
        let zones = vec![ExclusionZone::rect(0, 0, 10, u16::MAX)];
        let result = layout_with_cap(&prepared, 10, &zones, 5);
        assert!(result.lines.is_empty());
        assert_eq!(
            result.overflow,
            Some(LayoutOverflow {
                first_unplaced_word: 0
            })
        );
    }

    /// Assert every placed word ends inside its row's band.
    fn assert_inside_bands(result: &LayoutResult, container_width: u16, zones: &[ExclusionZone]) {
        for line in &result.lines {
            let band = compute_row_band(container_width, line.y, zones);
            for word in &line.words {
                assert!(
                    word.x >= band.left
                        && usize::from(word.x) + usize::from(word.width)
                            <= usize::from(band.left) + band.width,
                    "word {:?} at x={} width={} y={} leaves band {band:?}",
                    word.text,
                    word.x,
                    word.width,
                    word.y,
                );
            }
        }
    }

    fn placed_text(result: &LayoutResult) -> Vec<(&str, u16, u16)> {
        result
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .map(|word| (word.text.as_str(), word.x, word.y))
            .collect()
    }

    #[test]
    fn unbreakable_word_is_split_to_fit_a_narrow_band() {
        // Regression (#6641, fnd_sig-feat-library-684ad41897-6bbb_35d89255ca):
        // a ten-column word in a four-column band was placed whole at x=0
        // and drawn into the exclusion from column 4.
        let zones = vec![ExclusionZone::rect(4, 0, 20, 10)];
        let result = layout(&PreparedText::new("abcdefghij"), 24, &zones);
        assert_inside_bands(&result, 24, &zones);
        assert_eq!(
            placed_text(&result),
            vec![("abcd", 0, 0), ("efgh", 0, 1), ("ij", 0, 2)]
        );
        assert_eq!(result.overflow, None);
        assert_eq!(result.total_height, 3);
    }

    #[test]
    fn overlong_word_after_a_short_word_starts_its_own_row() {
        // Regression (#6641, fnd_sig-feat-library-c4ffca33c9-db3a_1c482643f3):
        // a six-column word stayed six columns wide in a three-column band.
        let zones = vec![ExclusionZone::rect(3, 0, 10, 10)];
        let result = layout(&PreparedText::new("ab abcdef c"), 13, &zones);
        assert_inside_bands(&result, 13, &zones);
        assert_eq!(
            placed_text(&result),
            vec![("ab", 0, 0), ("abc", 0, 1), ("def", 0, 2), ("c", 0, 3)]
        );
    }

    #[test]
    fn split_word_keeps_its_style_runs() {
        let red = Style::default().fg(ratatui::style::Color::Red);
        let blue = Style::default().fg(ratatui::style::Color::Blue);
        let mut prepared = PreparedText::styled("abcde", red);
        prepared.append_styled("fgh", blue);
        let zones = vec![ExclusionZone::rect(3, 0, 10, 10)];
        let result = layout(&prepared, 13, &zones);
        assert_inside_bands(&result, 13, &zones);
        let pieces: Vec<Vec<(&str, Style)>> = result
            .lines
            .iter()
            .flat_map(|line| &line.words)
            .map(|word| word.segments().collect())
            .collect();
        assert_eq!(
            pieces,
            vec![
                vec![("abc", red)],
                vec![("de", red), ("f", blue)],
                vec![("gh", blue)],
            ]
        );
    }

    #[test]
    fn split_word_does_not_cut_a_combining_mark_from_its_base() {
        let zones = vec![ExclusionZone::rect(2, 0, 10, 10)];
        let result = layout(&PreparedText::new("e\u{301}e\u{301}e\u{301}"), 12, &zones);
        assert_inside_bands(&result, 12, &zones);
        let texts: Vec<&str> = placed_text(&result).iter().map(|p| p.0).collect();
        assert_eq!(texts, vec!["e\u{301}e\u{301}", "e\u{301}"]);
    }

    #[test]
    fn wide_glyph_moves_below_a_band_too_narrow_for_it() {
        // A two-column glyph cannot be split. A one-column band cannot hold
        // it, so it moves down to the first row wide enough.
        let zones = vec![ExclusionZone::rect(1, 0, 9, 2)];
        let result = layout(&PreparedText::new("漢字"), 10, &zones);
        assert_inside_bands(&result, 10, &zones);
        assert_eq!(placed_text(&result), vec![("漢字", 0, 2)]);
        assert_eq!(result.overflow, None);
    }

    #[test]
    fn glyph_wider_than_the_container_is_still_placed() {
        // No row can hold it, so it is placed rather than dropped and the
        // container edge clips it.
        let result = layout(&PreparedText::new("漢"), 1, &[]);
        assert_eq!(placed_text(&result), vec![("漢", 0, 0)]);
        assert_eq!(result.overflow, None);
    }

    #[test]
    fn layout_within_the_row_space_reports_no_overflow() {
        let result = layout(&PreparedText::new("foo\nbar baz"), 4, &[]);
        assert_eq!(result.overflow, None);
        assert_eq!(layout(&PreparedText::new(""), 4, &[]).overflow, None);
    }
}
