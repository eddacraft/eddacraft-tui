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

fn kind_style<T: Theme>(kind: StatusKind, theme: &T) -> ratatui::style::Style {
    match kind {
        StatusKind::Normal => theme.base(),
        StatusKind::Success => theme.status_ok(),
        StatusKind::Error => theme.status_error(),
        StatusKind::Warning => theme.status_warning(),
        StatusKind::Muted => theme.disabled(),
    }
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
    pub fn left(mut self, items: impl IntoIterator<Item = StatusItem<'a>>) -> Self {
        self.left = items.into_iter().collect();
        self
    }

    #[must_use]
    pub fn right(mut self, items: impl IntoIterator<Item = StatusItem<'a>>) -> Self {
        self.right = items.into_iter().collect();
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
                let style = kind_style(item.kind, self.theme);
                vec![Span::styled(item.label, style), Span::raw(" ")]
            })
            .collect();

        let right_spans: Vec<Span> = self
            .right
            .iter()
            .flat_map(|item| {
                let style = kind_style(item.kind, self.theme);
                vec![Span::raw(" "), Span::styled(item.label, style)]
            })
            .collect();

        Line::from(left_spans).render(chunks[0], buf);
        Line::from(right_spans).render(chunks[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::Widget;

    use super::*;
    use crate::theme::EddaCraftTheme;

    fn item(label: &str, kind: StatusKind) -> StatusItem<'_> {
        StatusItem { label, kind }
    }

    #[test]
    fn empty_status_bar_renders_base_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        StatusBar::new(&theme).render(area, &mut buf);

        for x in 0..20 {
            assert_eq!(buf[(x, 0)].symbol(), " ");
        }
    }

    #[test]
    fn left_items_appear_in_left_half() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        StatusBar::new(&theme)
            .left(vec![item("OK", StatusKind::Normal)])
            .render(area, &mut buf);

        let text: String = (0..10).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(text.starts_with("OK"));
    }

    #[test]
    fn right_items_appear_in_right_half() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        StatusBar::new(&theme)
            .right(vec![item("v1", StatusKind::Muted)])
            .render(area, &mut buf);

        let text: String = (10..20).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(text.contains("v1"));
    }

    #[test]
    fn success_kind_uses_status_ok_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        StatusBar::new(&theme)
            .left(vec![item("pass", StatusKind::Success)])
            .render(area, &mut buf);

        let expected = theme.status_ok();
        assert_eq!(buf[(0, 0)].fg, expected.fg.unwrap());
    }

    #[test]
    fn error_kind_uses_status_error_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        StatusBar::new(&theme)
            .left(vec![item("fail", StatusKind::Error)])
            .render(area, &mut buf);

        let expected = theme.status_error();
        assert_eq!(buf[(0, 0)].fg, expected.fg.unwrap());
    }

    #[test]
    fn warning_kind_uses_status_warning_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        StatusBar::new(&theme)
            .left(vec![item("warn", StatusKind::Warning)])
            .render(area, &mut buf);

        let expected = theme.status_warning();
        assert_eq!(buf[(0, 0)].fg, expected.fg.unwrap());
    }
}
