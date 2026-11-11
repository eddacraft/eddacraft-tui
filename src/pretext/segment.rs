use std::iter::Peekable;
use std::str::CharIndices;

use ratatui::style::Style;
use textwrap::core::Fragment;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

/// A word-level fragment with pre-measured display width and style runs.
/// Implements textwrap's Fragment trait so it plugs directly into wrap algorithms.
///
/// A word may carry multiple styles (style runs) when streaming appends split
/// mid-word across a style transition. Each run is a `(byte_offset, style)`
/// entry; the run at index i applies from `style_runs[i].0` until
/// `style_runs[i+1].0` (or the end of `text`). The first run always starts at 0.
#[derive(Debug, Clone)]
pub struct MeasuredWord {
    /// The word text (no trailing whitespace).
    pub text: String,
    /// Display width of the word content (from unicode-width).
    pub width: usize,
    /// Display width of trailing soft whitespace.
    ///
    /// Hard breaks are not included here. See [`Self::hard_breaks`].
    pub whitespace_width: usize,
    /// Forced row boundaries after this word.
    ///
    /// `0` keeps the next word on this row. `1` puts it on the next row.
    /// Each extra boundary inserts a blank row. This is part of the prepare
    /// cache: layout reads it instead of [`Self::whitespace_width`], so a
    /// hard break cannot alias a soft space of the same display width.
    pub hard_breaks: u16,
    /// Penalty string (e.g. "-" for hyphenation). Empty for normal words.
    pub penalty: String,
    /// Style runs: `(byte_offset_into_text, style)` ordered by offset.
    /// Always non-empty; first entry has offset 0.
    pub style_runs: Vec<(usize, Style)>,
}

impl MeasuredWord {
    /// Measure a word and its trailing whitespace with a single style.
    pub fn new(word: &str, trailing_whitespace: &str, style: Style) -> Self {
        Self {
            width: UnicodeWidthStr::width(word),
            whitespace_width: UnicodeWidthStr::width(trailing_whitespace),
            hard_breaks: 0,
            text: word.to_string(),
            penalty: String::new(),
            style_runs: vec![(0, style)],
        }
    }

    /// The primary (first) style of this word. Convenience for single-style words.
    pub fn primary_style(&self) -> Style {
        self.style_runs.first().map_or_else(Style::default, |r| r.1)
    }

    /// Iterate over styled segments of this word as `(&str, Style)` pairs.
    /// Empty segments are skipped.
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

    /// Append another word's text and style runs to this word.
    /// Used when the boundary merge detects a continuation mid-word.
    pub(crate) fn append_fragment(&mut self, other: &MeasuredWord) {
        let base = self.text.len();
        // Merge style runs: if the incoming first run matches our last run's
        // style, we don't need a new run; otherwise push a new run at `base`.
        let last_style = self.style_runs.last().expect("style_runs never empty").1;
        for (off, style) in &other.style_runs {
            let merged_off = base + off;
            if *off == 0 && *style == last_style {
                // Same style as current trailing run — no new run needed
                continue;
            }
            self.style_runs.push((merged_off, *style));
        }
        self.text.push_str(&other.text);
        self.width += other.width;
        self.whitespace_width = other.whitespace_width;
        self.hard_breaks = self.hard_breaks.saturating_add(other.hard_breaks);
    }

    /// The widest prefix of `text[from..]` that fits in `max_width`
    /// columns, cut only between display clusters.
    ///
    /// Returns `(end, width)`: the byte offset where the prefix ends and its
    /// display width, summed cluster by cluster. `end == from` when the first
    /// cluster is wider than `max_width`. `from` must be a cluster boundary.
    pub(crate) fn fitting_prefix(&self, from: usize, max_width: usize) -> (usize, usize) {
        let mut end = from;
        let mut width = 0usize;
        while end < self.text.len() {
            let (next, cluster_width) = self.cluster_at(end);
            if width.saturating_add(cluster_width) > max_width {
                break;
            }
            end = next;
            width += cluster_width;
        }
        (end, width)
    }

    /// The display cluster that starts at `from`, as `(end, width)`.
    pub(crate) fn cluster_at(&self, from: usize) -> (usize, usize) {
        let end = cluster_end(&self.text, from);
        (end, UnicodeWidthStr::width(&self.text[from..end]))
    }

