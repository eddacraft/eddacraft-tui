use super::segment::{MeasuredWord, measure_words_pending};
use ratatui::style::Style;

/// Cached preparation of text content.
/// The prepare phase: expensive unicode-width measurement happens once here.
/// Subsequent layout calls use only the cached width values.
#[derive(Debug, Clone)]
pub struct PreparedText {
    /// All measured words from the text.
    words: Vec<MeasuredWord>,
    /// The raw text, kept for rendering.
    raw_text: String,
    /// Total display width of all content (words + whitespace).
    total_width: usize,
    /// Trailing CR from the last chunk. It is already one hard break.
    /// A following chunk that starts with LF completes that CRLF pair
    /// instead of adding a second break.
    pending_cr: bool,
}

impl PreparedText {
    /// Prepare text by measuring all word widths with a uniform style.
    /// This is the expensive operation — call it once, then reuse.
    pub fn new(text: &str) -> Self {
        Self::styled(text, Style::default())
    }

    /// Prepare text with a specific style applied to all words.
    pub fn styled(text: &str, style: Style) -> Self {
        let (words, pending_cr) = measure_words_pending(text, style);
        let total_width = words.iter().map(|w| w.width + w.whitespace_width).sum();

        Self {
            words,
            raw_text: text.to_string(),
            total_width,
            pending_cr,
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
        // An empty chunk must not clear a pending CR. The next non-empty
        // chunk may still supply the LF of a split CRLF pair.
        if text.is_empty() {
            return;
        }

        let (mut new_words, new_pending) = measure_words_pending(text, style);
        // The previous chunk already counted its trailing CR as one break.
        // A leading LF completes that pair; drop the extra break `measure`
        // assigned to it. Any other leading character leaves the CR break
        // in place.
        if self.pending_cr && text.starts_with('\n') {
            subtract_one_leading_break(&mut new_words);
        }
        self.pending_cr = new_pending;
        // Cross-chunk leading soft whitespace still attaches to the previous
        // word when that word has not already ended on a hard break. A chunk
        // that ends on a break, or starts with one, must not merge the
        // boundary into a single word. Mid-word style runs are preserved
        // when the boundary really is a continuation.
        stitch_words(&mut self.words, &mut self.total_width, new_words);
        self.raw_text.push_str(text);
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

/// Drop one leading hard break. Used when a new chunk's LF completes a CR
/// that the previous chunk already recorded.
fn subtract_one_leading_break(words: &mut Vec<MeasuredWord>) {
    let Some(first) = words.first_mut() else {
        return;
    };
    if first.hard_breaks == 0 {
        return;
    }
    first.hard_breaks -= 1;
    if first.hard_breaks == 0
        && first.text.is_empty()
        && first.width == 0
        && first.whitespace_width == 0
    {
        words.remove(0);
    }
}

/// Join `new_words` onto `words`, attaching leading breaks and soft
/// whitespace and merging a mid-word boundary when the previous word is
/// still open.
fn stitch_words(
    words: &mut Vec<MeasuredWord>,
    total_width: &mut usize,
    mut new_words: Vec<MeasuredWord>,
) {
    let mut index = 0;
    while index < new_words.len() {
        let Some(prev) = words.last() else {
            break;
        };
        let first = &new_words[index];
        let empty_carrier = first.text.is_empty() && first.width == 0;
        if !empty_carrier {
            break;
        }
        if prev.hard_breaks == 0 && first.whitespace_width > 0 {
            let extra = first.whitespace_width;
            let breaks = first.hard_breaks;
            let last = words.len() - 1;
            let prev = &mut words[last];
            prev.whitespace_width += extra;
            prev.hard_breaks = prev.hard_breaks.saturating_add(breaks);
            *total_width += extra;
            index += 1;
            continue;
        }
        if first.whitespace_width == 0 && first.hard_breaks > 0 {
            let breaks = first.hard_breaks;
            let last = words.len() - 1;
            let prev = &mut words[last];
            prev.hard_breaks = prev.hard_breaks.saturating_add(breaks);
            index += 1;
            continue;
        }
        break;
    }

    let can_merge = words
        .last()
        .is_some_and(|prev| prev.whitespace_width == 0 && prev.hard_breaks == 0)
        && new_words
            .get(index)
            .is_some_and(|word| !word.text.is_empty());
    if can_merge {
        let first = new_words.remove(index);
        let last = words.len() - 1;
        let prev = &mut words[last];
        let old = prev.width + prev.whitespace_width;
        prev.append_fragment(&first);
        *total_width += (prev.width + prev.whitespace_width) - old;
    }

    *total_width += new_words[index..]
        .iter()
        .map(|word| word.width + word.whitespace_width)
        .sum::<usize>();
    words.extend(new_words.drain(index..));
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

    #[test]
    fn append_newline_between_chunks_forces_a_new_row() {
        let mut prepared = PreparedText::new("foo");
        prepared.append("\n");
        prepared.append("bar");
        assert_eq!(prepared.words().len(), 2);
        assert_eq!(prepared.words()[0].text, "foo");
        assert_eq!(prepared.words()[0].hard_breaks, 1);
        assert_eq!(prepared.words()[1].text, "bar");
    }

    #[test]
    fn append_chunk_ending_in_a_break_does_not_merge() {
        let mut prepared = PreparedText::new("foo\n");
        prepared.append("bar");
        assert_eq!(prepared.word_count(), 2);
        assert_eq!(prepared.words()[0].text, "foo");
        assert_eq!(prepared.words()[0].hard_breaks, 1);
        assert_eq!(prepared.words()[1].text, "bar");
    }

    #[test]
    fn append_chunk_starting_with_a_break_does_not_merge() {
        let mut prepared = PreparedText::new("foo");
        prepared.append_styled("\nbar", Style::default());
        assert_eq!(prepared.word_count(), 2);
        assert_eq!(prepared.words()[0].hard_breaks, 1);
        assert_eq!(prepared.words()[1].text, "bar");
    }

    #[test]
    fn append_split_across_a_break_keeps_one_boundary() {
        let mut prepared = PreparedText::new("fo");
        prepared.append("o\nba");
        prepared.append("r");
        assert_eq!(prepared.word_count(), 2);
        assert_eq!(prepared.words()[0].text, "foo");
        assert_eq!(prepared.words()[0].hard_breaks, 1);
        assert_eq!(prepared.words()[1].text, "bar");
    }

    #[test]
    fn append_split_crlf_is_one_break() {
        let mut split = PreparedText::new("foo");
        split.append("\r");
        split.append("\n");
        split.append("bar");
        assert_eq!(split.words()[0].hard_breaks, 1);
        assert_eq!(split.words()[1].text, "bar");

        let mut glued = PreparedText::new("foo\r");
        glued.append("\nbar");
        assert_eq!(glued.words()[0].text, "foo");
        assert_eq!(glued.words()[0].hard_breaks, 1);
        assert_eq!(glued.word_count(), 2);
        assert_eq!(glued.words()[1].text, "bar");

        let mut direct = PreparedText::new("");
        direct.append("foo\r\nbar");
        assert_eq!(direct.words()[0].hard_breaks, 1);
        assert_eq!(direct.words()[1].text, "bar");
    }

    #[test]
    fn append_after_a_break_keeps_leading_soft_space_on_the_new_row() {
        let mut prepared = PreparedText::new("foo");
        prepared.append("\n bar");
        assert_eq!(prepared.words()[0].hard_breaks, 1);
        assert_eq!(prepared.words()[0].whitespace_width, 0);
        let indent = prepared
            .words()
            .iter()
            .find(|word| word.text.is_empty())
            .expect("indent");
        assert_eq!(indent.whitespace_width, 1);
        assert!(prepared.words().iter().any(|word| word.text == "bar"));
    }
}
