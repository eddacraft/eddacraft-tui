use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::StatefulWidget;
use unicode_width::UnicodeWidthStr;

use crate::pretext::{ExclusionZone, LayoutResult, PreparedText, layout};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, Default)]
pub struct PretextWidget {
    pub base_style: Style,
}

impl PretextWidget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn themed<T: Theme>(theme: &T) -> Self {
        Self::new().base_style(theme.base())
    }

    #[must_use]
    pub fn base_style(mut self, style: Style) -> Self {
        self.base_style = style;
        self
    }
}

#[derive(Debug, Clone)]
pub struct PretextState {
    prepared: PreparedText,
    layout_cache: Option<(u16, LayoutResult)>,
    exclusions: Vec<ExclusionZone>,
    pub scroll: u16,
}

impl PretextState {
    pub fn new(text: &str) -> Self {
        Self {
            prepared: PreparedText::new(text),
            layout_cache: None,
            exclusions: Vec::new(),
            scroll: 0,
        }
    }

    pub fn styled(text: &str, style: Style) -> Self {
        Self {
            prepared: PreparedText::styled(text, style),
            layout_cache: None,
            exclusions: Vec::new(),
            scroll: 0,
        }
    }

    pub fn prepared(&self) -> &PreparedText {
        &self.prepared
    }

    pub fn exclusions(&self) -> &[ExclusionZone] {
        &self.exclusions
    }

    pub fn set_text(&mut self, text: &str) {
        self.prepared = PreparedText::new(text);
        self.layout_cache = None;
    }

    pub fn set_styled_text(&mut self, text: &str, style: Style) {
        self.prepared = PreparedText::styled(text, style);
        self.layout_cache = None;
    }

    pub fn append(&mut self, text: &str) {
        self.prepared.append(text);
        self.layout_cache = None;
    }

    pub fn append_styled(&mut self, text: &str, style: Style) {
        self.prepared.append_styled(text, style);
        self.layout_cache = None;
    }

    pub fn set_exclusions(&mut self, exclusions: Vec<ExclusionZone>) {
        self.exclusions = exclusions;
        self.layout_cache = None;
    }

    pub fn invalidate_layout(&mut self) {
        self.layout_cache = None;
    }

    pub fn layout_result(&self) -> Option<&LayoutResult> {
        self.layout_cache.as_ref().map(|(_, result)| result)
    }
}

impl StatefulWidget for PretextWidget {
    type State = PretextState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let needs_relayout = state
            .layout_cache
            .as_ref()
            .is_none_or(|(width, _)| *width != area.width);

        if needs_relayout {
            let result = layout(&state.prepared, area.width, &state.exclusions);
            state.layout_cache = Some((area.width, result));
        }

        let Some((_, layout_result)) = state.layout_cache.as_ref() else {
            return;
        };

        for line in &layout_result.lines {
            if line.y < state.scroll {
                continue;
            }

            let render_y = line.y - state.scroll;
            if render_y >= area.height {
                break;
            }

            for word in &line.words {
                let mut segment_x = area.x.saturating_add(word.x);
                let y = area.y.saturating_add(render_y);
                if segment_x >= area.right() || y >= area.bottom() {
                    continue;
                }

                for (segment_text, segment_style) in word.segments() {
                    if segment_x >= area.right() {
                        break;
                    }

                    let max_width = usize::from(area.right() - segment_x);
                    let style = self.base_style.patch(segment_style);
                    buf.set_stringn(segment_x, y, segment_text, max_width, style);
                    let segment_width = UnicodeWidthStr::width(segment_text).min(max_width);
                    segment_x = segment_x.saturating_add(usize_to_u16(segment_width));
                }
            }
        }
    }
}

fn usize_to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    use super::*;
    use crate::theme::{EddaCraftTheme, Theme};

    #[test]
    fn renders_wrapped_text() {
        let theme = EddaCraftTheme;
        let mut state = PretextState::new("hello world foo");
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 3));

        PretextWidget::themed(&theme).render(Rect::new(0, 0, 8, 3), &mut buf, &mut state);

        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(0, 0)].fg, theme.fg());
        assert_eq!(buf[(0, 0)].bg, theme.bg());
        assert_eq!(buf[(0, 1)].symbol(), "w");
        assert_eq!(buf[(0, 1)].fg, theme.fg());
        assert_eq!(buf[(0, 1)].bg, theme.bg());
    }

    #[test]
    fn preserves_mid_word_style_runs() {
        let red = Style::default().fg(Color::Red);
        let blue = Style::default().fg(Color::Blue);
        let mut state = PretextState::styled("hel", red);
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));

        state.append_styled("lo", blue);
        PretextWidget::new().render(Rect::new(0, 0, 8, 1), &mut buf, &mut state);

        assert_eq!(buf[(0, 0)].fg, Color::Red);
        assert_eq!(buf[(2, 0)].fg, Color::Red);
        assert_eq!(buf[(3, 0)].fg, Color::Blue);
        assert_eq!(buf[(4, 0)].fg, Color::Blue);
    }

    #[test]
    fn reuses_layout_until_width_changes_or_state_invalidates() {
        let mut state = PretextState::new("hello world");
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 2));

        PretextWidget::new().render(Rect::new(0, 0, 20, 2), &mut buf, &mut state);
        let first_height = state.layout_result().unwrap().total_height;

        state.append(" foo bar baz");
        PretextWidget::new().render(Rect::new(0, 0, 20, 2), &mut buf, &mut state);

        assert!(state.layout_result().unwrap().total_height >= first_height);
    }
}
