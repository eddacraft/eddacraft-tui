use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::theme::Theme;

const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub struct Spinner<'a, T: Theme> {
    theme: &'a T,
    label: Option<&'a str>,
}

#[derive(Debug, Default)]
pub struct SpinnerState {
    pub frame: usize,
}

impl SpinnerState {
    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % FRAMES.len();
    }
}

impl<'a, T: Theme> Spinner<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self { theme, label: None }
    }

    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label.into();
        self
    }
}

impl<T: Theme> StatefulWidget for Spinner<'_, T> {
    type State = SpinnerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        state.frame %= FRAMES.len();
        let frame_char = FRAMES[state.frame];

        let line = if let Some(label) = self.label {
            Line::from(vec![
                Span::styled(frame_char.to_string(), self.theme.title()),
                Span::raw(" "),
                Span::styled(label, self.theme.disabled()),
            ])
        } else {
            Line::from(vec![Span::styled(
                frame_char.to_string(),
                self.theme.title(),
            )])
        };

        let row_area = Rect::new(area.x, area.y, area.width, 1);
        line.render(row_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_frame_is_zero() {
        let state = SpinnerState::default();
        assert_eq!(state.frame, 0);
    }

    #[test]
    fn tick_advances_frame() {
        let mut state = SpinnerState::default();
        state.tick();
        assert_eq!(state.frame, 1);
    }

    #[test]
    fn tick_wraps_around() {
        let mut state = SpinnerState {
            frame: FRAMES.len() - 1,
        };
        state.tick();
        assert_eq!(state.frame, 0);
    }
}
