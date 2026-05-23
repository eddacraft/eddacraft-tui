//! Themed modal dialog designed to be rendered inside an [`OverlayStack`]
//! layer. Provides a bordered, severity-coloured container with title and
//! multi-line body text.
//!
//! ```rust,no_run
//! # use eddacraft_tui::prelude::*;
//! # use eddacraft_tui::widgets::modal::Modal;
//! # let theme = EddaCraftTheme;
//! let _ = Modal::new(&theme).title("Confirm").body("Are you sure?");
//! ```
//!
//! Pair with [`OverlayStack`](crate::widgets::overlay::OverlayStack) for
//! placement and scrim:
//!
//! ```rust,no_run
//! use eddacraft_tui::prelude::*;
//! use eddacraft_tui::widgets::modal::Modal;
//! use eddacraft_tui::widgets::overlay::{Layer, OverlayStack, Placement};
//! use ratatui::Frame;
//! use ratatui::layout::Rect;
//!
//! fn render(frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
//!     OverlayStack::new(theme)
//!         .push(
//!             Layer::new(|f, a, t| {
//!                 f.render_widget(Modal::new(t).title("Saved").body("Snapshot stored."), a);
//!             })
//!             .placement(Placement::Center { width: 40, height: 7 })
//!             .scrim(true),
//!         )
//!         .render_to_frame(frame, area);
//! }
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap};

use crate::theme::Theme;
use crate::widgets::status_badge::BadgeStatus;

pub struct Modal<'a, T: Theme> {
    theme: &'a T,
    title: Option<&'a str>,
    body: &'a str,
    severity: BadgeStatus,
    footer: Option<&'a str>,
}

impl<'a, T: Theme> Modal<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            title: None,
            body: "",
            severity: BadgeStatus::Info,
            footer: None,
        }
    }

    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    #[must_use]
    pub fn body(mut self, body: &'a str) -> Self {
        self.body = body;
        self
    }

    /// Severity controls the border/title colour. Defaults to
    /// [`BadgeStatus::Info`].
    #[must_use]
    pub fn severity(mut self, severity: BadgeStatus) -> Self {
        self.severity = severity;
        self
    }

    /// Optional footer text rendered along the bottom border (e.g. key hints).
    #[must_use]
    pub fn footer(mut self, footer: &'a str) -> Self {
        self.footer = Some(footer);
        self
    }
}

impl<T: Theme> Widget for Modal<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let border_style = self.severity.severity_style(self.theme);
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .style(self.theme.base());

        if let Some(title) = self.title {
            block = block.title(Line::styled(format!(" {title} "), border_style));
        }
        if let Some(footer) = self.footer {
            block = block.title_bottom(Line::styled(format!(" {footer} "), self.theme.disabled()));
        }

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        Paragraph::new(self.body)
            .style(self.theme.base())
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn renders_rounded_border() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 10, 4);
        let mut buf = Buffer::empty(area);
        Modal::new(&theme).render(area, &mut buf);
        assert_eq!(buf[(0, 0)].symbol(), "╭");
        assert_eq!(buf[(9, 0)].symbol(), "╮");
        assert_eq!(buf[(0, 3)].symbol(), "╰");
        assert_eq!(buf[(9, 3)].symbol(), "╯");
    }

    #[test]
    fn title_appears_on_top_border() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 14, 4);
        let mut buf = Buffer::empty(area);
        Modal::new(&theme).title("Hi").render(area, &mut buf);
        let top: String = (0..14).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(top.contains("Hi"), "got: {top:?}");
    }

    #[test]
    fn footer_appears_on_bottom_border() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 18, 4);
        let mut buf = Buffer::empty(area);
        Modal::new(&theme)
            .footer("[esc] close")
            .render(area, &mut buf);
        let bottom: String = (0..18).map(|x| buf[(x, 3)].symbol().to_string()).collect();
        assert!(bottom.contains("esc"), "got: {bottom:?}");
    }

    #[test]
    fn body_renders_in_inner_area() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 10, 4);
        let mut buf = Buffer::empty(area);
        Modal::new(&theme).body("hello").render(area, &mut buf);
        let body_row: String = (1..9).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert!(body_row.starts_with("hello"));
    }

    #[test]
    fn severity_changes_border_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 6, 3);
        let mut buf = Buffer::empty(area);
        Modal::new(&theme)
            .severity(BadgeStatus::Error)
            .render(area, &mut buf);
        assert_eq!(buf[(0, 0)].fg, theme.status_error().fg.unwrap());
    }

    #[test]
    fn tiny_area_is_a_noop() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        Modal::new(&theme).body("x").render(area, &mut buf);
        // Should not panic; cell stays blank.
        assert_eq!(buf[(0, 0)].symbol(), " ");
    }
}
