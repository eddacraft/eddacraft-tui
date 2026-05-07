//! Non-blocking notification widget. A [`Toast`] renders a single severity
//! message; [`ToastStack`] lays out a vertical stack of toasts in a corner of
//! the parent area.
//!
//! Toasts have no built-in timer — the application is responsible for
//! retaining/expiring them in its own state and reconstructing the stack each
//! frame. Pair with [`OverlayStack`](crate::widgets::overlay::OverlayStack) to
//! float them above page content.
//!
//! ```rust,no_run
//! # use eddacraft_tui::prelude::*;
//! # use eddacraft_tui::widgets::toast::{Toast, ToastPlacement, ToastStack};
//! # let theme = EddaCraftTheme;
//! let _ = ToastStack::new(&theme)
//!     .placement(ToastPlacement::TopRight)
//!     .width(32)
//!     .push(Toast::new(&theme, "Saved").severity(BadgeStatus::Success))
//!     .push(Toast::new(&theme, "Connection lost").severity(BadgeStatus::Error));
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;
use crate::widgets::status_badge::BadgeStatus;

/// Where in the parent area a [`ToastStack`] anchors its toasts.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub enum ToastPlacement {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    Top,
    Bottom,
}

pub struct Toast<'a, T: Theme> {
    theme: &'a T,
    message: &'a str,
    severity: BadgeStatus,
    icon: Option<&'a str>,
}

impl<'a, T: Theme> Toast<'a, T> {
    pub fn new(theme: &'a T, message: &'a str) -> Self {
        Self {
            theme,
            message,
            severity: BadgeStatus::Info,
            icon: None,
        }
    }

    #[must_use]
    pub fn severity(mut self, severity: BadgeStatus) -> Self {
        self.severity = severity;
        self
    }

    /// Override the leading icon. Defaults to the icon implied by `severity`.
    #[must_use]
    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    fn default_icon(&self) -> &'static str {
        match self.severity {
            BadgeStatus::Success => "✓",
            BadgeStatus::Error => "✖",
            BadgeStatus::Warning => "!",
            BadgeStatus::Info | BadgeStatus::Running => "i",
            BadgeStatus::Skipped => "·",
        }
    }

    /// Estimated height (in cells) needed to render this toast at `width`.
    /// Includes the 2-row border. Used by [`ToastStack`] to lay out toasts.
    ///
    /// Width is measured in display columns via [`unicode_width`] so wide
    /// characters (CJK, emoji) and combining marks count correctly.
    #[must_use]
    pub fn measured_height(&self, width: u16) -> u16 {
        if width <= 4 {
            return 3;
        }
        let inner_width = usize::from(width - 4); // borders + icon column
        let total = UnicodeWidthStr::width(self.message);
        let lines = total.div_ceil(inner_width.max(1));
        let lines = u16::try_from(lines).unwrap_or(1).max(1);
        lines.saturating_add(2)
    }
}

impl<T: Theme> Widget for Toast<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }
        let style = self.severity.severity_style(self.theme);
        let icon = self.icon.unwrap_or_else(|| self.default_icon());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(style)
            .style(self.theme.base());

        let inner = block.inner(area);
        Clear.render(area, buf);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let body = Line::from(vec![
            Span::styled(format!("{icon} "), style),
            Span::styled(self.message, self.theme.base()),
        ]);
        Paragraph::new(body)
            .wrap(Wrap { trim: true })
            .render(inner, buf);
    }
}

pub struct ToastStack<'a, T: Theme> {
    theme: &'a T,
    toasts: Vec<Toast<'a, T>>,
    placement: ToastPlacement,
    width: u16,
    gap: u16,
    max: Option<usize>,
}

impl<'a, T: Theme> ToastStack<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            toasts: Vec::new(),
            placement: ToastPlacement::default(),
            width: 32,
            gap: 0,
            max: None,
        }
    }

    #[must_use]
    pub fn placement(mut self, placement: ToastPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Width of each toast in cells. Default 32.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.width = width.max(4);
        self
    }

    /// Vertical gap (rows) between stacked toasts. Default 0.
    #[must_use]
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Cap the number of toasts retained in the stack. When the cap is
    /// exceeded, the oldest toasts are dropped on `push`. Default uncapped.
    /// Use this to bound memory when toasts are produced from a message bus
    /// or log stream.
    #[must_use]
    pub fn max(mut self, n: usize) -> Self {
        self.max = Some(n);
        if let Some(cap) = self.max
            && self.toasts.len() > cap
        {
            let drop = self.toasts.len() - cap;
            self.toasts.drain(0..drop);
        }
        self
    }

    #[must_use]
    pub fn push(mut self, toast: Toast<'a, T>) -> Self {
        self.toasts.push(toast);
        if let Some(cap) = self.max
            && self.toasts.len() > cap
        {
            let drop = self.toasts.len() - cap;
            self.toasts.drain(0..drop);
        }
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.toasts.len()
    }
}

