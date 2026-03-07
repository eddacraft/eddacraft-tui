use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::theme::Theme;

pub struct Confirm<'a, T: Theme> {
    theme: &'a T,
    message: &'a str,
    block: Option<Block<'a>>,
}

#[derive(Debug)]
pub struct ConfirmState {
    pub selected: bool,
    pub confirmed: Option<bool>,
}

impl Default for ConfirmState {
    fn default() -> Self {
        Self {
            selected: true,
            confirmed: None,
        }
    }
}

impl ConfirmState {
    pub fn toggle(&mut self) {
        self.selected = !self.selected;
    }

    pub fn confirm(&mut self) {
        self.confirmed = Some(self.selected);
    }

    pub fn confirm_yes(&mut self) {
        self.confirmed = Some(true);
    }

    pub fn confirm_no(&mut self) {
        self.confirmed = Some(false);
    }

    #[must_use]
    pub fn is_confirmed(&self) -> Option<bool> {
        self.confirmed
    }

    pub fn reset(&mut self) {
        self.confirmed = None;
    }
}

impl<'a, T: Theme> Confirm<'a, T> {
    pub fn new(message: &'a str, theme: &'a T) -> Self {
        Self {
            theme,
            message,
            block: None,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }
}

impl<T: Theme> StatefulWidget for Confirm<'_, T> {
    type State = ConfirmState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = if let Some(block) = &self.block {
            let styled = block.clone().border_style(self.theme.border_focused());
            let inner = styled.inner(area);
            styled.render(area, buf);
            inner
        } else {
            area
        };

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let yes_style = if state.selected {
            self.theme.title().add_modifier(Modifier::BOLD)
        } else {
            self.theme.disabled()
        };
        let no_style = if state.selected {
            self.theme.disabled()
        } else {
            self.theme.status_error().add_modifier(Modifier::BOLD)
        };

        let line = Line::from(vec![
            Span::styled(format!("{} ", self.message), self.theme.base()),
            Span::styled("Yes", yes_style),
            Span::styled(" / ", self.theme.disabled()),
            Span::styled("No", no_style),
            Span::styled(" (y/n)", self.theme.disabled()),
        ]);
        line.render(Rect::new(inner.x, inner.y, inner.width, 1), buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_switches_selection() {
        let mut state = ConfirmState::default();
        assert!(state.selected);
        state.toggle();
        assert!(!state.selected);
    }

    #[test]
    fn confirm_yes_and_no_set_values() {
        let mut state = ConfirmState::default();
        state.confirm_yes();
        assert_eq!(state.is_confirmed(), Some(true));

        state.confirm_no();
        assert_eq!(state.is_confirmed(), Some(false));
    }

    #[test]
    fn reset_clears_confirmation() {
        let mut state = ConfirmState::default();
        state.confirm();
        assert_eq!(state.is_confirmed(), Some(true));
        state.reset();
        assert_eq!(state.is_confirmed(), None);
    }
}
