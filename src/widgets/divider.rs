use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Widget;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, Default)]
pub enum DividerVariant {
    Heavy,
    #[default]
    Light,
}

pub struct Divider<'a, T: Theme> {
    theme: &'a T,
    variant: DividerVariant,
    character: Option<char>,
}

impl<'a, T: Theme> Divider<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            variant: DividerVariant::Light,
            character: None,
        }
    }

    #[must_use]
    pub fn variant(mut self, variant: DividerVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn character(mut self, character: char) -> Self {
        self.character = character.into();
        self
    }
}

impl<T: Theme> Widget for Divider<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let (default_character, style) = match self.variant {
            DividerVariant::Heavy => ('━', self.theme.border_focused()),
            DividerVariant::Light => ('─', self.theme.border_unfocused()),
        };

        let character = self.character.unwrap_or(default_character);
        let line = character.to_string().repeat(area.width as usize);
        Line::styled(line, style).render(Rect::new(area.x, area.y, area.width, 1), buf);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::Widget;

    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn light_variant_renders_light_character() {
        let theme = EddaCraftTheme;
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));

        Divider::new(&theme)
            .variant(DividerVariant::Light)
            .render(Rect::new(0, 0, 5, 1), &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "─");
    }

    #[test]
    fn custom_character_overrides_variant() {
        let theme = EddaCraftTheme;
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));

        Divider::new(&theme)
            .variant(DividerVariant::Heavy)
            .character('=')
            .render(Rect::new(0, 0, 5, 1), &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "=");
    }
}