    /// Display width of `text[from..]`, summed cluster by cluster.
    ///
    /// This is the measure [`Self::fitting_prefix`] uses, so a word split
    /// into pieces has piece widths that add up to this total. `from` must
    /// be a cluster boundary.
    pub(crate) fn cluster_width_from(&self, from: usize) -> usize {
        self.text[from..]
            .graphemes(true)
            .map(UnicodeWidthStr::width)
            .fold(0, usize::saturating_add)
    }

    /// The style runs that cover `text[start..end]`, rebased so the first
    /// run starts at offset 0.
    ///
    /// `start` and `end` must be cluster boundaries. Every returned offset
    /// is a cluster boundary too: a display cluster draws whole in the style
    /// active at its first character, and a style change inside a cluster
    /// (a stream split mid ZWJ sequence or before an emoji modifier) takes
    /// effect at the next cluster.
    ///
    /// Only the runs that start inside the slice are visited: the run
    /// active at `start` is found by binary search.
    pub(crate) fn style_runs_in(&self, start: usize, end: usize) -> Vec<(usize, Style)> {
        let runs = &self.style_runs;
        let active = runs
            .partition_point(|(offset, _)| *offset <= start)
            .saturating_sub(1);
        let mut sliced = vec![(0, runs[active].1)];
        for &(offset, style) in runs[active + 1..]
            .iter()
            .take_while(|(offset, _)| *offset < end)
        {
            let offset = cluster_boundary_at_or_after(&self.text, offset);
            if offset >= end {
                break;
            }
            let offset = offset - start;
            match sliced.last_mut() {
                // Several changes inside one cluster: the last one is the
                // style active once the cluster ends.
                Some(last) if last.0 == offset => last.1 = style,
                _ => sliced.push((offset, style)),
            }
        }
        sliced.dedup_by(|next, previous| next.1 == previous.1);
        sliced
    }
}

/// Byte offset where the display cluster starting at `start` ends.
///
/// A display cluster is an extended grapheme cluster (UAX #29): a base
/// character with its combining marks and variation selectors, an emoji
/// with its skin-tone modifier, or a whole emoji ZWJ sequence. Splitting a
/// word only between clusters never strands a mark or modifier from its
/// base or cuts a ZWJ sequence apart. The terminal renderer draws text
/// cluster by cluster too.
fn cluster_end(text: &str, start: usize) -> usize {
    GraphemeCursor::new(start, text.len(), true)
        .next_boundary(text, 0)
        .ok()
        .flatten()
        .unwrap_or(text.len())
}

/// `offset` if it is a display cluster boundary of `text`, else the end of
/// the cluster it falls inside.
fn cluster_boundary_at_or_after(text: &str, offset: usize) -> usize {
    let mut cursor = GraphemeCursor::new(offset, text.len(), true);
    if cursor.is_boundary(text, 0).unwrap_or(true) {
        return offset;
    }
    cursor
        .next_boundary(text, 0)
        .ok()
        .flatten()
        .unwrap_or(text.len())
}

impl Fragment for MeasuredWord {
    fn width(&self) -> f64 {
        self.width as f64
    }

    fn whitespace_width(&self) -> f64 {
        self.whitespace_width as f64
    }

    fn penalty_width(&self) -> f64 {
        UnicodeWidthStr::width(self.penalty.as_str()) as f64
    }
}

/// Split text into [`MeasuredWord`]s with a uniform style.
///
/// # Hard and soft whitespace
///
/// Hard breaks are forced row boundaries. They do not add to
/// [`MeasuredWord::whitespace_width`].
///
/// - `\n` (LF) is one break.
/// - `\r\n` (CRLF) is one break. A stream that splits the `\r` and the
///   `\n` across two appends is still one break: the prepare cache holds
///   the trailing CR until the next chunk arrives.
/// - A lone `\r` (CR not followed by LF) is one break. A legacy Mac line
///   ending stays a row boundary rather than zero-width glue between
///   words. A CR at the end of a chunk is held so a following LF does not
///   become a second break.
///
/// Soft whitespace stays on the row. `\t` and every other Unicode white
/// space (`char::is_whitespace`, including vertical tab, form feed, and
/// U+2028 LINE SEPARATOR / U+2029 PARAGRAPH SEPARATOR) only contribute
/// display width. Tab stops are not expanded.
///
/// [`MeasuredWord::hard_breaks`] is part of the prepare cache. Layout
/// reads it when it chooses rows, so text with a break cannot share a
/// layout with the same words separated by a soft space.
///
/// Each word includes its trailing soft-whitespace measurement.
pub fn measure_words(text: &str, style: Style) -> Vec<MeasuredWord> {
    measure_words_pending(text, style).0
}

