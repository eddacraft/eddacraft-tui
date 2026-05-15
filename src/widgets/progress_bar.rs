use std::fmt;

use animate::Animate;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, StatefulWidget, Widget};

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
    /// widget is rendered, provided [`animate::tick`] is called in the event loop.
    pub fn display_fraction(&self) -> f64 {
        *self.display_fraction
    }
}

/// Half of one logical step expressed as a fraction of `total`. No floor:
/// once `total` exceeds f64's 53-bit mantissa, `1.0 / total` rides below
/// `f64::EPSILON`, and an EPSILON floor would clamp the threshold back
/// *above* the per-unit delta we want to detect — re-introducing the
/// original freeze. The comparator stays tolerant of true rounding noise
/// because at those totals consecutive fractions are themselves
/// indistinguishable in f64.
#[allow(clippy::cast_precision_loss)]
fn step_threshold(total: u64) -> f64 {
    if total == 0 {
        return f64::EPSILON;
    }
    1.0 / (total as f64) * 0.5
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
        self.block = Some(block);
        self
    }

    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
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
        // Threshold scales with `total` so single-unit increments still
        // register at very large counters — at `total ≥ 9×10¹⁵`, a 1-unit
        // delta is smaller than `f64::EPSILON` and would otherwise be
        // silently dropped, freezing the bar.
        let target = state.fraction();
        let threshold = step_threshold(state.total);
        if (target - state.target_fraction).abs() > threshold {
            state.display_fraction.set(target);
            state.target_fraction = target;
        }
        state.display_fraction.update();

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
    fn target_advances_with_unit_increment_at_very_large_total() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;

        use crate::theme::EddaCraftTheme;

        // `total = 9 * 10^15` sits just below f64's 53-bit mantissa: two
        // consecutive `current` values map to distinct f64 fractions, so a
        // single-unit increment produces a non-zero delta. The previous
        // `f64::EPSILON` threshold dropped that delta; this test now also
        // catches the EPSILON-floor regression by asserting strict `>`.
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        let mut state = ProgressBarState {
            current: 4_500_000_000_000_000,
            total: 9_000_000_000_000_000,
            ..Default::default()
        };
        ProgressBar::new(&theme).render(area, &mut buf, &mut state);
        let first_target = state.target_fraction;

        state.current = state.current.saturating_add(1);
        ProgressBar::new(&theme).render(area, &mut buf, &mut state);

        assert!(
            state.target_fraction > first_target,
            "target_fraction must advance on unit increment at large totals: \
             first={first_target} after={target}",
            target = state.target_fraction,
        );
    }

    #[test]
    fn step_threshold_scales_with_total() {
        // At total=2, a step is 0.5 — threshold is 0.25.
        assert!(step_threshold(2) > f64::EPSILON);
        // At total=u64::MAX the natural step underflows below EPSILON; the
        // helper now returns the true sub-EPSILON value rather than clamping
        // (the prior EPSILON floor re-created the original bug for
        // total > ~4.5e15).
        let extreme = step_threshold(u64::MAX);
        assert!(extreme > 0.0);
        assert!(extreme < f64::EPSILON);
        // total=0 falls back to EPSILON (matches the legacy behaviour).
        assert!((step_threshold(0) - f64::EPSILON).abs() < f64::EPSILON / 2.0);
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
        animate::tick(advance);
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
