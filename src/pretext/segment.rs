use std::iter::Peekable;
use std::str::CharIndices;

use ratatui::style::Style;
use textwrap::core::Fragment;
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
