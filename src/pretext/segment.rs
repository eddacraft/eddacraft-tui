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
///
/// Hard line breaks (`\n`, `\r\n`) appear in the word stream as zero-width
/// sentinels with [`is_hard_break`](Self::is_hard_break) set. Other whitespace
/// (` `, `\t`, other unicode spaces, lone `\r`) is soft and is absorbed into
/// the preceding word's [`whitespace_width`](Self::whitespace_width).
#[derive(Debug, Clone)]
pub struct MeasuredWord {
    /// The word text (no trailing whitespace).
    pub text: String,
    /// Display width of the word content (from unicode-width).
    pub width: usize,
    /// Display width of trailing whitespace.
    pub whitespace_width: usize,
    /// Penalty string (e.g. "-" for hyphenation). Empty for normal words.
    pub penalty: String,
    /// Style runs: `(byte_offset_into_text, style)` ordered by offset.
    /// Always non-empty; first entry has offset 0.
    pub style_runs: Vec<(usize, Style)>,
    /// True when this word represents a forced row boundary (`\n` or `\r\n`
    /// in the source text). Hard-break sentinels carry no glyphs and have
    /// zero `width` and `whitespace_width`; the layout engine splits the
    /// word stream at these positions and starts the next paragraph on a
    /// fresh row.
    pub is_hard_break: bool,
}

impl MeasuredWord {
    /// Measure a word and its trailing whitespace with a single style.
    pub fn new(word: &str, trailing_whitespace: &str, style: Style) -> Self {
        Self {
            width: UnicodeWidthStr::width(word),
            whitespace_width: UnicodeWidthStr::width(trailing_whitespace),
            text: word.to_string(),
            penalty: String::new(),
            style_runs: vec![(0, style)],
            is_hard_break: false,
        }
    }

    /// Build a hard-break sentinel. Carries no glyphs; the layout engine
    /// uses these to force row boundaries between paragraphs.
    pub(crate) fn hard_break(style: Style) -> Self {
        Self {
            text: String::new(),
            width: 0,
            whitespace_width: 0,
            penalty: String::new(),
            style_runs: vec![(0, style)],
            is_hard_break: true,
        }
    }

    /// The primary (first) style of this word. Convenience for single-style words.
    pub fn primary_style(&self) -> Style {
        self.style_runs.first().map(|r| r.1).unwrap_or_default()
    }

    /// Iterate over styled segments of this word as `(&str, Style)` pairs.
    /// Empty segments are skipped.
    pub fn segments(&self) -> impl Iterator<Item = (&str, Style)> {
        let text = self.text.as_str();
        let runs = &self.style_runs;
        (0..runs.len()).filter_map(move |i| {
            let start = runs[i].0;
            let end = runs.get(i + 1).map(|r| r.0).unwrap_or(text.len());
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
        let last_style = self.style_runs.last().map(|r| r.1).unwrap_or_default();
        for (off, style) in &other.style_runs {
            let merged_off = base + off;
            if *off == 0 && *style == last_style {
                continue;
            }
            self.style_runs.push((merged_off, *style));
        }
        self.text.push_str(&other.text);
        self.width += other.width;
        self.whitespace_width = other.whitespace_width;
        debug_assert!(
            self.style_runs.windows(2).all(|w| w[0].0 < w[1].0),
            "style_runs offsets must be strictly increasing"
        );
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

/// Split text into MeasuredWords with a uniform style.
/// Each word carries the display width of its trailing soft whitespace.
///
/// Whitespace policy:
/// - `\n` and `\r\n` are **hard**: each emits a [`MeasuredWord::hard_break`]
///   sentinel so the layout engine can force a row boundary.
/// - Spaces, tabs, other unicode whitespace, and lone `\r` are **soft**: they
///   are absorbed into the preceding word's `whitespace_width` (or, when there
///   is no preceding word, into a leading-indent sentinel).
///
/// **Grapheme limitation.** Width is measured per word with
/// `unicode_width::UnicodeWidthStr::width`. A grapheme cluster (e.g. an
/// emoji ZWJ sequence) that is *split* across two `append_styled` calls is
/// measured as two independent fragments rather than one composite glyph,
/// which can over-count display width for that grapheme. Full grapheme
/// segmentation across streaming boundaries is out of scope.
pub fn measure_words(text: &str, style: Style) -> Vec<MeasuredWord> {
    let mut words: Vec<MeasuredWord> = Vec::new();
    let mut chars = text.char_indices().peekable();

    let push_soft_ws = |words: &mut Vec<MeasuredWord>, ws_width: usize, style: Style| {
        if ws_width == 0 {
            return;
        }
        match words.last_mut() {
            Some(last) if !last.is_hard_break => last.whitespace_width += ws_width,
            _ => {
                // Either no preceding word, or the previous entry is a hard
                // break (so this whitespace begins a new row's indent). Emit
                // an empty sentinel: zero glyph width, but `whitespace_width`
                // advances `x` for the next visible word.
                words.push(MeasuredWord {
                    text: String::new(),
                    width: 0,
                    whitespace_width: ws_width,
                    penalty: String::new(),
                    style_runs: vec![(0, style)],
                    is_hard_break: false,
                });
            }
        }
    };

    while chars.peek().is_some() {
        // 1. Consume a non-whitespace word, if one starts here.
        let word_start = chars.peek().map(|&(i, _)| i).unwrap_or(0);
        let mut word_end = word_start;
        while let Some(&(i, ch)) = chars.peek() {
            if ch.is_whitespace() {
                break;
            }
            chars.next();
            word_end = i + ch.len_utf8();
        }
        if word_end > word_start {
            let word = &text[word_start..word_end];
            words.push(MeasuredWord::new(word, "", style));
        }

        // 2. Consume the following whitespace run, splitting at every hard
        //    break. Each iteration either emits one hard-break sentinel (after
        //    attaching any preceding soft whitespace) or finishes the run.
        while let Some(&(ws_start, _)) = chars.peek() {
            let mut ws_end = ws_start;
            let mut hit_hard_break = false;

            while let Some(&(i, ch)) = chars.peek() {
                if !ch.is_whitespace() {
                    break;
                }
                if ch == '\n' {
                    chars.next();
                    hit_hard_break = true;
                    break;
                }
                if ch == '\r' {
                    chars.next();
                    ws_end = i + ch.len_utf8();
                    if let Some(&(_, '\n')) = chars.peek() {
                        chars.next();
                        hit_hard_break = true;
                        // Pull `\r` back out of the soft span — the `\r\n`
                        // pair is a single hard break, not soft whitespace.
                        ws_end = i;
                        break;
                    }
                    // Lone `\r`: treat as soft whitespace. It has zero
                    // display width, so leave `ws_end` advanced past it.
                    continue;
                }
                chars.next();
                ws_end = i + ch.len_utf8();
            }

            let soft_ws_width = UnicodeWidthStr::width(&text[ws_start..ws_end]);
            push_soft_ws(&mut words, soft_ws_width, style);

            if hit_hard_break {
                words.push(MeasuredWord::hard_break(style));
                continue;
            }
            break;
        }
    }

    words
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
        assert_eq!(word.style_runs[1], (3, blue));

        let segments: Vec<_> = word.segments().collect();
        assert_eq!(segments, vec![("hel", red), ("lo", blue)]);
    }

    #[test]
    fn test_measure_leading_whitespace_emits_sentinel() {
        let words = measure_words("  hello", Style::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "");
        assert_eq!(words[0].width, 0);
        assert_eq!(words[0].whitespace_width, 2);
        assert_eq!(words[1].text, "hello");
        assert_eq!(words[1].whitespace_width, 0);
    }

    #[test]
    fn test_measure_only_whitespace_emits_sentinel() {
        let words = measure_words("    ", Style::default());
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "");
        assert_eq!(words[0].whitespace_width, 4);
    }

    #[test]
    fn test_measure_hard_break_lf() {
        let words = measure_words("foo\nbar", Style::default());
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].text, "foo");
        assert!(!words[0].is_hard_break);
        assert!(words[1].is_hard_break);
        assert_eq!(words[1].width, 0);
        assert_eq!(words[1].whitespace_width, 0);
        assert_eq!(words[2].text, "bar");
    }

