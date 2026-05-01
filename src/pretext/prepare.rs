use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::pretext::{MeasuredWord, measure_words};

#[derive(Debug, Clone)]
pub struct PreparedText {
    words: Vec<MeasuredWord>,
    raw_text: String,
    total_width: usize,
}

impl PreparedText {
    pub fn new(text: &str) -> Self {
        Self::styled(text, Style::default())
    }

    pub fn styled(text: &str, style: Style) -> Self {
        let words = measure_words(text, style);
        let total_width = words
            .iter()
            .map(|word| word.width + word.whitespace_width)
            .sum();

        Self {
            words,
            raw_text: text.to_owned(),
            total_width,
        }
    }

    pub fn append(&mut self, text: &str) {
        self.append_styled(text, Style::default());
    }

    pub fn append_styled(&mut self, text: &str, style: Style) {
        let leading_ws_end = text
            .char_indices()
            .take_while(|(_, ch)| ch.is_whitespace())
            .last()
            .map_or(0, |(index, ch)| index + ch.len_utf8());

        if leading_ws_end > 0
            && let Some(last) = self.words.last_mut()
        {
            let width = UnicodeWidthStr::width(&text[..leading_ws_end]);
            last.whitespace_width += width;
            self.total_width += width;
        }

        let remainder = &text[leading_ws_end..];
        let new_words = measure_words(remainder, style);

        if let (Some(last), Some(first_new)) = (self.words.last_mut(), new_words.first())
            && last.whitespace_width == 0
            && !remainder.starts_with(char::is_whitespace)
        {
            let old_last_total = last.width + last.whitespace_width;

            last.append_fragment(first_new);

            let merged_last_total = last.width + last.whitespace_width;
            let remaining_new_total: usize = new_words
                .iter()
                .skip(1)
                .map(|word| word.width + word.whitespace_width)
                .sum();

            self.words.extend(new_words.into_iter().skip(1));
            self.raw_text.push_str(text);
            self.total_width += (merged_last_total - old_last_total) + remaining_new_total;
            return;
        }

        let new_total = new_words
            .iter()
            .map(|word| word.width + word.whitespace_width)
            .sum::<usize>();

        self.words.extend(new_words);
        self.raw_text.push_str(text);
        self.total_width += new_total;
    }

    pub fn words(&self) -> &[MeasuredWord] {
        &self.words
    }

    pub fn total_width(&self) -> usize {
        self.total_width
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    pub fn raw_text(&self) -> &str {
        &self.raw_text
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;

    #[test]
    fn prepare_measures_words_once() {
        let prepared = PreparedText::new("hello world");

        assert_eq!(prepared.word_count(), 2);
        assert_eq!(prepared.total_width(), 11);
    }

    #[test]
    fn append_streaming_chunks_preserves_spacing() {
        let mut prepared = PreparedText::new("");

        prepared.append("The ");
        prepared.append("quick ");
        prepared.append("brown fox");

        assert_eq!(prepared.word_count(), 4);
        assert_eq!(prepared.raw_text(), "The quick brown fox");
        assert_eq!(prepared.total_width(), 19);
    }

    #[test]
    fn append_merges_mid_word_and_preserves_styles() {
        let red = Style::default().fg(Color::Red);
        let blue = Style::default().fg(Color::Blue);
        let mut prepared = PreparedText::styled("hel", red);

        prepared.append_styled("lo world", blue);

        let hello = &prepared.words()[0];
        assert_eq!(hello.text, "hello");
        assert_eq!(hello.style_runs, vec![(0, red), (3, blue)]);
        assert_eq!(prepared.words()[1].primary_style(), blue);
    }
}
