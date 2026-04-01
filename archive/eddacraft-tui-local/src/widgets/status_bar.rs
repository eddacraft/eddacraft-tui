use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;

pub struct StatusBar<'a, T: Theme> {
    theme: &'a T,
    left: Vec<StatusItem<'a>>,
    right: Vec<StatusItem<'a>>,
}

pub struct StatusItem<'a> {
    pub label: &'a str,
    pub kind: StatusKind,
}

#[derive(Clone, Copy)]
pub enum StatusKind {
    Normal,
    Success,
    Error,
    Warning,
    Muted,
}

impl<'a, T: Theme> StatusBar<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    #[must_use]
    pub fn left(mut self, items: Vec<StatusItem<'a>>) -> Self {
        self.left = items;
        self
    }

    #[must_use]
    pub fn right(mut self, items: Vec<StatusItem<'a>>) -> Self {
        self.right = items;
        self
    }
}

impl<T: Theme> Widget for StatusBar<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.theme.base());

        let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let left_spans: Vec<Span> = self
            .left
            .iter()
            .flat_map(|item| {
                let style = match item.kind {
                    StatusKind::Normal => self.theme.base(),
                    StatusKind::Success => self.theme.status_ok(),
                    StatusKind::Error => self.theme.status_error(),
                    StatusKind::Warning => self.theme.status_warning(),
                    StatusKind::Muted => self.theme.disabled(),
                };
                vec![Span::styled(item.label, style), Span::raw(" ")]
            })
            .collect();

        let right_spans: Vec<Span> = self
            .right
            .iter()
            .flat_map(|item| {
                let style = match item.kind {
                    StatusKind::Normal => self.theme.base(),
                    StatusKind::Success => self.theme.status_ok(),
                    StatusKind::Error => self.theme.status_error(),
                    StatusKind::Warning => self.theme.status_warning(),
                    StatusKind::Muted => self.theme.disabled(),
                };
                vec![Span::raw(" "), Span::styled(item.label, style)]
            })
            .collect();

        Line::from(left_spans).render(chunks[0], buf);
        Line::from(right_spans).render(chunks[1], buf);
    }
}
