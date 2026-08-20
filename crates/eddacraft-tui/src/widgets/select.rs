use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

/// Pad `text` with trailing spaces until its Unicode display width is `width`.
fn pad_to_display_width(text: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(text);
    if used >= width {
        text.to_string()
    } else {
        let mut out = String::with_capacity(text.len() + (width - used));
        out.push_str(text);
        out.push_str(&" ".repeat(width - used));
        out
    }
}

/// Widest label among items that actually show a description, so the second
/// column starts at one shared offset (Dave B26 / #4057).
fn description_label_width(items: &[SelectItem]) -> usize {
    items
        .iter()
        .filter(|item| item.description.as_ref().is_some_and(|d| !d.is_empty()))
        .map(|item| UnicodeWidthStr::width(item.label.as_str()))
        .max()
        .unwrap_or(0)
}

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
        let inner =
            super::render_block(self.block.as_ref(), self.theme.border_focused(), area, buf);

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

        let label_width = description_label_width(&self.items);

        for (i, item) in self
            .items
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(visible_height)
        {
            #[allow(clippy::cast_possible_truncation)]
            let y = inner.y.saturating_add((i - state.offset) as u16);
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
                    Span::styled(
                        format!("{prefix}{}", pad_to_display_width(&item.label, label_width)),
                        label_style,
                    ),
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

    #[test]
    fn descriptions_share_a_column_across_variable_width_labels() {
        use crate::theme::EddaCraftTheme;

        let area = Rect::new(0, 0, 60, 3);
        let mut buf = Buffer::empty(area);
        let mut state = SelectState::default();
        let theme = EddaCraftTheme;
        StatefulWidget::render(
            Select::new(
                vec![
                    SelectItem::new("Short", "alpha"),
                    SelectItem::new("Much longer label", "bravo"),
                ],
                &theme,
            ),
            area,
            &mut buf,
            &mut state,
        );

        let text = buffer_plain(&buf);
        let alpha = cell_column(&buf, "alpha").unwrap_or_else(|| panic!("alpha missing:\n{text}"));
        let bravo = cell_column(&buf, "bravo").unwrap_or_else(|| panic!("bravo missing:\n{text}"));
        assert_eq!(
            alpha, bravo,
            "descriptions must share a column (alpha={alpha}, bravo={bravo}):\n{text}"
        );
    }

    fn buffer_plain(buf: &Buffer) -> String {
        let mut out = String::new();
        for y in buf.area.y..buf.area.y + buf.area.height {
            for x in buf.area.x..buf.area.x + buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn cell_column(buf: &Buffer, needle: &str) -> Option<u16> {
        for y in buf.area.y..buf.area.y + buf.area.height {
            let mut line = String::new();
            let mut at_byte = Vec::new();
            for x in buf.area.x..buf.area.x + buf.area.width {
                at_byte.push((x, line.len()));
                line.push_str(buf[(x, y)].symbol());
            }
            if let Some(byte_idx) = line.find(needle) {
                return at_byte
                    .into_iter()
                    .find(|(_, b)| *b == byte_idx)
                    .map(|(x, _)| x);
            }
        }
        None
    }
}
