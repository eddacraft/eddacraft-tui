use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub label: String,
    pub description: Option<String>,
}

impl From<String> for SelectItem {
    fn from(label: String) -> Self {
        Self {
            label,
            description: None,
        }
    }
}

impl From<&str> for SelectItem {
    fn from(label: &str) -> Self {
        Self {
            label: label.to_string(),
            description: None,
        }
    }
}

impl SelectItem {
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: Some(description.into()),
        }
    }
}

pub struct Select<'a, T: Theme> {
    items: Vec<SelectItem>,
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
    pub fn new<I, S>(items: I, theme: &'a T) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SelectItem>,
    {
        Self {
            items: items.into_iter().map(Into::into).collect(),
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

            let prefix = if i == state.selected { "▸ " } else { "  " };

            let has_desc = item.description.as_ref().is_some_and(|d| !d.is_empty());

            let line = if has_desc {
                let label_style = if i == state.selected {
                    self.theme.highlighted()
                } else {
                    self.theme.base()
                };
                let desc_style = label_style.fg(self.theme.muted());

                Line::from(vec![
                    Span::styled(format!("{prefix}{}", item.label), label_style),
                    Span::styled(
                        "  ",
                        if i == state.selected {
                            label_style
                        } else {
                            self.theme.base()
                        },
                    ),
                    Span::styled(item.description.as_deref().unwrap_or(""), desc_style),
                ])
            } else {
                let style = if i == state.selected {
                    self.theme.highlighted()
                } else {
                    self.theme.base()
                };
                Line::styled(format!("{prefix}{}", item.label), style)
            };

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

    #[test]
    fn select_item_from_string() {
        let item: SelectItem = "hello".to_string().into();
        assert_eq!(item.label, "hello");
        assert!(item.description.is_none());
    }

    #[test]
    fn select_item_from_str() {
        let item: SelectItem = "hello".into();
        assert_eq!(item.label, "hello");
        assert!(item.description.is_none());
    }

    #[test]
    fn select_item_with_description() {
        let item = SelectItem::new("Run audit", "Scan for issues");
        assert_eq!(item.label, "Run audit");
        assert_eq!(item.description, Some("Scan for issues".to_string()));
    }
}
