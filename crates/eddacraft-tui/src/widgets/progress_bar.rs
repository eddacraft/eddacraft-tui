use std::fmt;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::animation::advance;
use crate::theme::Theme;
use crate::widgets::{AnimatedF64, animated_f64};

pub struct ProgressBar<'a, T: Theme> {
    theme: &'a T,
    block: Option<Block<'a>>,
    label: Option<&'a str>,
}

#[non_exhaustive]
pub struct ProgressBarState {
    pub current: u64,
    pub total: u64,
    pub(crate) display_fraction: AnimatedF64,
    pub(crate) target_fraction: f64,
}

impl Default for ProgressBarState {
    fn default() -> Self {
        Self {
            current: 0,
            total: 0,
            display_fraction: animated_f64(0.0),
            target_fraction: 0.0,
        }
    }
}

impl Clone for ProgressBarState {
    fn clone(&self) -> Self {
        let frac = self.fraction();
        Self {
            current: self.current,
            total: self.total,
            display_fraction: animated_f64(frac),
            target_fraction: self.target_fraction,
        }
    }
}

impl fmt::Debug for ProgressBarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgressBarState")
            .field("current", &self.current)
            .field("total", &self.total)
            .field("fraction", &self.fraction())
            .finish_non_exhaustive()
    }
}

impl ProgressBarState {
    #[allow(clippy::cast_precision_loss)]
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.current as f64 / self.total as f64).clamp(0.0, 1.0)
    }

    /// Returns the current visually-interpolated fraction (smoothed by easing).
    ///
    /// This value transitions smoothly toward [`Self::fraction()`] each time the
    /// widget is rendered, provided [`crate::animation::animate_tick`] is called in
    /// the event loop.
    pub fn display_fraction(&self) -> f64 {
        *self.display_fraction
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
        let inner =
            super::render_block(self.block.as_ref(), self.theme.border_focused(), area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Sync animation target when the logical fraction changes.
        let target = state.fraction();
        if (target - state.target_fraction).abs() > f64::EPSILON {
            state.display_fraction.to(target);
            state.target_fraction = target;
        }
        advance(&mut state.display_fraction);

        let fraction = (*state.display_fraction).clamp(0.0, 1.0);
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
            ..Default::default()
        };
        let diff = (state.fraction() - 0.5).abs();
        assert!(diff < f64::EPSILON);
    }

    #[test]
    fn fraction_clamps_to_one() {
        let state = ProgressBarState {
            current: 200,
            total: 100,
            ..Default::default()
        };
        let diff = (state.fraction() - 1.0).abs();
        assert!(diff < f64::EPSILON);
    }

    #[test]
    fn fraction_zero_when_empty() {
        let state = ProgressBarState {
            current: 0,
            total: 0,
            ..Default::default()
        };
        assert!(state.fraction().abs() < f64::EPSILON);
    }

    #[test]
    fn display_fraction_converges_after_animation_duration() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;

        use crate::theme::EddaCraftTheme;
        use crate::widgets::ANIM_DURATION_MS;

        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        let mut state = ProgressBarState {
            current: 50,
            total: 100,
            ..Default::default()
        };

        // First render primes the animation toward the new target (0.5).
        ProgressBar::new(&theme).render(area, &mut buf, &mut state);
        let first = state.display_fraction();

        // Advance the animate clock past the configured duration and re-render.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let advance = ANIM_DURATION_MS as usize + 1;
        crate::animation::animate_tick(advance);
        ProgressBar::new(&theme).render(area, &mut buf, &mut state);

        let converged = state.display_fraction();
        assert!(
            first <= converged,
            "expected display_fraction to move toward target, got {first} -> {converged}"
        );
        let diff = (converged - state.fraction()).abs();
        assert!(
            diff < 1e-6,
            "expected convergence within duration, diff={diff}"
        );
    }
}
