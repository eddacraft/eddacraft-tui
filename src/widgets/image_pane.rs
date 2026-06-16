//! Themed wrapper around [`ratatui_image::Image`] that renders an image with
//! eddacraft-styled chrome (optional rounded border + title). Available behind
//! the `image` Cargo feature.
//!
//! Image protocol detection (Kitty / Sixel / iTerm2 / halfblocks) lives in
//! `ratatui_image::picker::Picker` — applications should construct one once at
//! startup and reuse it. This widget just renders an existing
//! [`Protocol`](ratatui_image::protocol::Protocol).
//!
//! Validate image dimensions and source before constructing a `Protocol` —
//! the `image` crate has a parser surface that has carried decoder
//! advisories in the past; keep the dependency current and treat
//! attacker-controlled paths with care.
//!
//! ```rust,no_run
//! # #[cfg(feature = "image")]
//! # fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use eddacraft_tui::theme::EddaCraftTheme;
//! use eddacraft_tui::widgets::image_pane::ImagePane;
//! use ratatui_image::picker::Picker;
//!
//! let theme = EddaCraftTheme;
//! let picker = Picker::from_query_stdio()?;
//! let dyn_img = image::open("logo.png")?;
//! let protocol = picker.new_protocol(
//!     dyn_img,
//!     ratatui::layout::Rect::new(0, 0, 40, 20).into(),
//!     ratatui_image::Resize::Fit(None),
//! )?;
//! let _ = ImagePane::new(&theme, &protocol).title("Logo");
//! # Ok(())
//! # }
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use ratatui_image::Image;
use ratatui_image::protocol::Protocol;

use crate::theme::Theme;

pub struct ImagePane<'a, T: Theme> {
    theme: &'a T,
    protocol: &'a Protocol,
    title: Option<&'a str>,
    bordered: bool,
}

impl<'a, T: Theme> ImagePane<'a, T> {
    pub fn new(theme: &'a T, protocol: &'a Protocol) -> Self {
        Self {
            theme,
            protocol,
            title: None,
            bordered: true,
        }
    }

    /// Set the title shown in the top border. The title is only rendered
    /// when [`Self::bordered`] is `true` (the default); calling `.title(...)`
    /// on an unbordered pane is a no-op for clarity, surfaced via a debug
    /// assertion in [`<ImagePane as Widget>::render`].
    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Disable the surrounding border. Default is bordered.
    ///
    /// Note: title (if any) is only rendered when bordered. Pair with a
    /// border, or render the title separately above the pane.
    #[must_use]
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }
}

impl<T: Theme> Widget for ImagePane<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        debug_assert!(
            self.bordered || self.title.is_none(),
            "ImagePane: title is only rendered when bordered(true); the supplied title would be silently dropped",
        );
        let inner = if self.bordered {
            let mut block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(self.theme.border_focused())
                .style(self.theme.base());
            if let Some(title) = self.title {
                block = block.title(Line::styled(format!(" {title} "), self.theme.title()));
            }
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        Image::new(self.protocol).render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::EddaCraftTheme;
    use ratatui::layout::Rect;
    use ratatui_image::Resize;
    use ratatui_image::picker::Picker;

    fn tiny_protocol(area: Rect) -> Protocol {
        // 2x2 RGB image so the test doesn't depend on any external file.
        let mut img = image::RgbImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        img.put_pixel(1, 1, image::Rgb([0, 255, 0]));
        let dynamic = image::DynamicImage::ImageRgb8(img);
        let picker = Picker::halfblocks();
        picker
            .new_protocol(dynamic, area.into(), Resize::Fit(None))
            .expect("protocol")
    }

    #[test]
    fn renders_border_around_image() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 10, 6);
        let proto = tiny_protocol(Rect::new(0, 0, 8, 4));
        let mut buf = Buffer::empty(area);
        ImagePane::new(&theme, &proto).render(area, &mut buf);
        // Rounded border corner at (0,0).
        assert_eq!(buf[(0, 0)].symbol(), "╭");
        assert_eq!(buf[(9, 5)].symbol(), "╯");
    }

    #[test]
    fn title_appears_on_top_border() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 14, 6);
        let proto = tiny_protocol(Rect::new(0, 0, 12, 4));
        let mut buf = Buffer::empty(area);
        ImagePane::new(&theme, &proto)
            .title("Logo")
            .render(area, &mut buf);
        let top: String = (0..14).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(top.contains("Logo"), "got: {top:?}");
    }

    #[test]
    fn bordered_false_skips_border() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 10, 6);
        let proto = tiny_protocol(area);
        let mut buf = Buffer::empty(area);
        ImagePane::new(&theme, &proto)
            .bordered(false)
            .render(area, &mut buf);
        assert_ne!(buf[(0, 0)].symbol(), "╭");
    }

    #[test]
    fn zero_area_is_a_noop() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 0, 0);
        let proto = tiny_protocol(Rect::new(0, 0, 4, 4));
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 5));
        ImagePane::new(&theme, &proto).render(area, &mut buf);
    }
}
