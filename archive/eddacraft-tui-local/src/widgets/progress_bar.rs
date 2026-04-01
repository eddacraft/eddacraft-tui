use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::theme::Theme;

pub struct ProgressBar<'a, T: Theme> {
    theme: &'a T,
    block: Option<Block<'a>>,
    label: Option<&'a str>,
}

#[derive(Debug, Default, Clone)]
pub struct ProgressBarState {
    pub current: u64,
    pub total: u64,
}

impl ProgressBarState {
    #[allow(clippy::cast_precision_loss)]
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.current as f64 / self.total as f64).clamp(0.0, 1.0)
    }
}

impl<'a, T: Theme> ProgressBar<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            block: None,
            label: None,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label.into();
        self
    }
}

impl<T: Theme> StatefulWidget for ProgressBar<'_, T> {
    type State = ProgressBarState;

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

        let fraction = state.fraction();
        let bar_width = inner.width as usize;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let filled = (bar_width as f64 * fraction) as usize;

        let bar: String = "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let pct = (fraction * 100.0) as u64;
        let display = if let Some(label) = self.label {
            format!("{label}: {bar} {pct}%")
        } else {
            format!("{bar} {pct}%")
        };

        let line = Line::styled(display, self.theme.base());
        line.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_calculation() {
        let state = ProgressBarState {
            current: 50,
            total: 100,
        };
        let diff = (state.fraction() - 0.5).abs();
        assert!(diff < f64::EPSILON);
    }

    #[test]
    fn fraction_clamps_to_one() {
        let state = ProgressBarState {
            current: 200,
            total: 100,
        };
        let diff = (state.fraction() - 1.0).abs();
        assert!(diff < f64::EPSILON);
    }

    #[test]
    fn fraction_zero_when_empty() {
        let state = ProgressBarState {
            current: 0,
            total: 0,
        };
        assert!(state.fraction().abs() < f64::EPSILON);
    }
}
