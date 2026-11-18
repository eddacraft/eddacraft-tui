use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthChar;

use crate::theme::Theme;

pub struct TextInput<'a, T: Theme> {
    theme: &'a T,
    block: Option<Block<'a>>,
    placeholder: &'a str,
}

#[derive(Debug, Default, Clone)]
pub struct TextInputState {
    /// The text being edited. Callers may replace it directly; the cursor is
    /// repaired (clamped to the end and snapped back to a char boundary) by
    /// the next editing call or render, so a stale cursor never panics.
    pub value: String,
    cursor: usize,
}

/// Clamp `pos` to `value.len()` and walk back to the nearest char boundary.
fn snap_to_char_boundary(value: &str, pos: usize) -> usize {
    let pos = pos.min(value.len());
    (0..=pos)
        .rev()
        .find(|&i| value.is_char_boundary(i))
        .unwrap_or(0)
}

impl TextInputState {
    /// Returns the current cursor byte offset, always a char boundary within
    /// `value`, even if `value` was replaced since the cursor last moved.
    pub fn cursor(&self) -> usize {
        snap_to_char_boundary(&self.value, self.cursor)
    }

    /// Set the cursor to `pos`. Clamps to `value.len()` and snaps to the
    /// nearest valid char boundary.
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = snap_to_char_boundary(&self.value, pos);
    }

    /// Repair a cursor left stale by a direct write to `value` (GH #6604).
    fn normalise_cursor(&mut self) {
        self.set_cursor(self.cursor);
    }

    pub fn insert(&mut self, c: char) {
        self.normalise_cursor();
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        self.normalise_cursor();
        if self.cursor > 0 {
            let prev = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.value.replace_range(prev..self.cursor, "");
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        self.normalise_cursor();
        if self.cursor < self.value.len() {
            let next = self.value[self.cursor..]
                .char_indices()
                .nth(1)
                .map_or(self.value.len(), |(i, _)| self.cursor + i);
            self.value.replace_range(self.cursor..next, "");
        }
    }

    pub fn move_left(&mut self) {
        self.normalise_cursor();
        if self.cursor > 0 {
            self.cursor = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
        }
    }

    pub fn move_right(&mut self) {
        self.normalise_cursor();
        if self.cursor < self.value.len() {
            self.cursor = self.value[self.cursor..]
                .char_indices()
                .nth(1)
                .map_or(self.value.len(), |(i, _)| self.cursor + i);
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.value.len();
    }
}

impl<'a, T: Theme> TextInput<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            block: None,
            placeholder: "",
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    #[must_use]
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }
}

impl<T: Theme> StatefulWidget for TextInput<'_, T> {
    type State = TextInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // `value` is public and may have been replaced since the last edit.
        state.normalise_cursor();
        let inner = if let Some(block) = &self.block {
            let styled = block.clone().border_style(self.theme.border_focused());
            let inner = styled.inner(area);
            styled.render(area, buf);
            inner
        } else {
            area
        };

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        if state.value.is_empty() {
            Line::styled(self.placeholder, self.theme.disabled()).render(inner, buf);
            return;
        }

        let width = inner.width as usize;
        let cursor = state.cursor;

        // Horizontal scrolling: find a byte offset `visible_start` so the
        // cursor stays on screen. Measured in display columns, not bytes —
        // see [`visible_window_start`].
        let visible_start = visible_window_start(&state.value, cursor, width);

        let visible = &state.value[visible_start..];

        let offset = cursor - visible_start;
        let before = &visible[..offset];

        let (cursor_ch, after) = if offset < visible.len() {
            let ch = &visible[offset..];
            let char_len = ch.chars().next().unwrap().len_utf8();
            (&ch[..char_len], &ch[char_len..])
        } else {
            (" ", "")
        };

        let base = self.theme.base();
        let cursor_style = base.add_modifier(Modifier::REVERSED);

        Line::from(vec![
            Span::styled(before, base),
            Span::styled(cursor_ch, cursor_style),
            Span::styled(after, base),
        ])
        .render(inner, buf);
    }
}

