use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{StatefulWidget, Widget};
use rattles::{rattle, Rattle};

use crate::theme::Theme;

rattle!(
    EddaCraftSpinner,
    eddacraft,
    3,
    90,
    ["[ ]", "[=]", "[≡]", "[=]", "[ ]"]
);
rattle!(AnvilSpinner, anvil, 1, 110, ["⚒", "⚒", "⚒", "🔨", "⚒", "🛠"]);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpinnerPreset {
    #[default]
    EddaCraft,
    Anvil,
}

impl SpinnerPreset {
    #[must_use]
    pub fn frame(self, index: usize) -> &'static str {
        match self {
            Self::EddaCraft => eddacraft().frame(index),
            Self::Anvil => anvil().frame(index),
        }
    }

    #[must_use]
    pub fn interval(self) -> Duration {
        match self {
            Self::EddaCraft => EddaCraftSpinner::INTERVAL,
            Self::Anvil => AnvilSpinner::INTERVAL,
        }
    }

    #[must_use]
    pub fn len(self) -> usize {
        match self {
            Self::EddaCraft => eddacraft().len(),
            Self::Anvil => anvil().len(),
        }
    }
}

pub struct Spinner<'a, T: Theme> {
    theme: &'a T,
    label: Option<&'a str>,
    preset: SpinnerPreset,
}

#[derive(Debug, Default)]
pub struct SpinnerState {
    pub frame: usize,
}

impl SpinnerState {
    #[must_use]
    pub fn with_preset(preset: SpinnerPreset) -> Self {
        let _ = preset;
        Self { frame: 0 }
    }

    pub fn tick(&mut self) {
        self.tick_with(SpinnerPreset::default());
    }

    pub fn tick_with(&mut self, preset: SpinnerPreset) {
        let len = preset.len();
        if len == 0 {
            self.frame = 0;
            return;
        }
        self.frame = (self.frame + 1) % len;
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
        self.label = label.into();
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
        let mut state = SpinnerState {
            frame: SpinnerPreset::EddaCraft.len() - 1,
        };
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

        assert_eq!(buf[(0, 0)].symbol(), "🔨");
    }

    #[test]
    fn eddacraft_preset_uses_bracket_syntax_frames() {
        assert_eq!(SpinnerPreset::EddaCraft.frame(0), "[ ]");
        assert_eq!(SpinnerPreset::EddaCraft.frame(1), "[=]");
        assert_eq!(SpinnerPreset::EddaCraft.frame(2), "[≡]");
    }
}
