use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::theme::Theme;

const EDDACRAFT_FRAMES: &[&str] = &["[ ]", "[=]", "[≡]", "[=]", "[ ]"];
const ANVIL_FRAMES: &[&str] = &["-", "=", "I", "‡"];

const EDDACRAFT_INTERVAL: Duration = Duration::from_millis(90);
const ANVIL_INTERVAL: Duration = Duration::from_millis(110);

/// Read-only handle into a spinner frame set. Returned by [`eddacraft`] and
/// [`anvil`] for callers that only need direct frame access without a full
/// [`Spinner`] widget.
#[derive(Debug, Clone, Copy)]
pub struct FrameSet {
    frames: &'static [&'static str],
    interval: Duration,
}

impl FrameSet {
    #[must_use]
    pub fn frame(&self, index: usize) -> &'static str {
        if self.frames.is_empty() {
            ""
        } else {
            self.frames[index % self.frames.len()]
        }
    }

    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

#[must_use]
pub fn eddacraft() -> FrameSet {
    FrameSet {
        frames: EDDACRAFT_FRAMES,
        interval: EDDACRAFT_INTERVAL,
    }
}

#[must_use]
pub fn anvil() -> FrameSet {
    FrameSet {
        frames: ANVIL_FRAMES,
        interval: ANVIL_INTERVAL,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpinnerPreset {
    #[default]
    EddaCraft,
    Anvil,
}

impl SpinnerPreset {
    #[must_use]
    pub fn frame(self, index: usize) -> &'static str {
        self.frames().frame(index)
    }

    #[must_use]
    pub fn interval(self) -> Duration {
        self.frames().interval()
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn len(self) -> usize {
        self.frames().len()
    }

    #[must_use]
    pub fn next_frame(self, frame: usize) -> usize {
        let len = self.len();
        if len == 0 {
            return 0;
        }
        (frame + 1) % len
    }

    fn frames(self) -> FrameSet {
        match self {
            Self::EddaCraft => eddacraft(),
            Self::Anvil => anvil(),
        }
    }
}

pub struct Spinner<'a, T: Theme> {
    theme: &'a T,
    label: Option<&'a str>,
    preset: SpinnerPreset,
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct SpinnerState {
    pub frame: usize,
    preset: SpinnerPreset,
}

impl SpinnerState {
    /// Construct state pinned to `preset`. Subsequent calls to [`tick`] then
    /// advance `frame` against `preset`'s frame count, so non-default presets
    /// (e.g. [`SpinnerPreset::Anvil`]) wrap correctly.
    ///
    /// [`tick`]: SpinnerState::tick
    #[must_use]
    pub fn with_preset(preset: SpinnerPreset) -> Self {
        Self { frame: 0, preset }
    }

    /// Advance against the preset stored in this state (set via
    /// [`with_preset`], defaults to [`SpinnerPreset::default`]). Use
    /// [`tick_with`] to override the preset for a single tick.
    ///
    /// [`with_preset`]: SpinnerState::with_preset
    /// [`tick_with`]: SpinnerState::tick_with
    pub fn tick(&mut self) {
        self.frame = self.preset.next_frame(self.frame);
    }

    pub fn tick_with(&mut self, preset: SpinnerPreset) {
        self.frame = preset.next_frame(self.frame);
    }
}

impl<'a, T: Theme> Spinner<'a, T> {
    pub fn new(theme: &'a T) -> Self {
        Self {
            theme,
            label: None,
            preset: SpinnerPreset::default(),
        }
    }

    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    #[must_use]
    pub fn preset(mut self, preset: SpinnerPreset) -> Self {
        self.preset = preset;
        self
    }

    #[must_use]
    pub fn eddacraft(self) -> Self {
        self.preset(SpinnerPreset::EddaCraft)
    }

    #[must_use]
    pub fn anvil(self) -> Self {
        self.preset(SpinnerPreset::Anvil)
    }
}

impl<T: Theme> StatefulWidget for Spinner<'_, T> {
    type State = SpinnerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let frame = self.preset.frame(state.frame);

        let line = if let Some(label) = self.label {
            Line::from(vec![
                Span::styled(frame, self.theme.title()),
                Span::raw(" "),
                Span::styled(label, self.theme.disabled()),
            ])
        } else {
            Line::from(vec![Span::styled(frame, self.theme.title())])
        };

        let row_area = Rect::new(area.x, area.y, area.width, 1);
        line.render(row_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::widgets::StatefulWidget;

    use super::*;
    use crate::theme::EddaCraftTheme;

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
        let mut state = SpinnerState::with_preset(SpinnerPreset::EddaCraft);
        state.frame = SpinnerPreset::EddaCraft.len() - 1;
        state.tick();
        assert_eq!(state.frame, 0);
    }

    #[test]
    fn presets_expose_expected_intervals() {
        assert_eq!(
            SpinnerPreset::EddaCraft.interval(),
            Duration::from_millis(90)
        );
        assert_eq!(SpinnerPreset::Anvil.interval(), Duration::from_millis(110));
    }

    #[test]
    fn anvil_preset_renders_custom_frame() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        let mut state = SpinnerState::with_preset(SpinnerPreset::Anvil);
        state.tick_with(SpinnerPreset::Anvil);
        state.tick_with(SpinnerPreset::Anvil);
        state.tick_with(SpinnerPreset::Anvil);

        Spinner::new(&theme)
            .anvil()
            .render(area, &mut buf, &mut state);

        // After 3 ticks from frame 0, with the 4-frame anvil preset
        // ["-", "=", "I", "‡"], the rendered frame is the final glyph.
        assert_eq!(buf[(0, 0)].symbol(), "‡");
    }

    #[test]
    fn tick_with_uses_passed_preset() {
        let mut state = SpinnerState::with_preset(SpinnerPreset::Anvil);
        state.frame = SpinnerPreset::Anvil.len() - 1;

        state.tick_with(SpinnerPreset::Anvil);

        assert_eq!(state.frame, 0);
    }

    #[test]
    fn tick_uses_preset_stored_via_with_preset() {
        // Regression: `with_preset(Anvil)` previously discarded its argument,
        // so `tick()` always wrapped against EddaCraft's frame count. Now the
        // preset is stored on `SpinnerState` and `tick()` honours it.
        let mut state = SpinnerState::with_preset(SpinnerPreset::Anvil);
        state.frame = SpinnerPreset::Anvil.len() - 1;
        state.tick();
        assert_eq!(state.frame, 0);
    }

    #[test]
    fn eddacraft_preset_uses_bracket_syntax_frames() {
        assert_eq!(SpinnerPreset::EddaCraft.frame(0), "[ ]");
        assert_eq!(SpinnerPreset::EddaCraft.frame(1), "[=]");
        assert_eq!(SpinnerPreset::EddaCraft.frame(2), "[≡]");
    }
}
