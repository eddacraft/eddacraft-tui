use std::fmt;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

use crate::animation::advance;
use crate::theme::Theme;
use crate::widgets::{AnimatedF64, animated_f64, with_ellipsis};

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

        // Retarget from the integer-derived fraction by exact bit compare.
        // Absolute `f64::EPSILON` dropped one-unit steps near 2^53; a half-count
        // threshold fixed that but then suppressed real updates when only
        // `total` changed (e.g. 1/100 → 1/101). `fraction()` is deterministic
        // from `current`/`total`, so identical inputs keep the same bits and
        // do not restart the tween. `total == 0` always yields 0.
        let target = state.fraction();
        if target.to_bits() != state.target_fraction.to_bits() {
            state.display_fraction.to(target);
            state.target_fraction = target;
        }
        advance(&mut state.display_fraction);

        let fraction = (*state.display_fraction).clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let pct = (fraction * 100.0) as u64;
        let width = usize::from(inner.width);
        let row = Rect::new(inner.x, inner.y, inner.width, 1);
        let style = self.theme.base();

        if width < PCT_WIDTH {
            // Too narrow for the padded percentage: show what fits of it.
            Line::styled(format!("{pct}%"), style).render(row, buf);
            return;
        }

        // The percentage is reserved first, right-aligned at a fixed width so
        // the bar does not shift as the digits change. The label comes next,
        // cut with an ellipsis when it does not fit, and the bar fills what is
        // left, followed by one separating space.
        let rest = width - PCT_WIDTH;
        let label = self.label.map_or_else(String::new, |label| {
            let full = format!("{label}: ");
            if UnicodeWidthStr::width(full.as_str()) <= rest {
                full
            } else if rest >= 2 {
                with_ellipsis(&format!("{label}:"), rest - 1) + " "
            } else {
                String::new()
            }
        });
        let bar_width = rest
            .saturating_sub(UnicodeWidthStr::width(label.as_str()))
            .saturating_sub(1);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let filled = (bar_width as f64 * fraction) as usize;
        let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);

        let left = if bar_width > 0 {
            format!("{label}{bar} ")
        } else {
            label
        };
        let rest = u16::try_from(rest).unwrap_or(u16::MAX);
        Line::styled(left, style).render(Rect::new(row.x, row.y, rest, 1), buf);
        Line::styled(format!("{pct:>3}%"), style)
            .render(Rect::new(row.x + rest, row.y, inner.width - rest, 1), buf);
    }
}

