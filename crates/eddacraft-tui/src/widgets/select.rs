use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::theme::Theme;

pub struct Select<'a, T: Theme> {
    items: Vec<String>,
    theme: &'a T,
    block: Option<Block<'a>>,
}

#[derive(Debug, Default)]
pub struct SelectState {
    pub selected: usize,
    pub offset: usize,
}

impl SelectState {
    pub fn next(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.selected = (self.selected + 1) % item_count;
    }

    pub fn previous(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.selected = self.selected.checked_sub(1).unwrap_or(item_count - 1);
    }
}

impl<'a, T: Theme> Select<'a, T> {
    pub fn new(items: Vec<String>, theme: &'a T) -> Self {
        Self {
            items,
            theme,
            block: None,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }
}

impl<T: Theme> StatefulWidget for Select<'_, T> {
    type State = SelectState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = if let Some(block) = &self.block {
            let styled = block.clone().border_style(self.theme.border_focused());
            let inner = styled.inner(area);
            styled.render(area, buf);
            inner
        } else {
            area
        };

        let visible_height = inner.height as usize;
        if self.items.is_empty() || visible_height == 0 {
            return;
        }

        // Clamp stale selection index when the item list has shrunk since last render.
        state.selected = state.selected.min(self.items.len() - 1);

        if state.selected < state.offset {
            state.offset = state.selected;
        } else if state.selected >= state.offset + visible_height {
            state.offset = state.selected - visible_height + 1;
        }

        for (i, item) in self
            .items
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(visible_height)
        {
            #[allow(clippy::cast_possible_truncation)]
            let y = inner.y + (i - state.offset) as u16;
            let row_area = Rect::new(inner.x, y, inner.width, 1);

            let style = if i == state.selected {
                self.theme.highlighted()
            } else {
                self.theme.base()
            };

            let prefix = if i == state.selected { "▸ " } else { "  " };
            let line = Line::styled(format!("{prefix}{item}"), style);
            line.render(row_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_state_wraps_around() {
        let mut state = SelectState::default();
        state.next(3);
        assert_eq!(state.selected, 1);
        state.next(3);
        assert_eq!(state.selected, 2);
        state.next(3);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn select_state_wraps_backwards() {
        let mut state = SelectState::default();
        state.previous(3);
        assert_eq!(state.selected, 2);
        state.previous(3);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn select_state_handles_empty() {
        let mut state = SelectState::default();
        state.next(0);
        assert_eq!(state.selected, 0);
        state.previous(0);
        assert_eq!(state.selected, 0);
    }
}
