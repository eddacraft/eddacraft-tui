use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
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
    pub cursor: usize,
}

impl TextInputState {
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

        let display = if state.value.is_empty() {
            Line::styled(self.placeholder, self.theme.disabled())
        } else {
            Line::styled(&state.value, self.theme.base())
        };

        display.render(inner, buf);
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
}
