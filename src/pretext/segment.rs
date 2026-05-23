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
    /// Display width of trailing whitespace.
    pub whitespace_width: usize,
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
            text: word.to_string(),
            penalty: String::new(),
            style_runs: vec![(0, style)],
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

/// Split text into `MeasuredWord`s with a uniform style.
/// Each word includes its trailing whitespace measurement.
pub fn measure_words(text: &str, style: Style) -> Vec<MeasuredWord> {
    let mut words: Vec<MeasuredWord> = Vec::new();
    let mut chars = text.char_indices().peekable();

    while chars.peek().is_some() {
        let Some(&(word_start, _)) = chars.peek() else {
            break;
        };

        let mut word_end = word_start;
        while let Some(&(i, ch)) = chars.peek() {
            if ch.is_whitespace() {
                break;
            }
            chars.next();
            word_end = i + ch.len_utf8();
        }

        if word_end == word_start {
            let ws_start = word_start;
            let mut ws_end = ws_start;
            while let Some(&(i, ch)) = chars.peek() {
                if !ch.is_whitespace() {
                    break;
                }
                chars.next();
                ws_end = i + ch.len_utf8();
            }
            let ws = &text[ws_start..ws_end];
            let ws_width = UnicodeWidthStr::width(ws);
            if let Some(last) = words.last_mut() {
                last.whitespace_width += ws_width;
            } else {
                // Leading whitespace with no preceding word — push an empty
                // sentinel that carries the indent forward. Its zero width
                // means the renderer draws nothing for it; layout still
                // advances `x` by `whitespace_width`, indenting the next word.
                words.push(MeasuredWord {
                    text: String::new(),
                    width: 0,
                    whitespace_width: ws_width,
                    penalty: String::new(),
                    style_runs: vec![(0, style)],
                });
            }
            continue;
        }

        let word = &text[word_start..word_end];

        let ws_start = word_end;
        let mut ws_end = ws_start;
        while let Some(&(i, ch)) = chars.peek() {
            if !ch.is_whitespace() {
                break;
            }
            chars.next();
            ws_end = i + ch.len_utf8();
        }

        let trailing_ws = &text[ws_start..ws_end];
        words.push(MeasuredWord::new(word, trailing_ws, style));
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
