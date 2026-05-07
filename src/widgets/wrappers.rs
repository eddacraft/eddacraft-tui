//! Composable behaviour wrappers for any [`Widget`] or [`StatefulWidget`].
//!
//! Inspired by Cursive's wrapper views (`HideableView`, `EnableableView`,
//! `PaddedView`). These add behaviour without requiring every widget to grow
//! its own `visible`/`enabled`/`padding` builders.
//!
//! ```rust,no_run
//! # use eddacraft_tui::widgets::wrappers::{Disableable, Hideable, Padded};
//! # use ratatui::widgets::Paragraph;
//! let body = Paragraph::new("hello");
//! let _ = Hideable::new(body).visible(true);
//!
//! let disabled = Disableable::new(Paragraph::new("dim me")).enabled(false);
//! let _ = disabled;
//!
//! let _ = Padded::new(Paragraph::new("inset")).all(1);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{StatefulWidget, Widget};

use super::dim_buffer;

// ---------------------------------------------------------------------------
// Hideable
// ---------------------------------------------------------------------------

/// Conditionally render an inner widget. When `visible` is `false`, the
/// wrapper short-circuits without touching the buffer — useful for toggling
/// overlays, panels, or chrome without recreating layout.
pub struct Hideable<W> {
    inner: W,
    visible: bool,
}

impl<W> Hideable<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            visible: true,
        }
    }

    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Shorthand for `.visible(false)`.
    #[must_use]
    pub fn hidden(self) -> Self {
        self.visible(false)
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl<W: Widget> Widget for Hideable<W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.visible {
            self.inner.render(area, buf);
        }
    }
}

impl<W: StatefulWidget> StatefulWidget for Hideable<W> {
    type State = W::State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if self.visible {
            self.inner.render(area, buf, state);
        }
    }
}

// ---------------------------------------------------------------------------
// Disableable
// ---------------------------------------------------------------------------

/// Render an inner widget, then dim every cell in `area` when `enabled` is
/// `false`. Visually communicates "this control is non-interactive" without
/// each widget needing its own disabled state.
pub struct Disableable<W> {
    inner: W,
    enabled: bool,
}

impl<W> Disableable<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            enabled: true,
        }
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shorthand for `.enabled(false)`.
    #[must_use]
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl<W: Widget> Widget for Disableable<W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let enabled = self.enabled;
        self.inner.render(area, buf);
        if !enabled {
            dim_buffer(area, buf);
        }
    }
}

impl<W: StatefulWidget> StatefulWidget for Disableable<W> {
    type State = W::State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let enabled = self.enabled;
        self.inner.render(area, buf, state);
        if !enabled {
            dim_buffer(area, buf);
        }
    }
}

// ---------------------------------------------------------------------------
// Padded
// ---------------------------------------------------------------------------

/// Inset the inner widget by per-side padding. Cells in the padding region are
/// left untouched.
pub struct Padded<W> {
    inner: W,
    top: u16,
    right: u16,
    bottom: u16,
    left: u16,
}

impl<W> Padded<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        }
    }

    /// Set padding for all four sides.
    #[must_use]
    pub fn all(mut self, padding: u16) -> Self {
        self.top = padding;
        self.right = padding;
        self.bottom = padding;
        self.left = padding;
        self
    }

    /// Set top + bottom padding.
    #[must_use]
    pub fn vertical(mut self, padding: u16) -> Self {
        self.top = padding;
        self.bottom = padding;
        self
    }

    /// Set left + right padding.
    #[must_use]
    pub fn horizontal(mut self, padding: u16) -> Self {
        self.left = padding;
        self.right = padding;
        self
    }

    /// Set explicit per-side padding (CSS order: top, right, bottom, left).
    #[must_use]
    pub fn padding(mut self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
        self.top = top;
        self.right = right;
        self.bottom = bottom;
        self.left = left;
        self
    }

    #[must_use]
    pub fn top(mut self, padding: u16) -> Self {
        self.top = padding;
        self
    }

    #[must_use]
    pub fn right(mut self, padding: u16) -> Self {
        self.right = padding;
        self
    }

    #[must_use]
    pub fn bottom(mut self, padding: u16) -> Self {
        self.bottom = padding;
        self
    }

    #[must_use]
    pub fn left(mut self, padding: u16) -> Self {
        self.left = padding;
        self
    }

    fn inner_area(&self, area: Rect) -> Rect {
        let horizontal = self.left.saturating_add(self.right);
        let vertical = self.top.saturating_add(self.bottom);
        let width = area.width.saturating_sub(horizontal);
        let height = area.height.saturating_sub(vertical);
        let x = area.x.saturating_add(self.left.min(area.width));
        let y = area.y.saturating_add(self.top.min(area.height));
        Rect::new(x, y, width, height)
    }
}

