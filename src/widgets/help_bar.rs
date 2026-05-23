//! Auto-generated key hint footer driven by [`KeyHandler`] bindings.
//!
//! Inspired by Charm's `bubbles/help`. The widget renders an inline
//! `[keys] label` summary so applications never have to hand-author help text
//! that drifts from the actual keymap.
//!
//! ```rust,no_run
//! # use eddacraft_tui::prelude::*;
//! # use eddacraft_tui::widgets::help_bar::HelpBar;
//! # let theme = EddaCraftTheme;
//! let _ = HelpBar::new(&theme); // bindings default to `KeyHandler::default_bindings()`
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::keyboard::{Binding, KeyHandler};
use crate::theme::Theme;

pub struct HelpBar<'a, T: Theme> {
    theme: &'a T,
    bindings: &'a [Binding],
    separator: &'a str,
    bracket_keys: bool,
}

impl<'a, T: Theme> HelpBar<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            bindings: KeyHandler::default_bindings(),
            separator: "  ",
            bracket_keys: true,
        }
    }

    /// Override the bindings shown — useful for surface-specific help (e.g.
    /// only the keys that are valid in the current screen).
    #[must_use]
    pub fn bindings(mut self, bindings: &'a [Binding]) -> Self {
        self.bindings = bindings;
        self
    }

    #[must_use]
    pub fn separator(mut self, separator: &'a str) -> Self {
        self.separator = separator;
        self
    }

    /// When `false`, render keys without surrounding `[...]`.
    #[must_use]
    pub fn bracket_keys(mut self, enabled: bool) -> Self {
        self.bracket_keys = enabled;
        self
    }
}

impl<T: Theme> Widget for HelpBar<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        buf.set_style(area, self.theme.base());

        let key_style = self.theme.title();
        let label_style = self.theme.disabled();

        let mut spans: Vec<Span> = Vec::with_capacity(self.bindings.len() * 4);
        for (i, binding) in self.bindings.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(self.separator, label_style));
            }
            if self.bracket_keys {
                spans.push(Span::styled("[", label_style));
                spans.push(Span::styled(binding.keys, key_style));
                spans.push(Span::styled("]", label_style));
            } else {
                spans.push(Span::styled(binding.keys, key_style));
            }
            spans.push(Span::styled(" ", label_style));
            spans.push(Span::styled(binding.label, label_style));
        }

        Line::from(spans).render(Rect::new(area.x, area.y, area.width, 1), buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::Action;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn empty_bindings_renders_blank_line() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        HelpBar::new(&theme).bindings(&[]).render(area, &mut buf);
        for x in 0..20 {
            assert_eq!(buf[(x, 0)].symbol(), " ");
        }
    }

    #[test]
    fn renders_bracketed_key_and_label() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let bindings = [Binding {
            keys: "q",
            action: Action::Quit,
            label: "Quit",
        }];
        HelpBar::new(&theme)
            .bindings(&bindings)
            .render(area, &mut buf);

        let text: String = (0..10).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(text.starts_with("[q] Quit"), "got: {text:?}");
    }

    #[test]
    fn separator_appears_between_bindings() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        let bindings = [
            Binding {
                keys: "a",
                action: Action::Up,
                label: "A",
            },
            Binding {
                keys: "b",
                action: Action::Down,
                label: "B",
            },
        ];
        HelpBar::new(&theme)
            .bindings(&bindings)
            .separator(" | ")
            .render(area, &mut buf);

        let text: String = (0..40).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(text.contains("[a] A | [b] B"), "got: {text:?}");
    }

    #[test]
    fn bracket_keys_false_renders_keys_without_brackets() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let bindings = [Binding {
            keys: "q",
            action: Action::Quit,
            label: "Quit",
        }];
        HelpBar::new(&theme)
            .bindings(&bindings)
            .bracket_keys(false)
            .render(area, &mut buf);

        let text: String = (0..10).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(text.starts_with("q Quit"), "got: {text:?}");
    }

    #[test]
    fn key_span_uses_title_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let bindings = [Binding {
            keys: "q",
            action: Action::Quit,
            label: "Quit",
        }];
        HelpBar::new(&theme)
            .bindings(&bindings)
            .render(area, &mut buf);

        // Cell at index 1 is the "q" itself ([q]); should use accent fg.
        let expected_fg = theme.title().fg.unwrap();
        assert_eq!(buf[(1, 0)].fg, expected_fg);
    }

    #[test]
    fn default_bindings_render_without_panic() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        HelpBar::new(&theme).render(area, &mut buf);
    }

    #[test]
    fn zero_height_area_is_a_noop() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 20, 0);
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        HelpBar::new(&theme).render(area, &mut buf);
    }
}
