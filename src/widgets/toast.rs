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
use ratatui::widgets::{Block, Borders, Clear, Widget};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;
use crate::widgets::status_badge::BadgeStatus;
use crate::widgets::with_ellipsis;

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

    /// Override the leading marker. Defaults to the bracket marker for
    /// `severity` ([`BadgeStatus::marker`]).
    #[must_use]
    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    fn default_icon(&self) -> &'static str {
        self.severity.marker()
    }

    fn marker(&self) -> &'a str {
        self.icon.unwrap_or_else(|| self.default_icon())
    }

    /// Lays out the marker and message for a toast `width` cells wide (2 of
    /// them border). [`measured_height`](Self::measured_height) and
    /// [`render`](Widget::render) share it, so what is measured is what draws.
    fn layout(&self, width: u16) -> ToastLayout {
        let marker = self.marker();
        let inner = usize::from(width).saturating_sub(2);
        if inner == 0 {
            return ToastLayout {
                marker: vec![marker.to_owned()],
                message: Vec::new(),
                message_width: 0,
                beside: true,
            };
        }
        let prefix = UnicodeWidthStr::width(marker).saturating_add(1);
        if let Some(message_width) = inner.checked_sub(prefix).filter(|&w| w > 0) {
            return ToastLayout {
                marker: vec![marker.to_owned()],
                message: wrap_message(self.message, message_width),
                message_width,
                beside: true,
            };
        }
        // No column left beside the marker: wrap the marker itself by display
        // column and put the message under it at the full inner width.
        let message = if self.message.is_empty() {
            Vec::new()
        } else {
            wrap_message(self.message, inner)
        };
        ToastLayout {
            marker: split_columns(marker, inner),
            message,
            message_width: inner,
            beside: false,
        }
    }

    /// Estimated height (in cells) needed to render this toast at `width`.
    /// Includes the 2-row border. Used by [`ToastStack`] to lay out toasts.
    ///
    /// Width is measured in display columns via [`unicode_width`] so wide
    /// characters (CJK, emoji) and combining marks count correctly — both in
    /// the message body and in a custom [`icon`](Self::icon). A marker that
    /// leaves no column for the message wraps across rows, and the message
    /// follows below it; both count towards the height.
    #[must_use]
    pub fn measured_height(&self, width: u16) -> u16 {
        u16::try_from(self.layout(width).rows())
            .unwrap_or(u16::MAX)
            .saturating_add(2)
    }
}

/// Where a toast's marker and message lines go inside its border.
struct ToastLayout {
    /// Marker rows, drawn from the top at the full inner width.
    marker: Vec<String>,
    /// Message rows, wrapped to `message_width` columns.
    message: Vec<String>,
    message_width: usize,
    /// The message sits beside the single marker row rather than below it.
    beside: bool,
}

impl ToastLayout {
    fn rows(&self) -> usize {
        if self.beside {
            self.message.len().max(1)
        } else {
            self.marker.len() + self.message.len()
        }
    }
}

/// Word-wraps `message` into lines at most `width` columns wide, splitting a
/// word longer than the line.
fn wrap_message(message: &str, width: usize) -> Vec<String> {
    textwrap::wrap(message, width)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect()
}

/// Splits `text` into lines at most `width` display columns wide without
/// breaking a grapheme cluster (a ZWJ emoji sequence, a base plus its
/// variation selector or combining marks). Each cluster is measured whole,
/// so VS16 widening counts. A cluster wider than `width` sits on its own line.
fn split_columns(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let gw = UnicodeWidthStr::width(grapheme);
        if used + gw > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            used = 0;
        }
        line.push_str(grapheme);
        used += gw;
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

