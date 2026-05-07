//! Themed wrapper around [`tui_big_text::BigText`] for branded banners and
//! splash screens. Available behind the `big-text` Cargo feature.
//!
//! **Caveat:** the upstream `tui-big-text` crate has a known limitation
//! around multi-codepoint graphemes (e.g. emoji with skin-tone modifiers,
//! ZWJ sequences). Pass plain ASCII / single-codepoint text until upstream
//! resolves the issue, or accept the risk of a panic on render.
//!
//! ```rust,no_run
//! # #[cfg(feature = "big-text")]
//! # {
//! use eddacraft_tui::theme::EddaCraftTheme;
//! use eddacraft_tui::widgets::big_banner::BigBanner;
//! use tui_big_text::PixelSize;
//!
//! let theme = EddaCraftTheme;
//! let _ = BigBanner::new(&theme, "EDDA")
//!     .pixel_size(PixelSize::Quadrant)
//!     .centered();
//! # }
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::Widget;
use tui_big_text::{BigText, PixelSize};

use crate::theme::Theme;

pub struct BigBanner<'a, T: Theme> {
    theme: &'a T,
    text: &'a str,
    pixel_size: PixelSize,
    alignment: Alignment,
    accent: bool,
}

impl<'a, T: Theme> BigBanner<'a, T> {
    pub fn new(theme: &'a T, text: &'a str) -> Self {
        Self {
            theme,
            text,
            pixel_size: PixelSize::Quadrant,
            alignment: Alignment::Left,
            accent: true,
        }
    }

    /// Choose how 'big' a single 8x8 glyph is in cells. Default
    /// [`PixelSize::Quadrant`] (sub-cell, fits more text into the area).
    #[must_use]
    pub fn pixel_size(mut self, pixel_size: PixelSize) -> Self {
        self.pixel_size = pixel_size;
        self
    }

    #[must_use]
    pub fn centered(mut self) -> Self {
        self.alignment = Alignment::Center;
        self
    }

    #[must_use]
    pub fn left_aligned(mut self) -> Self {
        self.alignment = Alignment::Left;
        self
    }

    #[must_use]
    pub fn right_aligned(mut self) -> Self {
        self.alignment = Alignment::Right;
        self
    }

    /// When `true` (default), render in `theme.title()` (accent + bold). When
    /// `false`, render in the base foreground style.
    #[must_use]
    pub fn accent(mut self, accent: bool) -> Self {
        self.accent = accent;
        self
    }
}

impl<T: Theme> Widget for BigBanner<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let style = if self.accent {
            self.theme.title()
        } else {
            self.theme.base()
        };
        let lines: Vec<Line<'_>> = self.text.lines().map(Line::from).collect();
        let banner = BigText::builder()
            .pixel_size(self.pixel_size)
            .alignment(self.alignment)
            .style(style)
            .lines(lines)
            .build();
        banner.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn renders_without_panic_in_small_area() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 4);
        let mut buf = Buffer::empty(area);
        BigBanner::new(&theme, "EDDA").render(area, &mut buf);
        // We can't easily assert the bitmap, but at least one cell should be
        // touched somewhere in the area.
        let any_painted = (0..area.width).any(|x| {
            (0..area.height).any(|y| {
                buf[(x, y)].symbol() != " " || buf[(x, y)].fg.eq(&theme.title().fg.unwrap())
            })
        });
        assert!(any_painted, "expected at least one painted cell");
    }

    #[test]
    fn zero_area_is_a_noop() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 5));
        BigBanner::new(&theme, "X").render(area, &mut buf);
    }

    #[test]
    fn alignment_setters_round_trip() {
        let theme = EddaCraftTheme;
        let banner = BigBanner::new(&theme, "x").centered();
        assert_eq!(banner.alignment, Alignment::Center);
        let banner = banner.right_aligned();
        assert_eq!(banner.alignment, Alignment::Right);
        let banner = banner.left_aligned();
        assert_eq!(banner.alignment, Alignment::Left);
    }
}
