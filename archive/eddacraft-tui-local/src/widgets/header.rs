use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;

pub struct Header<'a, T: Theme> {
    theme: &'a T,
    title: &'a str,
    subtitle: Option<&'a str>,
    version: Option<&'a str>,
}

impl<'a, T: Theme> Header<'a, T> {
    pub fn new(title: &'a str, theme: &'a T) -> Self {
        Self {
            theme,
            title,
            subtitle: None,
            version: None,
        }
    }

    #[must_use]
    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    #[must_use]
    pub fn version(mut self, version: &'a str) -> Self {
        self.version = version.into();
        self
    }
}

impl<T: Theme> Widget for Header<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let separator = "━".repeat(area.width as usize);
        Line::styled(separator, self.theme.border_unfocused())
            .render(Rect::new(area.x, area.y, area.width, 1), buf);

        if area.height < 2 {
            return;
        }

        let uppercase_title = self.title.to_uppercase();
        let mut title_spans = vec![Span::styled(uppercase_title, self.theme.title())];
        if let Some(version) = self.version {
            title_spans.push(Span::styled(format!(" v{version}"), self.theme.disabled()));
        }
        Line::from(title_spans).render(Rect::new(area.x, area.y + 1, area.width, 1), buf);

        if let Some(subtitle) = self.subtitle {
            if area.height < 3 {
                return;
            }
            Line::styled(subtitle, self.theme.disabled())
                .render(Rect::new(area.x, area.y + 2, area.width, 1), buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::Widget;

    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn renders_separator_and_title() {
        let theme = EddaCraftTheme;
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 3));
        Header::new("Anvil", &theme).render(Rect::new(0, 0, 40, 3), &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "━");
        assert_eq!(buf[(0, 1)].symbol(), "A");
    }

    #[test]
    fn renders_subtitle_when_present() {
        let theme = EddaCraftTheme;
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 3));
        Header::new("Anvil", &theme)
            .subtitle("Deterministic automation")
            .render(Rect::new(0, 0, 40, 3), &mut buf);

        assert_eq!(buf[(0, 2)].symbol(), "D");
    }
}