    #[test]
    fn test_measure_hard_break_crlf() {
        let words = measure_words("foo\r\nbar", Style::default());
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].text, "foo");
        assert!(words[1].is_hard_break);
        assert_eq!(words[2].text, "bar");
    }

    #[test]
    fn test_measure_consecutive_hard_breaks_are_distinct() {
        let words = measure_words("foo\n\nbar", Style::default());
        assert_eq!(words.len(), 4);
        assert_eq!(words[0].text, "foo");
        assert!(words[1].is_hard_break);
        assert!(words[2].is_hard_break);
        assert_eq!(words[3].text, "bar");
    }

    #[test]
    fn test_measure_leading_hard_break() {
        let words = measure_words("\nfoo", Style::default());
        assert_eq!(words.len(), 2);
        assert!(words[0].is_hard_break);
        assert_eq!(words[1].text, "foo");
    }

    #[test]
    fn test_measure_trailing_hard_break() {
        let words = measure_words("foo\n", Style::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "foo");
        assert!(words[1].is_hard_break);
    }

    #[test]
    fn test_measure_soft_ws_around_hard_break_preserved() {
        // `"foo \n bar"`: the trailing space after `foo` is attached to `foo`,
        // the hard break advances a row, then the leading space before `bar`
        // is carried by an indent sentinel so `bar` starts indented.
        let words = measure_words("foo \n bar", Style::default());
        assert_eq!(words.len(), 4);
        assert_eq!(words[0].text, "foo");
        assert_eq!(words[0].whitespace_width, 1);
        assert!(words[1].is_hard_break);
        assert_eq!(words[2].text, "");
        assert_eq!(words[2].whitespace_width, 1);
        assert!(!words[2].is_hard_break);
        assert_eq!(words[3].text, "bar");
    }

    #[test]
    fn test_measure_lone_cr_is_soft_whitespace() {
        // Lone `\r` (no following `\n`) is soft whitespace, matching the
        // issue's policy that only `\n` / `\r\n` are hard. No hard-break
        // sentinel should appear between the two words.
        let words = measure_words("foo\rbar", Style::default());
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "foo");
        assert!(!words[0].is_hard_break);
        assert_eq!(words[1].text, "bar");
        assert!(!words[1].is_hard_break);
    }

    #[test]
    fn test_append_fragment_same_style_stays_single_run() {
        use ratatui::style::Color;
        let red = Style::default().fg(Color::Red);

        let mut word = MeasuredWord::new("hel", "", red);
        let tail = MeasuredWord::new("lo", "", red);
        word.append_fragment(&tail);

        assert_eq!(word.text, "hello");
        assert_eq!(word.style_runs.len(), 1);
    }
}
