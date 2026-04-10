use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::theme::Theme;

pub struct TextInput<'a, T: Theme> {
    theme: &'a T,
    block: Option<Block<'a>>,
    placeholder: &'a str,
}

#[derive(Debug, Default, Clone)]
pub struct TextInputState {
    pub value: String,
    cursor: usize,
}

impl TextInputState {
    /// Returns the current cursor byte offset.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Set the cursor to `pos`. Clamps to `value.len()` and snaps to the
    /// nearest valid char boundary.
    pub fn set_cursor(&mut self, pos: usize) {
        let pos = pos.min(self.value.len());
        // Walk backward to the nearest char boundary
        let pos = (0..=pos)
            .rev()
            .find(|&i| self.value.is_char_boundary(i))
            .unwrap_or(0);
        self.cursor = pos;
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
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
        if self.cursor < self.value.len() {
            let next = self.value[self.cursor..]
                .char_indices()
                .nth(1)
                .map_or(self.value.len(), |(i, _)| self.cursor + i);
            self.value.replace_range(self.cursor..next, "");
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
        }
    }

    pub fn move_right(&mut self) {
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
        let cursor = state.cursor.min(state.value.len());

        // Horizontal scrolling: find a visible_start so the cursor is in view.
        let visible_start = {
            let mut start = 0;
            // Advance start until cursor fits within `width` columns.
            while cursor.saturating_sub(start) >= width && start < state.value.len() {
                // Move forward one char boundary.
                start = state.value[start..]
                    .char_indices()
                    .nth(1)
                    .map_or(state.value.len(), |(i, _)| start + i);
            }
            start
        };

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
}