/// Measure `text`, and report whether it ended on a CR that the next chunk
/// may still pair with a following LF.
pub(crate) fn measure_words_pending(text: &str, style: Style) -> (Vec<MeasuredWord>, bool) {
    let mut words = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut pending_cr = false;

    while let Some(&(_, ch)) = chars.peek() {
        if ch == '\n' || ch == '\r' {
            let breaks = consume_hard_breaks(&mut chars, &mut pending_cr);
            add_hard_breaks(&mut words, style, breaks);
            continue;
        }
        if ch.is_whitespace() {
            let ws = consume_soft_ws(text, &mut chars);
            add_soft_ws(&mut words, style, ws);
            continue;
        }
        let word = consume_word(text, &mut chars);
        let trailing = consume_soft_ws(text, &mut chars);
        words.push(MeasuredWord::new(word, trailing, style));
    }

    (words, pending_cr)
}

fn consume_word<'a>(text: &'a str, chars: &mut Peekable<CharIndices<'a>>) -> &'a str {
    let Some(&(start, _)) = chars.peek() else {
        return "";
    };
    let mut end = start;
    while let Some(&(index, ch)) = chars.peek() {
        if ch.is_whitespace() {
            break;
        }
        chars.next();
        end = index + ch.len_utf8();
    }
    &text[start..end]
}

fn consume_soft_ws<'a>(text: &'a str, chars: &mut Peekable<CharIndices<'a>>) -> &'a str {
    let Some(&(start, ch)) = chars.peek() else {
        return "";
    };
    if ch == '\n' || ch == '\r' || !ch.is_whitespace() {
        return "";
    }
    let mut end = start;
    while let Some(&(index, ch)) = chars.peek() {
        if ch == '\n' || ch == '\r' || !ch.is_whitespace() {
            break;
        }
        chars.next();
        end = index + ch.len_utf8();
    }
    &text[start..end]
}

/// Count a run of hard breaks. A trailing CR sets `pending_cr` so the next
/// chunk can complete one CRLF instead of adding another break.
fn consume_hard_breaks(chars: &mut Peekable<CharIndices<'_>>, pending_cr: &mut bool) -> u16 {
    let mut count: u16 = 0;
    while let Some(&(_, ch)) = chars.peek() {
        if ch == '\r' {
            chars.next();
            if chars.peek().is_some_and(|(_, next)| *next == '\n') {
                chars.next();
                count = count.saturating_add(1);
                *pending_cr = false;
            } else if chars.peek().is_none() {
                count = count.saturating_add(1);
                *pending_cr = true;
                break;
            } else {
                count = count.saturating_add(1);
                *pending_cr = false;
            }
        } else if ch == '\n' {
            chars.next();
            count = count.saturating_add(1);
            *pending_cr = false;
        } else {
            break;
        }
    }
    count
}

fn add_hard_breaks(words: &mut Vec<MeasuredWord>, style: Style, count: u16) {
    if count == 0 {
        return;
    }
    if let Some(last) = words.last_mut() {
        last.hard_breaks = last.hard_breaks.saturating_add(count);
        return;
    }
    words.push(MeasuredWord {
        text: String::new(),
        width: 0,
        whitespace_width: 0,
        hard_breaks: count,
        penalty: String::new(),
        style_runs: vec![(0, style)],
    });
}

