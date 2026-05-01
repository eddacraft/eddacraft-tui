use ratatui::style::Style;
use textwrap::core::Fragment;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub struct MeasuredWord {
    pub text: String,
    pub width: usize,
    pub whitespace_width: usize,
    pub penalty: String,
    pub style_runs: Vec<(usize, Style)>,
}

impl MeasuredWord {
    pub fn new(word: &str, trailing_whitespace: &str, style: Style) -> Self {
        Self {
            text: word.to_owned(),
            width: UnicodeWidthStr::width(word),
            whitespace_width: UnicodeWidthStr::width(trailing_whitespace),
            penalty: String::new(),
            style_runs: vec![(0, style)],
        }
    }

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

    pub(crate) fn append_fragment(&mut self, other: &Self) {
        let base = self.text.len();
        let last_style = self.style_runs.last().map(|run| run.1).unwrap_or_default();

        for (offset, style) in &other.style_runs {
            if *offset != 0 || *style != last_style {
                self.style_runs.push((base + offset, *style));
            }
        }

        self.text.push_str(&other.text);
        self.width += other.width;
        self.whitespace_width = other.whitespace_width;
    }
}

impl Fragment for MeasuredWord {
    fn width(&self) -> f64 {
        usize_to_f64(self.width)
    }

    fn whitespace_width(&self) -> f64 {
        usize_to_f64(self.whitespace_width)
    }

    fn penalty_width(&self) -> f64 {
        usize_to_f64(UnicodeWidthStr::width(self.penalty.as_str()))
    }
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

pub fn measure_words(text: &str, style: Style) -> Vec<MeasuredWord> {
    let mut words: Vec<MeasuredWord> = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some(&(word_start, _)) = chars.peek() {
        let mut word_end = word_start;
        while let Some(&(index, ch)) = chars.peek() {
            if ch.is_whitespace() {
                break;
            }
            chars.next();
            word_end = index + ch.len_utf8();
        }

        if word_end == word_start {
            let ws_start = word_start;
            let mut ws_end = ws_start;
            while let Some(&(index, ch)) = chars.peek() {
                if !ch.is_whitespace() {
                    break;
                }
                chars.next();
                ws_end = index + ch.len_utf8();
            }

            if let Some(last) = words.last_mut() {
                last.whitespace_width += UnicodeWidthStr::width(&text[ws_start..ws_end]);
            }
            continue;
        }

        let ws_start = word_end;
        let mut ws_end = ws_start;
        while let Some(&(index, ch)) = chars.peek() {
            if !ch.is_whitespace() {
                break;
            }
            chars.next();
            ws_end = index + ch.len_utf8();
        }

        words.push(MeasuredWord::new(
            &text[word_start..word_end],
            &text[ws_start..ws_end],
            style,
        ));
    }

    words
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;

    #[test]
    fn measure_words_keeps_width_and_trailing_space() {
        let words = measure_words("hello   world", Style::default());

        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "hello");
        assert_eq!(words[0].width, 5);
        assert_eq!(words[0].whitespace_width, 3);
    }

    #[test]
    fn measure_words_handles_cjk_width() {
        let words = measure_words("你好 world", Style::default());

        assert_eq!(words[0].text, "你好");
        assert_eq!(words[0].width, 4);
    }

    #[test]
    fn append_fragment_preserves_style_runs() {
        let red = Style::default().fg(Color::Red);
        let blue = Style::default().fg(Color::Blue);
        let mut word = MeasuredWord::new("hel", "", red);

        word.append_fragment(&MeasuredWord::new("lo", "", blue));

        assert_eq!(word.text, "hello");
        assert_eq!(word.style_runs, vec![(0, red), (3, blue)]);
        assert_eq!(
            word.segments().collect::<Vec<_>>(),
            vec![("hel", red), ("lo", blue)]
        );
    }
}
