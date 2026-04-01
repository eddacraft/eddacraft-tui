use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, Default)]
pub enum ContainerVariant {
    Primary,
    #[default]
    Secondary,
    Subtle,
}

pub struct Container<'a, T: Theme> {
    theme: &'a T,
    title: Option<&'a str>,
    variant: ContainerVariant,
}

impl<'a, T: Theme> Container<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            title: None,
            variant: ContainerVariant::Secondary,
        }
    }

    #[must_use]
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title.into();
        self
    }

    #[must_use]
    pub fn variant(mut self, variant: ContainerVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn to_block(&self) -> Block<'a> {
        let (border_type, border_style, title_style) = match self.variant {
            ContainerVariant::Primary => (
                BorderType::Double,
                self.theme.border_focused(),
                self.theme.title(),
            ),
            ContainerVariant::Secondary => (
                BorderType::Plain,
                self.theme.border_focused(),
                self.theme.title(),
            ),
            ContainerVariant::Subtle => (
                BorderType::Rounded,
                self.theme.border_unfocused(),
                self.theme.disabled(),
            ),
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style);

        if let Some(title) = self.title {
            block = block.title(Line::styled(title, title_style));
        }

        block
    }

    #[must_use]
    pub fn inner(&self, area: Rect) -> Rect {
        self.to_block().inner(area)
    }
}

impl<T: Theme> Widget for Container<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.to_block().render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::Widget;

    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn primary_variant_renders_double_border() {
        let theme = EddaCraftTheme;
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 3));

        Container::new(&theme)
            .variant(ContainerVariant::Primary)
            .render(Rect::new(0, 0, 6, 3), &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "╔");
    }

    #[test]
    fn subtle_variant_renders_rounded_border() {
        let theme = EddaCraftTheme;
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 3));

        Container::new(&theme)
            .variant(ContainerVariant::Subtle)
            .render(Rect::new(0, 0, 6, 3), &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "╭");
    }
}
