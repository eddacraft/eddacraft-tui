use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use crate::theme::{DASHED_BORDER, Theme};

#[derive(Debug, Clone, Copy, Default)]
pub enum ContainerVariant {
    Primary,
    #[default]
    Secondary,
    Subtle,
    /// A box whose state could not be determined: the unknown status colour
    /// on a dashed border ([`DASHED_BORDER`]), so it reads as unknown without
    /// colour too.
    Unknown,
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
            ContainerVariant::Unknown => (
                BorderType::Plain,
                self.theme.status_unknown(),
                self.theme.status_unknown(),
            ),
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style);
        if matches!(self.variant, ContainerVariant::Unknown) {
            block = block.border_set(DASHED_BORDER);
        }

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

    #[test]
    fn unknown_variant_renders_dashed_border_in_the_unknown_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 6, 3);
        let mut buf = Buffer::empty(area);

        Container::new(&theme)
            .variant(ContainerVariant::Unknown)
            .render(area, &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "┌");
        assert_eq!(buf[(1, 0)].symbol(), "╌");
        assert_eq!(buf[(0, 1)].symbol(), "╎");
        assert_eq!(buf[(1, 0)].fg, theme.status_unknown().fg.unwrap());
    }
}