impl<W: Widget> Widget for Padded<W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = self.inner_area(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        self.inner.render(inner, buf);
    }
}

impl<W: StatefulWidget> StatefulWidget for Padded<W> {
    type State = W::State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = self.inner_area(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        self.inner.render(inner, buf, state);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::Line;
    use ratatui::widgets::Paragraph;

    use super::*;

    /// Minimal stateful widget that writes a single character driven by state.
    struct StubStateful;
    struct StubState {
        symbol: char,
    }

    impl StatefulWidget for StubStateful {
        type State = StubState;
        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            if area.width > 0 && area.height > 0 {
                buf[(area.x, area.y)].set_symbol(&state.symbol.to_string());
            }
        }
    }

    fn fill_with_letter_a(area: Rect) -> Buffer {
        let mut buf = Buffer::empty(area);
        Paragraph::new(Line::from("AAAAA")).render(area, &mut buf);
        buf
    }

    // ----- Hideable ---------------------------------------------------------

    #[test]
    fn hideable_visible_renders_inner() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        Hideable::new(Paragraph::new("hello")).render(area, &mut buf);
        let text: String = (0..5).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(text, "hello");
    }

    #[test]
    fn hideable_hidden_does_not_touch_buffer() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        Hideable::new(Paragraph::new("hello"))
            .hidden()
            .render(area, &mut buf);
        for x in 0..5 {
            assert_eq!(buf[(x, 0)].symbol(), " ");
        }
    }

    #[test]
    fn hideable_visible_setter_round_trips() {
        let h = Hideable::new(Paragraph::new("x")).visible(false);
        assert!(!h.is_visible());
        let h = h.visible(true);
        assert!(h.is_visible());
    }

    #[test]
    fn hideable_stateful_skips_when_hidden() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        let mut state = StubState { symbol: 'X' };
        Hideable::new(StubStateful)
            .hidden()
            .render(area, &mut buf, &mut state);
        assert_eq!(buf[(0, 0)].symbol(), " ");
    }

    #[test]
    fn hideable_stateful_renders_when_visible() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        let mut state = StubState { symbol: 'X' };
        Hideable::new(StubStateful).render(area, &mut buf, &mut state);
        assert_eq!(buf[(0, 0)].symbol(), "X");
    }

    // ----- Disableable ------------------------------------------------------

    #[test]
    fn disableable_enabled_does_not_dim() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = fill_with_letter_a(area);
        // Wrap a no-op render to verify enabled path skips the dim pass.
        Disableable::new(Paragraph::new(Line::styled("AAAAA", Style::default())))
            .render(area, &mut buf);
        for x in 0..5 {
            assert!(!buf[(x, 0)].modifier.contains(Modifier::DIM));
        }
    }

    #[test]
    fn disableable_disabled_applies_dim_to_area() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        Disableable::new(Paragraph::new("hello"))
            .disabled()
            .render(area, &mut buf);
        for x in 0..5 {
            assert!(
                buf[(x, 0)].modifier.contains(Modifier::DIM),
                "cell {x} not dim",
            );
        }
    }

    #[test]
    fn disableable_disabled_preserves_inner_symbols() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        Disableable::new(Paragraph::new("hello"))
            .disabled()
            .render(area, &mut buf);
        let text: String = (0..5).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(text, "hello");
    }

    #[test]
    fn disableable_enabled_setter_round_trips() {
        let d = Disableable::new(Paragraph::new("x")).enabled(false);
        assert!(!d.is_enabled());
        let d = d.enabled(true);
        assert!(d.is_enabled());
    }

    #[test]
    fn disableable_stateful_dims_when_disabled() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        let mut state = StubState { symbol: 'X' };
        Disableable::new(StubStateful)
            .disabled()
            .render(area, &mut buf, &mut state);
        assert_eq!(buf[(0, 0)].symbol(), "X");
        assert!(buf[(0, 0)].modifier.contains(Modifier::DIM));
    }

    // ----- Padded -----------------------------------------------------------

    #[test]
    fn padded_default_is_zero_padding() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        Padded::new(Paragraph::new("hello")).render(area, &mut buf);
        let text: String = (0..5).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(text, "hello");
    }

    #[test]
    fn padded_horizontal_insets_left_and_right() {
        let area = Rect::new(0, 0, 7, 1);
        let mut buf = Buffer::empty(area);
        Padded::new(Paragraph::new("xxxxx"))
            .horizontal(1)
            .render(area, &mut buf);
        // Width 7 - 2 = 5 inner; columns 1..6 hold xxxxx, columns 0 and 6 blank.
        assert_eq!(buf[(0, 0)].symbol(), " ");
        assert_eq!(buf[(6, 0)].symbol(), " ");
        let inner: String = (1..6).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(inner, "xxxxx");
    }

    #[test]
    fn padded_vertical_insets_top_and_bottom() {
        let area = Rect::new(0, 0, 3, 3);
        let mut buf = Buffer::empty(area);
        Padded::new(Paragraph::new("abc"))
            .vertical(1)
            .render(area, &mut buf);
        // Top row blank, middle row has content, bottom row blank.
        for x in 0..3 {
            assert_eq!(buf[(x, 0)].symbol(), " ");
            assert_eq!(buf[(x, 2)].symbol(), " ");
        }
        let mid: String = (0..3).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert_eq!(mid, "abc");
    }

    #[test]
    fn padded_all_sides() {
        let area = Rect::new(0, 0, 5, 5);
        let mut buf = Buffer::empty(area);
        Padded::new(Paragraph::new("xyz"))
            .all(1)
            .render(area, &mut buf);
        // Inner area is (1,1) sized 3x3; Paragraph renders top-left.
        let top_inner: String = (1..4).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert_eq!(top_inner, "xyz");
        // Padding row 0 stays blank.
        for x in 0..5 {
            assert_eq!(buf[(x, 0)].symbol(), " ");
        }
    }

    #[test]
    fn padded_per_side_overrides() {
        let area = Rect::new(0, 0, 6, 4);
        let mut buf = Buffer::empty(area);
        Padded::new(Paragraph::new("hi"))
            .padding(1, 2, 0, 1)
            .render(area, &mut buf);
        // top=1 right=2 bottom=0 left=1 -> inner at (1,1) sized 3x3
        assert_eq!(buf[(1, 1)].symbol(), "h");
        assert_eq!(buf[(2, 1)].symbol(), "i");
    }

    #[test]
    fn padded_oversized_padding_is_noop() {
        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        Padded::new(Paragraph::new("hello"))
            .horizontal(5)
            .render(area, &mut buf);
        for x in 0..3 {
            assert_eq!(buf[(x, 0)].symbol(), " ");
        }
    }

    #[test]
    fn padded_stateful_inner_area_is_inset() {
        let area = Rect::new(0, 0, 3, 3);
        let mut buf = Buffer::empty(area);
        let mut state = StubState { symbol: 'X' };
        Padded::new(StubStateful)
            .all(1)
            .render(area, &mut buf, &mut state);
        // Inner top-left is (1,1).
        assert_eq!(buf[(1, 1)].symbol(), "X");
        assert_eq!(buf[(0, 0)].symbol(), " ");
    }
}
