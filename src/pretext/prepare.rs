use super::segment::{MeasuredWord, measure_words};
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

/// Cached preparation of text content.
/// The prepare phase: expensive unicode-width measurement happens once here.
/// Subsequent layout calls use only the cached width values.
pub struct PreparedText {
    /// All measured words from the text.
    words: Vec<MeasuredWord>,
    /// The raw text, kept for rendering.
    raw_text: String,
    /// Total display width of all content (words + whitespace).
    total_width: usize,
}

impl PreparedText {
    /// Prepare text by measuring all word widths with a uniform style.
    /// This is the expensive operation — call it once, then reuse.
    pub fn new(text: &str) -> Self {
        Self::styled(text, Style::default())
    }

    /// Prepare text with a specific style applied to all words.
    pub fn styled(text: &str, style: Style) -> Self {
        let words = measure_words(text, style);
        let total_width = words.iter().map(|w| w.width + w.whitespace_width).sum();

        Self {
            words,
            raw_text: text.to_string(),
            total_width,
        }
    }

    /// Append new text with default style without re-measuring existing content.
    /// Only the new text gets measured — existing cached widths are preserved.
    pub fn append(&mut self, text: &str) {
        self.append_styled(text, Style::default());
    }

    /// Append new text with a specific style without re-measuring existing content.
    /// This is the key optimization for streaming AI output — each token can carry
    /// its own style (e.g. code vs prose, user vs assistant).
    pub fn append_styled(&mut self, text: &str, style: Style) {
        // Cross-chunk leading whitespace must attach to the previous word
        // when one exists. `measure_words` only sees the new chunk, so a
        // chunk like `" world"` would otherwise drop or phantom-word the
        // space rather than extending the prior word's trailing whitespace.
        let leading_ws_end = text
            .char_indices()
            .take_while(|(_, ch)| ch.is_whitespace())
            .last()
            .map_or(0, |(i, ch)| i + ch.len_utf8());

        if leading_ws_end > 0
            && let Some(last) = self.words.last_mut()
        {
            let ws = &text[..leading_ws_end];
            last.whitespace_width += UnicodeWidthStr::width(ws);
            self.total_width += UnicodeWidthStr::width(ws);
        }

        // When there's no prior word, keep leading whitespace so
        // `measure_words` can emit a leading-whitespace sentinel.
        let strip_leading = leading_ws_end > 0 && !self.words.is_empty();
        let text_remainder = if strip_leading {
            &text[leading_ws_end..]
        } else {
            text
        };
        let new_words = measure_words(text_remainder, style);

        // Handle word boundary: if previous text ended without whitespace
        // and new text starts without whitespace, merge the boundary words.
        // Style runs from both fragments are preserved via append_fragment,
        // so mid-word style transitions (e.g. `"hel"` red + `"lo"` blue) are
        // retained exactly — no style is silently dropped.
        if let (Some(last), Some(first_new)) = (self.words.last_mut(), new_words.first())
            && last.whitespace_width == 0
            && !text_remainder.starts_with(char::is_whitespace)
        {
            let old_last_total = last.width + last.whitespace_width;

            last.append_fragment(first_new);

            let merged_last_total = last.width + last.whitespace_width;
            let remaining_new_total: usize = new_words
                .iter()
                .skip(1)
                .map(|w| w.width + w.whitespace_width)
                .sum();

            self.words.extend(new_words.into_iter().skip(1));
            self.raw_text.push_str(text);
            self.total_width += (merged_last_total - old_last_total) + remaining_new_total;
            return;
        }

        let new_total: usize = new_words.iter().map(|w| w.width + w.whitespace_width).sum();

        self.words.extend(new_words);
        self.raw_text.push_str(text);
        self.total_width += new_total;
    }

    /// Access the measured words for the layout phase.
    pub fn words(&self) -> &[MeasuredWord] {
        &self.words
    }

    /// Total display width of all content.
    pub fn total_width(&self) -> usize {
        self.total_width
    }