impl<T: Theme> Widget for ToastStack<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.toasts.is_empty() || area.width == 0 || area.height == 0 {
            return;
        }
        // Anchor the stack against the theme base so gap rows pick up the
        // background and unicode-width measurement does not change semantics.
        buf.set_style(area, self.theme.base());

        let toast_width = self.width.min(area.width);

        // Measure first so we know vertical extent.
        let heights: Vec<u16> = self
            .toasts
            .iter()
            .map(|t| t.measured_height(toast_width))
            .collect();

        let x = match self.placement {
            ToastPlacement::TopLeft | ToastPlacement::BottomLeft => area.x,
            ToastPlacement::TopRight | ToastPlacement::BottomRight => {
                area.x + area.width.saturating_sub(toast_width)
            }
            ToastPlacement::Top | ToastPlacement::Bottom => {
                area.x + area.width.saturating_sub(toast_width) / 2
            }
        };

        let mut y = match self.placement {
            ToastPlacement::TopLeft | ToastPlacement::TopRight | ToastPlacement::Top => area.y,
            ToastPlacement::BottomLeft | ToastPlacement::BottomRight | ToastPlacement::Bottom => {
                // Accumulate in u32 then saturate to u16 — sums can overflow
                // u16 with thousands of toasts, which would silently wrap and
                // anchor the stack at the wrong Y in release builds.
                let height_sum: u32 = heights.iter().copied().map(u32::from).sum();
                let gap_count = u32::try_from(heights.len().saturating_sub(1)).unwrap_or(0);
                let gap_total = u32::from(self.gap).saturating_mul(gap_count);
                let total = u16::try_from(height_sum.saturating_add(gap_total)).unwrap_or(u16::MAX);
                area.y + area.height.saturating_sub(total)
            }
        };

        for (toast, height) in self.toasts.into_iter().zip(heights) {
            if y >= area.y + area.height {
                break;
            }
            let render_height = height.min(area.y + area.height - y);
            let rect = Rect::new(x, y, toast_width, render_height);
            toast.render(rect, buf);
            y = y.saturating_add(render_height).saturating_add(self.gap);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn toast_renders_border_and_message() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 16, 3);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "ok").render(area, &mut buf);
        assert_eq!(buf[(0, 0)].symbol(), "╭");
        let inner: String = (1..15).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert!(inner.contains("ok"), "got: {inner:?}");
    }

    #[test]
    fn toast_severity_drives_border_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "boom")
            .severity(BadgeStatus::Error)
            .render(area, &mut buf);
        assert_eq!(buf[(0, 0)].fg, theme.status_error().fg.unwrap());
    }

    #[test]
    fn toast_default_icon_changes_per_severity() {
        let theme = EddaCraftTheme;
        let success = Toast::new(&theme, "x").severity(BadgeStatus::Success);
        let warning = Toast::new(&theme, "x").severity(BadgeStatus::Warning);
        assert_eq!(success.default_icon(), "✓");
        assert_eq!(warning.default_icon(), "!");
    }

    #[test]
    fn toast_explicit_icon_overrides_default() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 16, 3);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "msg").icon("@").render(area, &mut buf);
        let inner: String = (1..15).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert!(inner.starts_with('@'), "got: {inner:?}");
    }

    #[test]
    fn toast_too_small_is_noop() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 3, 2);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "x").render(area, &mut buf);
        assert_eq!(buf[(0, 0)].symbol(), " ");
    }

    #[test]
    fn toast_measured_height_grows_with_message() {
        let theme = EddaCraftTheme;
        let short = Toast::new(&theme, "x").measured_height(20);
        let long_msg = "a".repeat(100);
        let long = Toast::new(&theme, &long_msg).measured_height(20);
        assert!(long > short, "long={long} short={short}");
    }

    #[test]
    fn stack_top_right_anchors_at_top_right() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        ToastStack::new(&theme)
            .placement(ToastPlacement::TopRight)
            .width(10)
            .push(Toast::new(&theme, "a"))
            .render(area, &mut buf);
        // Top-right corner at column 39, row 0 should be border.
        assert_eq!(buf[(39, 0)].symbol(), "╮");
    }

    #[test]
    fn stack_bottom_left_anchors_at_bottom_left() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        ToastStack::new(&theme)
            .placement(ToastPlacement::BottomLeft)
            .width(10)
            .push(Toast::new(&theme, "a"))
            .render(area, &mut buf);
        // Bottom-left corner at row 9 column 0 should be `╰` (3-row toast → rows 7..10).
        assert_eq!(buf[(0, 9)].symbol(), "╰");
    }

    #[test]
    fn stack_top_centred_horizontally() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        ToastStack::new(&theme)
            .placement(ToastPlacement::Top)
            .width(10)
            .push(Toast::new(&theme, "a"))
            .render(area, &mut buf);
        // (40-10)/2 = 15, top-left of toast at column 15 row 0.
        assert_eq!(buf[(15, 0)].symbol(), "╭");
    }

    #[test]
    fn stack_renders_multiple_toasts_in_order() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        ToastStack::new(&theme)
            .placement(ToastPlacement::TopLeft)
            .width(20)
            .push(Toast::new(&theme, "first"))
            .push(Toast::new(&theme, "second"))
            .render(area, &mut buf);
        // First toast occupies rows 0..3; second starts at row 3.
        let first_inner: String = (1..19).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        let second_inner: String = (1..19).map(|x| buf[(x, 4)].symbol().to_string()).collect();
        assert!(first_inner.contains("first"), "row1={first_inner:?}");
        assert!(second_inner.contains("second"), "row4={second_inner:?}");
    }

    #[test]
    fn stack_gap_separates_toasts() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        ToastStack::new(&theme)
            .placement(ToastPlacement::TopLeft)
            .width(20)
            .gap(1)
            .push(Toast::new(&theme, "a"))
            .push(Toast::new(&theme, "b"))
            .render(area, &mut buf);
        // First toast rows 0..3, gap row 3 blank, second starts row 4.
        for x in 0..20 {
            assert_eq!(buf[(x, 3)].symbol(), " ");
        }
        assert_eq!(buf[(0, 4)].symbol(), "╭");
    }

    #[test]
    fn measured_height_uses_display_columns_for_cjk() {
        // ASCII control: 14 columns of inner space, 14 chars → 1 line.
        let theme = EddaCraftTheme;
        let ascii = Toast::new(&theme, "abcdefghijklmn").measured_height(18);
        // CJK: each of the 4 characters is 2 display columns (8 columns)
        // and the inner width is 14 columns → still 1 line.
        let cjk = Toast::new(&theme, "日本語版").measured_height(18);
        assert_eq!(cjk, ascii);
        // CJK long enough to wrap: 16 chars at 2 columns each = 32 columns
        // → wraps to 3 lines + 2 border rows.
        let long = Toast::new(&theme, &"日".repeat(16)).measured_height(18);
        assert!(long >= 5, "long={long}");
    }

    #[test]
    fn stack_height_sum_does_not_overflow_u16() {
        // Push more toasts than `u16::MAX / 3` (the minimum-height ToastStack
        // anchor calculation). The previous implementation summed heights as
        // u16 and overflowed; the fix accumulates in u32 then saturates.
        let theme = EddaCraftTheme;
        let mut stack = ToastStack::new(&theme).placement(ToastPlacement::BottomLeft);
        for _ in 0..30_000 {
            stack = stack.push(Toast::new(&theme, "x"));
        }
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        // Must not panic in debug mode.
        stack.render(area, &mut buf);
    }

    #[test]
    fn stack_max_drops_oldest() {
        let theme = EddaCraftTheme;
        let stack = ToastStack::new(&theme)
            .max(2)
            .push(Toast::new(&theme, "old"))
            .push(Toast::new(&theme, "mid"))
            .push(Toast::new(&theme, "new"));
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn empty_stack_is_noop() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        let stack = ToastStack::new(&theme);
        assert!(stack.is_empty());
        stack.render(area, &mut buf);
        for y in 0..5 {
            for x in 0..20 {
                assert_eq!(buf[(x, y)].symbol(), " ");
            }
        }
    }
}