/// Byte offset into `value` at which the visible window should start so the
/// cursor stays on screen in a field `width` **columns** wide.
///
/// The window is advanced one character at a time until the text preceding the
/// cursor, plus the cursor cell itself, fits within `width` display columns.
/// Distances are measured with [`unicode_width`] rather than byte offsets, so
/// multi-byte characters (e.g. `é`, 2 bytes / 1 column) and wide characters
/// (e.g. `世`, 3 bytes / 2 columns) scroll at the correct point instead of too
/// early or too late (#1877).
///
/// The cursor cell occupies the display width of the character under it, so a
/// wide glyph reserves two columns; otherwise a cursor sitting on a wide char
/// in a narrow field could be left a single spare column, and ratatui skips a
/// two-cell glyph that cannot fit — the cursor would vanish entirely.
fn visible_window_start(value: &str, cursor: usize, width: usize) -> usize {
    let cursor = cursor.min(value.len());
    // Columns the cursor cell needs: the width of the char under it, or one
    // for the end-of-input reversed space. `max(1)` guards a zero-width char.
    let cursor_cols = value[cursor..]
        .chars()
        .next()
        .and_then(UnicodeWidthChar::width)
        .unwrap_or(1)
        .max(1);
    let budget = width.saturating_sub(cursor_cols);
    // Walk backwards from the cursor, accumulating the display columns of each
    // preceding character, and stop before the running total would exceed the
    // budget. O(n) in the characters scanned — the forward form re-measured the
    // whole `value[start..cursor]` prefix on every step, making it O(n²).
    let mut used = 0usize;
    let mut start = cursor;
    for (idx, ch) in value[..cursor].char_indices().rev() {
        let col = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + col > budget {
            break;
        }
        used += col;
        start = idx;
    }
    start
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_cursor() {
        let mut state = TextInputState::default();
        state.insert('h');
        state.insert('i');
        assert_eq!(state.value, "hi");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn backspace_removes_previous() {
        let mut state = TextInputState {
            value: "abc".into(),
            cursor: 3,
        };
        state.backspace();
        assert_eq!(state.value, "ab");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut state = TextInputState {
            value: "abc".into(),
            cursor: 0,
        };
        state.backspace();
        assert_eq!(state.value, "abc");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn delete_removes_next() {
        let mut state = TextInputState {
            value: "abc".into(),
            cursor: 0,
        };
        state.delete();
        assert_eq!(state.value, "bc");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn navigation() {
        let mut state = TextInputState {
            value: "abc".into(),
            cursor: 1,
        };
        state.move_right();
        assert_eq!(state.cursor, 2);
        state.move_left();
        assert_eq!(state.cursor, 1);
        state.home();
        assert_eq!(state.cursor, 0);
        state.end();
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn insert_multibyte_char() {
        let mut state = TextInputState::default();
        state.insert('é'); // 2 bytes
        assert_eq!(state.value, "é");
        assert_eq!(state.cursor, 2);
        state.insert('中'); // 3 bytes
        assert_eq!(state.value, "é中");
        assert_eq!(state.cursor, 5);
    }

    #[test]
    fn backspace_multibyte_char() {
        let mut state = TextInputState::default();
        state.insert('é');
        state.insert('中');
        state.backspace();
        assert_eq!(state.value, "é");
        assert_eq!(state.cursor, 2);
        state.backspace();
        assert_eq!(state.value, "");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn navigation_multibyte() {
        let mut state = TextInputState::default();
        state.insert('a');
        state.insert('é');
        state.insert('中');
        // cursor at end (6 bytes: a=1, é=2, 中=3)
        assert_eq!(state.cursor, 6);
        state.move_left();
        assert_eq!(state.cursor, 3); // before 中
        state.move_left();
        assert_eq!(state.cursor, 1); // before é
        state.move_right();
        assert_eq!(state.cursor, 3); // after é
    }

    #[test]
    fn set_cursor_snaps_to_char_boundary() {
        let mut state = TextInputState::default();
        state.insert('é'); // 2 bytes at positions 0-1
        state.set_cursor(1); // mid-codepoint, should snap to 0
        assert_eq!(state.cursor(), 0);
        state.set_cursor(2); // valid boundary
        assert_eq!(state.cursor(), 2);
        state.set_cursor(999); // past end, clamp
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn visible_window_start_scrolls_by_columns_for_ascii() {
        // 10 ASCII columns, 5-wide field, cursor at end: keep the last 4
        // columns plus the cursor cell (unchanged from the byte-based logic,
        // since 1 byte == 1 column for ASCII).
        assert_eq!(visible_window_start("abcdefghij", 10, 5), 6);
    }

    #[test]
    fn visible_window_start_fits_without_scrolling() {
        assert_eq!(visible_window_start("abc", 3, 10), 0);
    }

    #[test]
    fn visible_window_start_measures_display_columns_not_bytes() {
        // #1877: 5×'é' is 10 bytes but only 5 columns. In a 5-wide field with
        // the cursor at the end, the correct window hides one 'é' (start=2) so
        // 4 columns + the cursor fit. The old byte-based logic compared the
        // 10-byte distance against 5 columns and scrolled to byte 6 — hiding
        // three characters and wasting two columns.
        let value = "ééééé";
        assert_eq!(value.len(), 10, "each 'é' is two bytes");
        assert_eq!(visible_window_start(value, value.len(), 5), 2);
    }

    #[test]
    fn visible_window_start_accounts_for_wide_chars() {
        // '世' is 3 bytes / 2 columns. 3×'世' = 9 bytes, 6 columns. A 5-wide
        // field with the cursor at the end shows two '世' (4 columns) plus the
        // cursor, starting at byte 3.
        let value = "世世世";
        assert_eq!(value.len(), 9);
        assert_eq!(visible_window_start(value, value.len(), 5), 3);
    }

    #[test]
    fn visible_window_start_terminates_at_zero_width() {
        // Degenerate 0-column field: the `start < cursor` bound still
        // terminates the loop (no infinite spin) and returns the cursor offset.
        assert_eq!(visible_window_start("abc", 3, 0), 3);
    }

    #[test]
    fn render_with_wide_chars_keeps_cursor_visible_without_panicking() {
        use crate::theme::EddaCraftTheme;
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;

        let theme = EddaCraftTheme;
        let mut state = TextInputState::default();
        for _ in 0..8 {
            state.insert('世'); // 8 wide chars, cursor at the end
        }

        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        TextInput::new(&theme).render(area, &mut buf, &mut state);

        // The reversed cursor cell must land inside the 5-column field.
        let cursor_visible =
            (0..area.width).any(|x| buf[(x, 0)].modifier.contains(Modifier::REVERSED));
        assert!(
            cursor_visible,
            "cursor cell must be visible within the field"
        );
    }

    #[test]
    fn visible_window_start_reserves_columns_for_a_wide_cursor_glyph() {
        // #1877: with the cursor sitting on a wide (2-column) glyph in a narrow
        // field, the window must reserve two columns for the cursor cell.
        // "世世世世" is 4 wide chars (12 bytes / 8 columns); with the cursor
        // before the 2nd glyph (byte 3) in a 3-column field, budgeting only one
        // column would leave the 2-cell cursor glyph unrenderable.
        assert_eq!(visible_window_start("世世世世", 3, 3), 3);
    }

    #[test]
    fn render_wide_char_under_cursor_stays_visible_in_narrow_field() {
        use crate::theme::EddaCraftTheme;
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;

        // Regression for the wide-cursor-glyph case: cursor ON a 2-column char
        // in a 3-column field. Before the fix, ratatui skipped the 2-cell glyph
        // that had only one spare column and the reversed cursor disappeared.
        let theme = EddaCraftTheme;
        let mut state = TextInputState::default();
        for _ in 0..4 {
            state.insert('世');
        }
        state.set_cursor(3); // on the 2nd wide glyph

        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        TextInput::new(&theme).render(area, &mut buf, &mut state);

        let cursor_visible =
            (0..area.width).any(|x| buf[(x, 0)].modifier.contains(Modifier::REVERSED));
        assert!(
            cursor_visible,
            "a wide glyph under the cursor must still be drawn in a narrow field"
        );
    }

    // GH #6604: `value` is public, so a caller can replace it and leave the
    // cursor past the end or inside a multi-byte character. Every editing and
    // rendering operation must repair the cursor instead of panicking.

    fn state_with_stale_cursor(cursor_text: &str, replacement: &str) -> TextInputState {
        let mut state = TextInputState::default();
        for c in cursor_text.chars() {
            state.insert(c);
        }
        state.value = replacement.to_string();
        state
    }

    #[test]
    fn insert_after_value_replaced_with_shorter_string() {
        // Finding ceaf: cursor at 2, value replaced with "a", then insert.
        let mut state = state_with_stale_cursor("ab", "a");
        state.insert('b');
        assert_eq!(state.value, "ab");
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn insert_after_value_replaced_with_empty_string() {
        // Finding 792b: cursor at 1, value replaced with "", then insert.
        let mut state = state_with_stale_cursor("a", "");
        state.insert('b');
        assert_eq!(state.value, "b");
        assert_eq!(state.cursor(), 1);
    }

    #[test]
    fn insert_after_value_replaced_with_multibyte_string() {
        // Cursor at byte 1, value replaced with "é" (2 bytes): byte 1 is
        // inside the character, so the cursor snaps back to 0.
        let mut state = state_with_stale_cursor("a", "é");
        state.insert('x');
        assert_eq!(state.value, "xé");
        assert_eq!(state.cursor(), 1);
    }

    #[test]
    fn backspace_after_value_replaced_with_empty_string() {
        let mut state = state_with_stale_cursor("a", "");
        state.backspace();
        assert_eq!(state.value, "");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn backspace_after_value_replaced_with_shorter_string() {
        let mut state = state_with_stale_cursor("abc", "a");
        state.backspace();
        assert_eq!(state.value, "");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn backspace_after_value_replaced_with_multibyte_string() {
        let mut state = state_with_stale_cursor("abc", "é中");
        // Byte 3 is inside '中' (bytes 2..5): the cursor snaps to 2, so
        // backspace removes 'é' and leaves the cursor at 0.
        state.backspace();
        assert_eq!(state.value, "中");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn delete_after_value_replaced_with_multibyte_string() {
        let mut state = state_with_stale_cursor("a", "é");
        state.delete();
        assert_eq!(state.value, "");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn delete_after_value_replaced_with_empty_string() {
        let mut state = state_with_stale_cursor("abc", "");
        state.delete();
        assert_eq!(state.value, "");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn move_left_after_value_replaced_with_empty_string() {
        let mut state = state_with_stale_cursor("a", "");
        state.move_left();
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn move_left_after_value_replaced_with_multibyte_string() {
        let mut state = state_with_stale_cursor("abc", "é中");
        state.move_left();
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn move_right_after_value_replaced_with_multibyte_string() {
        let mut state = state_with_stale_cursor("a", "éx");
        state.move_right();
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn move_right_after_value_replaced_with_shorter_string() {
        let mut state = state_with_stale_cursor("abc", "a");
        state.move_right();
        assert_eq!(state.cursor(), 1);
    }

    #[test]
    fn cursor_reports_a_valid_offset_after_value_replaced() {
        let state = state_with_stale_cursor("abc", "é");
        assert_eq!(state.cursor(), 2);
        let state = state_with_stale_cursor("a", "é");
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn render_after_value_replaced_with_multibyte_string_does_not_panic() {
        use crate::theme::EddaCraftTheme;

        // Finding 8660: cursor at byte 1, value replaced with "é". Render
        // clamped to the length but sliced inside the character.
        let theme = EddaCraftTheme;
        let mut state = state_with_stale_cursor("a", "é");
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        TextInput::new(&theme).render(area, &mut buf, &mut state);
        assert_eq!(state.cursor(), 0, "render repairs the stored cursor");
        assert_eq!(buf[(0, 0)].symbol(), "é");
        assert!(buf[(0, 0)].modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn render_after_value_replaced_with_shorter_string_repairs_cursor() {
        use crate::theme::EddaCraftTheme;

        let theme = EddaCraftTheme;
        let mut state = state_with_stale_cursor("abcdef", "ab");
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        TextInput::new(&theme).render(area, &mut buf, &mut state);
        assert_eq!(state.cursor(), 2);
        assert!(buf[(2, 0)].modifier.contains(Modifier::REVERSED));
    }
}