    /// Number of measured words.
    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    /// The raw text content.
    pub fn raw_text(&self) -> &str {
        &self.raw_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_prepare_basic() {
        let prepared = PreparedText::new("hello world");
        assert_eq!(prepared.word_count(), 2);
        assert_eq!(prepared.total_width(), 11);
    }

    #[test]
    fn test_prepare_styled() {
        let style = Style::default().fg(Color::Red);
        let prepared = PreparedText::styled("hello world", style);
        assert_eq!(prepared.words()[0].primary_style(), style);
        assert_eq!(prepared.words()[1].primary_style(), style);
    }

    #[test]
    fn test_append_with_whitespace_boundary() {
        let mut prepared = PreparedText::new("hello ");
        prepared.append("world");
        assert_eq!(prepared.word_count(), 2);
        assert_eq!(prepared.words()[0].text, "hello");
        assert_eq!(prepared.words()[1].text, "world");
    }

    #[test]
    fn test_append_merges_boundary_word() {
        let mut prepared = PreparedText::new("hel");
        prepared.append("lo world");
        assert_eq!(prepared.word_count(), 2);
        assert_eq!(prepared.words()[0].text, "hello");
        assert_eq!(prepared.words()[0].width, 5);
        assert_eq!(prepared.words()[1].text, "world");
    }

    #[test]
    fn test_append_empty() {
        let mut prepared = PreparedText::new("hello");
        prepared.append("");
        assert_eq!(prepared.word_count(), 1);
    }

    #[test]
    fn test_streaming_tokens() {
        let mut prepared = PreparedText::new("");
        prepared.append("The ");
        prepared.append("quick ");
        prepared.append("brown ");
        prepared.append("fox");
        assert_eq!(prepared.word_count(), 4);
        assert_eq!(prepared.words()[0].text, "The");
        assert_eq!(prepared.words()[3].text, "fox");
    }

    #[test]
    fn test_append_styled_preserves_styles() {
        let red = Style::default().fg(Color::Red);
        let blue = Style::default().fg(Color::Blue);

        let mut prepared = PreparedText::styled("hello ", red);
        prepared.append_styled("world", blue);

        assert_eq!(prepared.words()[0].primary_style(), red);
        assert_eq!(prepared.words()[1].primary_style(), blue);
    }

    #[test]
    fn test_append_styled_mixed_stream() {
        let bold = Style::default().fg(Color::Yellow);
        let normal = Style::default().fg(Color::White);

        let mut prepared = PreparedText::new("");
        prepared.append_styled("**bold** ", bold);
        prepared.append_styled("normal ", normal);
        prepared.append_styled("**bold**", bold);

        assert_eq!(prepared.word_count(), 3);
        assert_eq!(prepared.words()[0].primary_style(), bold);
        assert_eq!(prepared.words()[1].primary_style(), normal);
        assert_eq!(prepared.words()[2].primary_style(), bold);
    }

    #[test]
    fn test_boundary_merge_preserves_both_styles() {
        // Streaming "hel" red + "lo world" with blue — mid-word style change
        // must be preserved, not silently replaced by the first style.
        let red = Style::default().fg(Color::Red);
        let blue = Style::default().fg(Color::Blue);

        let mut prepared = PreparedText::styled("hel", red);
        prepared.append_styled("lo world", blue);

        assert_eq!(prepared.word_count(), 2);

        // First word is the merged "hello" with two style runs
        let hello = &prepared.words()[0];
        assert_eq!(hello.text, "hello");
        assert_eq!(hello.width, 5);
        assert_eq!(hello.style_runs.len(), 2);
        assert_eq!(hello.style_runs[0], (0, red));
        assert_eq!(hello.style_runs[1], (3, blue));

        // Segments iterator yields both styled runs
        let segs: Vec<_> = hello.segments().collect();
        assert_eq!(segs, vec![("hel", red), ("lo", blue)]);

        // Second word is the blue "world"
        let world = &prepared.words()[1];
        assert_eq!(world.text, "world");
        assert_eq!(world.primary_style(), blue);
        assert_eq!(world.style_runs.len(), 1);
    }

    #[test]
    fn test_boundary_merge_same_style_collapses() {
        // Streaming "hel" + "lo" with the same style should merge to a single run
        let red = Style::default().fg(Color::Red);

        let mut prepared = PreparedText::styled("hel", red);
        prepared.append_styled("lo", red);

        assert_eq!(prepared.word_count(), 1);
        let hello = &prepared.words()[0];
        assert_eq!(hello.text, "hello");
        assert_eq!(hello.style_runs.len(), 1);
    }

    #[test]
    fn test_append_leading_whitespace_preserved() {
        // Streaming: "hello" then " world" — the leading space must become
        // whitespace on "hello", not be silently dropped or phantomed.
        let mut prepared = PreparedText::new("hello");
        prepared.append(" world");
        assert_eq!(prepared.word_count(), 2);
        assert_eq!(prepared.words()[0].text, "hello");
        assert_eq!(prepared.words()[0].whitespace_width, 1);
        assert_eq!(prepared.words()[1].text, "world");
        assert_eq!(prepared.total_width(), 11);
    }

    #[test]
    fn test_append_only_whitespace() {
        let mut prepared = PreparedText::new("hello");
        prepared.append("   ");
        assert_eq!(prepared.word_count(), 1);
        assert_eq!(prepared.words()[0].text, "hello");
        assert_eq!(prepared.words()[0].whitespace_width, 3);
    }

    #[test]
    fn test_append_leading_whitespace_no_prior_words_preserved() {
        // Streaming `"  indented"` into an empty state must keep the indent.
        let mut prepared = PreparedText::new("");
        prepared.append("  hello world");
        assert_eq!(prepared.word_count(), 3);
        assert_eq!(prepared.words()[0].text, "");
        assert_eq!(prepared.words()[0].width, 0);
        assert_eq!(prepared.words()[0].whitespace_width, 2);
        assert_eq!(prepared.words()[1].text, "hello");
        assert_eq!(prepared.words()[2].text, "world");
        assert_eq!(prepared.total_width(), 13);
    }
}