impl<T: Theme> Widget for Toast<'_, T> {
    /// The marker is drawn whole on the first row; the message wraps in the
    /// columns beside it. A marker that leaves no column for the message wraps
    /// across rows and the message goes below it. When the toast is too small
    /// for the message, the message is cut with an ellipsis before the marker
    /// is (ESC-054); only a toast with fewer rows than the marker needs clips it.
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }
        let style = self.severity.severity_style(self.theme);
        let layout = self.layout(area.width);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(self.severity.box_border())
            .border_style(style)
            .style(self.theme.base());

        let inner = block.inner(area);
        Clear.render(area, buf);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let rows = usize::from(inner.height);
        for (row, text) in (0u16..).zip(layout.marker.iter().take(rows)) {
            Line::from(Span::styled(text.as_str(), style))
                .render(Rect::new(inner.x, inner.y + row, inner.width, 1), buf);
        }

        let top = if layout.beside {
            0
        } else {
            layout.marker.len().min(rows)
        };
        let message_rows = rows - top;
        let mut lines = layout.message;
        if lines.len() > message_rows {
            lines.truncate(message_rows);
            if let Some(last) = lines.last_mut() {
                // Always mark the cut, even when the kept line already fits.
                *last = with_ellipsis(last, layout.message_width);
            }
        }
        let width = u16::try_from(layout.message_width).unwrap_or(u16::MAX);
        let x = inner.x + inner.width - width;
        let y = inner.y + u16::try_from(top).unwrap_or(u16::MAX);
        for (row, text) in (0u16..).zip(lines) {
            Line::from(Span::styled(text, self.theme.base()))
                .render(Rect::new(x, y + row, width, 1), buf);
        }
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

        let bottom = matches!(
            self.placement,
            ToastPlacement::BottomLeft | ToastPlacement::BottomRight | ToastPlacement::Bottom
        );
        let (skip, mut y) = if bottom {
            let (skip, total) = bottom_fit(&heights, self.gap, area.height);
            (skip, area.y + area.height - total)
        } else {
            (0, area.y)
        };

        for (toast, height) in self.toasts.into_iter().zip(heights).skip(skip) {
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

/// Picks the newest toasts whose heights and gaps fit in `area_height` rows,
/// returning how many older toasts to skip and the rows the kept ones span.
/// The newest toast is always kept; one taller than the area is cut to fit
/// it. A bottom-anchored stack that overflows thereby stays on the bottom
/// edge and never hands a toast fewer rows than it needs.
fn bottom_fit(heights: &[u16], gap: u16, area_height: u16) -> (usize, u16) {
    let limit = u32::from(area_height);
    let mut total = 0u32;
    let mut kept = 0usize;
    for &height in heights.iter().rev() {
        let next = if kept == 0 {
            u32::from(height)
        } else {
            total + u32::from(gap) + u32::from(height)
        };
        if kept > 0 && next > limit {
            break;
        }
        total = next;
        kept += 1;
        if total >= limit {
            break;
        }
    }
    let total = u16::try_from(total.min(limit)).unwrap_or(area_height);
    (heights.len() - kept, total)
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
    fn unknown_toast_draws_a_dashed_border() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 16, 3);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "state?")
            .severity(BadgeStatus::Unknown)
            .render(area, &mut buf);
        assert_eq!(buf[(0, 0)].symbol(), "┌");
        assert_eq!(buf[(1, 0)].symbol(), "╌");
        assert_eq!(buf[(1, 0)].fg, theme.status_unknown().fg.unwrap());
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
        assert_eq!(success.default_icon(), "[ OK ]");
        assert_eq!(warning.default_icon(), "[ WARN ]");
    }

    fn row(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    }

    #[test]
    fn toast_draws_the_full_marker_beside_the_message() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 24, 3);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "Saved")
            .severity(BadgeStatus::Warning)
            .render(area, &mut buf);
        assert_eq!(row(&buf, 1), "│[ WARN ] Saved        │");
    }

    #[test]
    fn toast_cuts_the_message_before_the_marker() {
        // ESC-054: at 3 rows there is one message line of 6 columns; the
        // marker stays whole and the message ends in an ellipsis.
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 16, 3);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "connection lost to the daemon")
            .severity(BadgeStatus::Error)
            .render(area, &mut buf);
        assert_eq!(row(&buf, 1), "│[ ERR ] conne…│");
    }

    #[test]
    fn toast_wraps_the_message_under_itself() {
        let theme = EddaCraftTheme;
        let toast = Toast::new(&theme, "alpha beta gamma").severity(BadgeStatus::Success);
        let height = toast.measured_height(16);
        let area = Rect::new(0, 0, 16, height);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "alpha beta gamma")
            .severity(BadgeStatus::Success)
            .render(area, &mut buf);
        assert_eq!(height, 5);
        assert_eq!(row(&buf, 1), "│[ OK ] alpha  │");
        assert_eq!(row(&buf, 2), "│       beta   │");
        assert_eq!(row(&buf, 3), "│       gamma  │");
    }

    #[test]
    fn toast_too_narrow_for_a_message_keeps_the_marker() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 9, 3);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "lost")
            .severity(BadgeStatus::Error)
            .render(area, &mut buf);
        assert_eq!(row(&buf, 1), "│[ ERR ]│");
    }

    #[test]
    fn toast_with_no_room_beside_the_marker_puts_the_message_below_it() {
        // #6634: the marker fills the inner row, so the message wraps under it
        // at the full inner width and the measured height counts that row.
        let theme = EddaCraftTheme;
        let toast = Toast::new(&theme, "lost").severity(BadgeStatus::Error);
        let height = toast.measured_height(9);
        assert_eq!(height, 4);
        let area = Rect::new(0, 0, 9, height);
        let mut buf = Buffer::empty(area);
        toast.render(area, &mut buf);
        assert_eq!(row(&buf, 1), "│[ ERR ]│");
        assert_eq!(row(&buf, 2), "│lost   │");
    }

    #[test]
    fn toast_wraps_a_custom_icon_wider_than_the_inner_area() {
        // #6634: a 10-column icon in a 6-column inner area used to measure 3
        // rows and clip to `ABCDEF`. It now wraps by display column, and the
        // message follows below it.
        let theme = EddaCraftTheme;
        let toast = Toast::new(&theme, "ok").icon("ABCDEFGHIJ");
        let height = toast.measured_height(8);
        assert_eq!(height, 5);
        let area = Rect::new(0, 0, 8, height);
        let mut buf = Buffer::empty(area);
        toast.render(area, &mut buf);
        assert_eq!(row(&buf, 1), "│ABCDEF│");
        assert_eq!(row(&buf, 2), "│GHIJ  │");
        assert_eq!(row(&buf, 3), "│ok    │");
    }

    #[test]
    fn toast_wraps_a_wide_icon_without_splitting_a_character() {
        // Three 2-column glyphs in a 5-column inner area: two per row.
        let theme = EddaCraftTheme;
        let toast = Toast::new(&theme, "").icon("🔥🔥🔥");
        let height = toast.measured_height(7);
        assert_eq!(height, 4);
        let area = Rect::new(0, 0, 7, height);
        let mut buf = Buffer::empty(area);
        toast.render(area, &mut buf);
        assert_eq!(buf[(1, 1)].symbol(), "🔥");
        assert_eq!(buf[(3, 1)].symbol(), "🔥");
        assert_eq!(buf[(1, 2)].symbol(), "🔥");
    }

    #[test]
    fn toast_wraps_an_icon_without_splitting_a_zwj_sequence() {
        // `👩‍💻` is one 2-column grapheme of three scalars. Wrapping by scalar
        // broke it after the ZWJ and measured three marker rows.
        let theme = EddaCraftTheme;
        let toast = Toast::new(&theme, "").icon("a👩\u{200d}💻");
        let height = toast.measured_height(4);
        assert_eq!(height, 4);
        let area = Rect::new(0, 0, 4, height);
        let mut buf = Buffer::empty(area);
        toast.render(area, &mut buf);
        assert_eq!(buf[(1, 1)].symbol(), "a");
        assert_eq!(buf[(1, 2)].symbol(), "👩\u{200d}💻");
    }

    #[test]
    fn toast_wraps_an_icon_by_grapheme_width() {
        // `❤️` is `❤` (1 column alone) plus VS16, which makes it 2 columns.
        // Summing scalar widths fitted both hearts in 3 columns and clipped
        // the second.
        let theme = EddaCraftTheme;
        let toast = Toast::new(&theme, "").icon("❤\u{fe0f}❤\u{fe0f}");
        let height = toast.measured_height(5);
        assert_eq!(height, 4);
        let area = Rect::new(0, 0, 5, height);
        let mut buf = Buffer::empty(area);
        toast.render(area, &mut buf);
        assert_eq!(buf[(1, 1)].symbol(), "❤\u{fe0f}");
        assert_eq!(buf[(1, 2)].symbol(), "❤\u{fe0f}");
    }

    #[test]
    fn toast_short_of_rows_cuts_the_message_below_a_wrapped_icon() {
        // Marker rows come first (ESC-054); the message gets what is left
        // and ends in an ellipsis.
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 8, 5);
        let mut buf = Buffer::empty(area);
        Toast::new(&theme, "alpha beta gamma")
            .icon("ABCDEFGHIJ")
            .render(area, &mut buf);
        assert_eq!(row(&buf, 1), "│ABCDEF│");
        assert_eq!(row(&buf, 2), "│GHIJ  │");
        assert_eq!(row(&buf, 3), "│alpha…│");
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
    fn measured_height_accounts_for_multi_cell_icon_width() {
        // Width 12 leaves 10 inner columns. A 1-cell icon takes 2 of them with
        // its space, leaving 8 for the message; a 2-cell icon leaves 7.
        let theme = EddaCraftTheme;
        let message = "a".repeat(15);
        let narrow = Toast::new(&theme, &message).icon("@").measured_height(12);
        let emoji = Toast::new(&theme, &message).icon("🔥").measured_height(12);
        let cjk = Toast::new(&theme, &message).icon("告").measured_height(12);
        assert_eq!(narrow, 4);
        assert_eq!(emoji, 5);
        assert_eq!(cjk, emoji);
    }

    #[test]
    fn measured_height_uses_display_columns_for_cjk() {
        // Width 18: 16 inner columns, 6 for `[ i ] `, 10 for the message.
        let theme = EddaCraftTheme;
        let ascii = Toast::new(&theme, "abcdefghij").measured_height(18);
        // 4 characters at 2 columns each fill 8 of the 10 → one line.
        let cjk = Toast::new(&theme, "日本語版").measured_height(18);
        assert_eq!(cjk, ascii);
        assert_eq!(ascii, 3);
        // 16 characters at 2 columns = 32 columns → 4 lines of 5 + 2 borders.
        let long = Toast::new(&theme, &"日".repeat(16)).measured_height(18);
        assert_eq!(long, 6);
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
    fn bottom_stack_overflow_keeps_the_newest_complete_toast_anchored() {
        // #6634: two 3-row toasts in a 4-row area used to start at row 0, draw
        // the older toast whole and give the newer one a single row (a no-op).
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 4);
        let mut buf = Buffer::empty(area);
        ToastStack::new(&theme)
            .placement(ToastPlacement::BottomLeft)
            .width(20)
            .push(Toast::new(&theme, "older"))
            .push(Toast::new(&theme, "newer"))
            .render(area, &mut buf);
        assert_eq!(row(&buf, 0).trim(), "");
        assert_eq!(buf[(0, 1)].symbol(), "╭");
        assert!(row(&buf, 2).contains("newer"), "row2={:?}", row(&buf, 2));
        assert_eq!(buf[(0, 3)].symbol(), "╰");
        assert!(!(0..4).any(|y| row(&buf, y).contains("older")));
    }

    #[test]
    fn bottom_stack_overflow_counts_gaps_when_choosing_toasts() {
        // Three 3-row toasts with a 1-row gap in 8 rows: the newest two need
        // 3 + 1 + 3 = 7 rows, so they sit on rows 1..8 and the oldest drops.
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 8);
        let mut buf = Buffer::empty(area);
        ToastStack::new(&theme)
            .placement(ToastPlacement::BottomRight)
            .width(20)
            .gap(1)
            .push(Toast::new(&theme, "one"))
            .push(Toast::new(&theme, "two"))
            .push(Toast::new(&theme, "three"))
            .render(area, &mut buf);
        assert_eq!(row(&buf, 0).trim(), "");
        assert_eq!(buf[(0, 1)].symbol(), "╭");
        assert!(row(&buf, 2).contains("two"), "row2={:?}", row(&buf, 2));
        assert_eq!(row(&buf, 4).trim(), "");
        assert!(row(&buf, 6).contains("three"), "row6={:?}", row(&buf, 6));
        assert_eq!(buf[(0, 7)].symbol(), "╰");
    }

    #[test]
    fn bottom_stack_shows_a_newest_toast_taller_than_the_area_cut() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 16, 3);
        let mut buf = Buffer::empty(area);
        ToastStack::new(&theme)
            .placement(ToastPlacement::Bottom)
            .width(16)
            .push(Toast::new(&theme, "old"))
            .push(Toast::new(&theme, "connection lost to the daemon"))
            .render(area, &mut buf);
        assert_eq!(row(&buf, 1), "│[ i ] connect…│");
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