/// Columns reserved for the percentage, wide enough for `100%`.
const PCT_WIDTH: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::theme::EddaCraftTheme;

    /// Renders a settled bar (animation finished) and returns its first row.
    fn settled(width: u16, label: Option<&str>, current: u64) -> String {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        let mut state = ProgressBarState {
            current,
            total: 100,
            ..Default::default()
        };
        let bar = |theme| match label {
            Some(label) => ProgressBar::new(theme).label(label),
            None => ProgressBar::new(theme),
        };
        bar(&theme).render(area, &mut buf, &mut state);
        state.display_fraction.finish();
        bar(&theme).render(area, &mut buf, &mut state);
        // Skip the continuation cell a wide glyph leaves behind it.
        let mut out = String::new();
        let mut x = 0;
        while x < width {
            let symbol = buf[(x, 0)].symbol();
            out.push_str(symbol);
            x += u16::try_from(UnicodeWidthStr::width(symbol).max(1)).unwrap_or(1);
        }
        out
    }

    #[test]
    fn percentage_is_reserved_beside_the_bar() {
        // #6634: the bar took the full width and pushed the percentage off.
        assert_eq!(settled(20, None, 50), "███████░░░░░░░░  50%");
        assert_eq!(settled(10, None, 100), "█████ 100%");
        assert_eq!(settled(10, None, 0), "░░░░░   0%");
    }

    #[test]
    fn label_and_percentage_are_reserved_beside_the_bar() {
        assert_eq!(settled(24, Some("Build"), 50), "Build: ██████░░░░░░  50%");
    }

    #[test]
    fn narrow_bar_keeps_the_percentage_then_a_cut_label() {
        // No room for a bar: the label is cut with an ellipsis and the
        // percentage stays whole.
        assert_eq!(settled(10, Some("Build"), 50), "Buil…  50%");
        assert_eq!(settled(5, Some("Build"), 50), "  50%");
        assert_eq!(settled(4, None, 50), " 50%");
        assert_eq!(settled(3, None, 50), "50%");
    }

    #[test]
    fn wide_label_is_measured_in_display_columns() {
        // `構築` is 4 columns, so `構築: ` takes 6 and the bar gets 3.
        assert_eq!(settled(14, Some("構築"), 100), "構築: ███ 100%");
    }

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

    #[test]
    fn single_unit_step_updates_target_when_total_exceeds_f64_epsilon() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;

        use crate::theme::EddaCraftTheme;

        // 2^53 counts: one step is 2^-53, below f64::EPSILON (2^-52, about
        // 2.22e-16). Totals near 9e15 hit the same gap. The bar must still
        // retarget on a single increment.
        const TOTAL: u64 = 1 << 53;

        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        let mut state = ProgressBarState {
            current: 0,
            total: TOTAL,
            ..Default::default()
        };

        ProgressBar::new(&theme).render(area, &mut buf, &mut state);
        assert_eq!(state.target_fraction.to_bits(), 0.0_f64.to_bits());

        state.current = 1;
        ProgressBar::new(&theme).render(area, &mut buf, &mut state);

        let step = state.fraction();
        assert!(
            step < f64::EPSILON,
            "fixture must be invisible to an absolute f64::EPSILON check, step={step}"
        );
        assert_eq!(
            state.target_fraction.to_bits(),
            step.to_bits(),
            "one-unit step at total={TOTAL} must update target_fraction"
        );

        state.current = 2;
        ProgressBar::new(&theme).render(area, &mut buf, &mut state);
        assert_eq!(state.target_fraction.to_bits(), state.fraction().to_bits());
    }

    #[test]
    fn unchanged_fraction_does_not_restart_animation() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;

        use crate::theme::EddaCraftTheme;

        const TOTAL: u64 = 1 << 53;

        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        let mut state = ProgressBarState {
            current: 1,
            total: TOTAL,
            ..Default::default()
        };

        ProgressBar::new(&theme).render(area, &mut buf, &mut state);
        state.display_fraction.finish();
        assert!(!state.display_fraction.is_running());

        let target_bits = state.target_fraction.to_bits();
        let display_bits = state.display_fraction().to_bits();

        ProgressBar::new(&theme).render(area, &mut buf, &mut state);

        assert_eq!(state.target_fraction.to_bits(), target_bits);
        assert_eq!(
            state.display_fraction().to_bits(),
            display_bits,
            "re-rendering an unchanged fraction must not restart the tween"
        );
        assert!(!state.display_fraction.is_running());
    }

    #[test]
    fn total_only_change_updates_target_even_when_delta_is_tiny() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;

        use crate::theme::EddaCraftTheme;

        // Copilot: after 1/100, bumping total to 101 yields ~9.9e-5 delta —
        // below a half-count threshold of 0.5/101 — but must still retarget.
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        let mut state = ProgressBarState {
            current: 1,
            total: 100,
            ..Default::default()
        };

        ProgressBar::new(&theme).render(area, &mut buf, &mut state);
        state.display_fraction.finish();
        let before = state.target_fraction.to_bits();

        state.total = 101;
        let expected = state.fraction();
        assert!(
            (expected - f64::from_bits(before)).abs() < 0.5 / 101.0,
            "fixture must be invisible to a half-count threshold"
        );

        ProgressBar::new(&theme).render(area, &mut buf, &mut state);
        assert_eq!(
            state.target_fraction.to_bits(),
            expected.to_bits(),
            "changing only total must update target_fraction"
        );
        assert!(state.display_fraction.is_running());
    }
}