fn add_soft_ws(words: &mut Vec<MeasuredWord>, style: Style, ws: &str) {
    if ws.is_empty() {
        return;
    }
    let width = UnicodeWidthStr::width(ws);
    if let Some(last) = words.last_mut()
        && last.hard_breaks == 0
    {
        last.whitespace_width += width;
        return;
    }
    words.push(MeasuredWord {
        text: String::new(),
        width: 0,
        whitespace_width: width,
        hard_breaks: 0,
        penalty: String::new(),
        style_runs: vec![(0, style)],
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitting_prefix_stops_at_the_last_whole_cluster() {
        let word = MeasuredWord::new("ab漢c", "", Style::default());
        assert_eq!(word.fitting_prefix(0, 3), (2, 2));
        assert_eq!(word.fitting_prefix(0, 4), (5, 4));
        assert_eq!(word.fitting_prefix(2, 1), (2, 0));
        assert_eq!(word.cluster_at(2), (5, 2));
    }

    #[test]
    fn clusters_keep_marks_and_zwj_sequences_whole() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let text = format!("e\u{301}{family}x");
        assert_eq!(cluster_end(&text, 0), "e\u{301}".len());
        assert_eq!(
            cluster_end(&text, "e\u{301}".len()),
            "e\u{301}".len() + family.len()
        );
    }

    #[test]
    fn clusters_keep_an_emoji_modifier_with_its_base() {
        // Review regression (#6641): the skin-tone modifier is two columns
        // wide on its own, so a width-based rule split it from its base.
        let thumbs = "\u{1F44D}\u{1F3FD}";
        let word = MeasuredWord::new(&format!("{thumbs}ab"), "", Style::default());
        assert_eq!(word.cluster_at(0), (thumbs.len(), 2));
        assert_eq!(word.fitting_prefix(0, 3), (thumbs.len() + 1, 3));
        assert_eq!(word.fitting_prefix(0, 1), (0, 0));
        assert_eq!(word.cluster_width_from(0), 4);
        assert_eq!(word.cluster_width_from(thumbs.len() + 1), 1);
    }

    #[test]
    fn style_runs_in_moves_a_change_inside_a_cluster_to_its_end() {
        // Review regression (#6641): a stream split after `👩` put a style
        // change inside the `👩‍💻` sequence. The sequence takes the style of
        // its first character; the change applies from the next cluster.
        let red = Style::default().fg(ratatui::style::Color::Red);
        let blue = Style::default().fg(ratatui::style::Color::Blue);
        let green = Style::default().fg(ratatui::style::Color::Green);
        let coder = "\u{1F469}\u{200D}\u{1F4BB}";
        let mut word = MeasuredWord::new("\u{1F469}", "", red);
        word.append_fragment(&MeasuredWord::new("\u{200D}", "", green));
        word.append_fragment(&MeasuredWord::new("\u{1F4BB}ab", "", blue));
        assert_eq!(word.style_runs.len(), 3);
        let len = word.text.len();
        assert_eq!(
            word.style_runs_in(0, len),
            vec![(0, red), (coder.len(), blue)]
        );
        assert_eq!(word.style_runs_in(0, coder.len()), vec![(0, red)]);
        assert_eq!(word.style_runs_in(coder.len(), len), vec![(0, blue)]);

        // A change inside a cluster back to the active style adds no run.
        let mut same = MeasuredWord::new("\u{1F469}", "", red);
        same.append_fragment(&MeasuredWord::new("\u{200D}", "", blue));
        same.append_fragment(&MeasuredWord::new("\u{1F4BB}a", "", red));
        assert_eq!(same.style_runs_in(0, same.text.len()), vec![(0, red)]);
    }

    #[test]
    fn style_runs_in_visits_only_the_runs_inside_the_slice() {
        let red = Style::default().fg(ratatui::style::Color::Red);
        let blue = Style::default().fg(ratatui::style::Color::Blue);
        let mut word = MeasuredWord::new("a", "", red);
        for index in 1..1_000 {
            let style = if index % 2 == 0 { red } else { blue };
            word.append_fragment(&MeasuredWord::new("a", "", style));
        }
        assert_eq!(word.style_runs.len(), 1_000);
        assert_eq!(word.style_runs_in(0, 1), vec![(0, red)]);
        assert_eq!(word.style_runs_in(999, 1_000), vec![(0, blue)]);
        assert_eq!(
            word.style_runs_in(500, 503),
            vec![(0, red), (1, blue), (2, red)]
        );
    }

    #[test]
    fn style_runs_in_rebases_the_active_run() {
        let red = Style::default().fg(ratatui::style::Color::Red);
        let blue = Style::default().fg(ratatui::style::Color::Blue);
        let mut word = MeasuredWord::new("abc", "", red);
        word.append_fragment(&MeasuredWord::new("def", "", blue));
        assert_eq!(word.style_runs_in(0, 2), vec![(0, red)]);
        assert_eq!(word.style_runs_in(2, 5), vec![(0, red), (1, blue)]);
        assert_eq!(word.style_runs_in(3, 6), vec![(0, blue)]);
    }

    #[test]
    fn test_measure_simple_words() {
        let words = measure_words("hello world", Style::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "hello");
        assert_eq!(words[0].width, 5);
        assert_eq!(words[0].whitespace_width, 1);
        assert_eq!(words[1].text, "world");
        assert_eq!(words[1].width, 5);
        assert_eq!(words[1].whitespace_width, 0);
    }

    #[test]
    fn test_measure_cjk() {
        let words = measure_words("你好 world", Style::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "你好");
        assert_eq!(words[0].width, 4);
        assert_eq!(words[1].text, "world");
        assert_eq!(words[1].width, 5);
    }

    #[test]
    fn test_measure_empty() {
        let words = measure_words("", Style::default());
        assert_eq!(words.len(), 0);
    }

    #[test]
    fn test_measure_multiple_spaces() {
        let words = measure_words("hello   world", Style::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "hello");
        assert_eq!(words[0].whitespace_width, 3);
    }

    #[test]
    fn test_measure_preserves_style() {
        use ratatui::style::Color;
        let style = Style::default().fg(Color::Red);
        let words = measure_words("hello world", style);
        assert_eq!(words[0].primary_style(), style);
        assert_eq!(words[1].primary_style(), style);
        assert_eq!(words[0].style_runs.len(), 1);
    }

    #[test]
    fn test_append_fragment_preserves_both_styles() {
        use ratatui::style::Color;
        let red = Style::default().fg(Color::Red);
        let blue = Style::default().fg(Color::Blue);

        let mut word = MeasuredWord::new("hel", "", red);
        let tail = MeasuredWord::new("lo", "", blue);
        word.append_fragment(&tail);

        assert_eq!(word.text, "hello");
        assert_eq!(word.width, 5);
        assert_eq!(word.style_runs.len(), 2);
        assert_eq!(word.style_runs[0], (0, red));
        assert_eq!(word.style_runs[1], (3, blue)); // "hel".len() == 3

        let segments: Vec<_> = word.segments().collect();
        assert_eq!(segments, vec![("hel", red), ("lo", blue)]);
    }

    #[test]
    fn test_append_fragment_same_style_stays_single_run() {
        use ratatui::style::Color;
        let red = Style::default().fg(Color::Red);

        let mut word = MeasuredWord::new("hel", "", red);
        let tail = MeasuredWord::new("lo", "", red);
        word.append_fragment(&tail);

        assert_eq!(word.text, "hello");
        assert_eq!(word.style_runs.len(), 1); // no new run needed
    }

    #[test]
    fn newline_is_a_hard_break_not_soft_width() {
        let words = measure_words("foo\nbar", Style::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "foo");
        assert_eq!(words[0].hard_breaks, 1);
        assert_eq!(words[0].whitespace_width, 0);
        assert_eq!(words[1].text, "bar");
        assert_eq!(words[1].hard_breaks, 0);
    }

    #[test]
    fn crlf_is_one_hard_break() {
        let words = measure_words("foo\r\nbar", Style::default());
        assert_eq!(words[0].hard_breaks, 1);
        assert_eq!(words[1].text, "bar");
    }

    #[test]
    fn lone_cr_is_one_hard_break() {
        let words = measure_words("foo\rbar", Style::default());
        assert_eq!(words[0].hard_breaks, 1);
        assert_eq!(words[1].text, "bar");
        let (trailing, pending) = measure_words_pending("foo\r", Style::default());
        assert_eq!(trailing[0].hard_breaks, 1);
        assert!(pending);
    }

    #[test]
    fn tab_and_unicode_spaces_stay_soft() {
        let tab = measure_words("foo\tbar", Style::default());
        assert_eq!(tab.len(), 2);
        assert_eq!(tab[0].hard_breaks, 0);
        assert_eq!(tab[0].whitespace_width, UnicodeWidthStr::width("\t"));

        let ideographic = measure_words("foo\u{3000}bar", Style::default());
        assert_eq!(ideographic[0].hard_breaks, 0);
        assert!(ideographic[0].whitespace_width > 0);
    }

    #[test]
    fn consecutive_and_edge_breaks() {
        let consecutive = measure_words("foo\n\nbar", Style::default());
        assert_eq!(consecutive[0].hard_breaks, 2);
        assert_eq!(consecutive[1].text, "bar");

        let leading = measure_words("\nfoo", Style::default());
        assert_eq!(leading[0].text, "");
        assert_eq!(leading[0].hard_breaks, 1);
        assert_eq!(leading[1].text, "foo");

        let trailing = measure_words("foo\n", Style::default());
        assert_eq!(trailing.len(), 1);
        assert_eq!(trailing[0].hard_breaks, 1);
    }
}
