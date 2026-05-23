use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::StatefulWidget;
use unicode_width::UnicodeWidthStr;

use crate::pretext::{ExclusionZone, LayoutResult, PreparedText, layout};
use crate::theme::Theme;

/// A ratatui widget that renders [`crate::pretext`] layouts on top of a
/// long-lived [`PretextState`].
///
/// The widget itself is zero-sized — all data lives in the state — so it is
/// cheap to construct each frame. The two-phase prepare/layout split means
/// subsequent frames at the same width skip re-measurement entirely.
#[derive(Default, Clone, Copy)]
pub struct PretextWidget {
    /// Fallback style applied when a word has no explicit style set.
    pub base_style: Style,
}

impl PretextWidget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a widget whose fallback style follows the supplied theme's
    /// `base()` palette.
    pub fn themed<T: Theme>(theme: &T) -> Self {
        Self {
            base_style: theme.base(),
        }
    }

    #[must_use]
    pub fn base_style(mut self, style: Style) -> Self {
        self.base_style = style;
        self
    }
}

/// Persistent state for a [`PretextWidget`].
///
/// Holds the prepare cache and the per-width layout cache across frames.
/// `prepared` and `exclusions` are kept private so that every mutation goes
/// through a setter that invalidates `layout_cache`.
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
        self.layout_cache.as_ref().map(|(_, r)| r)
    }
}

impl StatefulWidget for PretextWidget {
    type State = PretextState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Reset cells in the render area before drawing. The renderer below
        // only writes occupied cells, so without this a shrinking layout
        // (text shortened, scroll advanced, exclusion grew) would leave
        // stale glyphs from a prior frame visible.
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].reset();
            }
        }
        if self.base_style != Style::default() {
            buf.set_style(area, self.base_style);
        }

        let needs_relayout = state
            .layout_cache
            .as_ref()
            .is_none_or(|(w, _)| *w != area.width);

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
                let word_start_x = area.x.saturating_add(word.x);
                let y = area.y.saturating_add(render_y);
                if word_start_x >= area.right() || y >= area.bottom() {
                    continue;
                }

                let mut seg_x = word_start_x;
                for (seg_text, seg_style) in word.segments() {
                    if seg_x >= area.right() {
                        break;
                    }
                    let max_w = (area.right() - seg_x) as usize;
                    let style = self.base_style.patch(seg_style);
                    buf.set_stringn(seg_x, y, seg_text, max_w, style);
                    let consumed = UnicodeWidthStr::width(seg_text).min(max_w);
                    seg_x = seg_x.saturating_add(u16::try_from(consumed).unwrap_or(u16::MAX));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::StatefulWidget;

    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn renders_basic_text() {
        let mut state = PretextState::new("hello world");
        let widget = PretextWidget::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 3));
        widget.render(Rect::new(0, 0, 20, 3), &mut buf, &mut state);
        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(6, 0)].symbol(), "w");
    }

    #[test]
    fn reuses_layout_cache_at_same_width() {
        let mut state = PretextState::new("hello world");
        let widget = PretextWidget::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 3));
        widget.render(Rect::new(0, 0, 20, 3), &mut buf, &mut state);

        let words_before = state.layout_result().map(|r| r.lines[0].words.len());
        widget.render(Rect::new(0, 0, 20, 3), &mut buf, &mut state);
        let words_after = state.layout_result().map(|r| r.lines[0].words.len());

        assert_eq!(words_before, words_after);
        assert_eq!(words_after, Some(2));
    }

    #[test]
    fn relayouts_on_width_change() {
        let mut state = PretextState::new("hello world foo bar");
        let widget = PretextWidget::new();
        let mut buf_wide = Buffer::empty(Rect::new(0, 0, 80, 3));
        widget.render(Rect::new(0, 0, 80, 3), &mut buf_wide, &mut state);
        let wide_lines = state.layout_result().map_or(0, |r| r.lines.len());

        let mut buf_narrow = Buffer::empty(Rect::new(0, 0, 6, 5));
        widget.render(Rect::new(0, 0, 6, 5), &mut buf_narrow, &mut state);
        let narrow_lines = state.layout_result().map_or(0, |r| r.lines.len());

        assert!(narrow_lines > wide_lines);
    }

    #[test]
    fn themed_widget_uses_theme_base_style() {
        let theme = EddaCraftTheme;
        let widget = PretextWidget::themed(&theme);
        assert_eq!(widget.base_style, theme.base());
    }

    #[test]
    fn zero_sized_area_is_no_op() {
        let mut state = PretextState::new("hello world");
        let widget = PretextWidget::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        widget.render(Rect::new(0, 0, 0, 1), &mut buf, &mut state);
        widget.render(Rect::new(0, 0, 10, 0), &mut buf, &mut state);
        // Layout was never computed because both render passes bailed out.
        assert!(state.layout_result().is_none());
    }

    #[test]
    fn shrinking_text_does_not_leave_stale_glyphs() {
        let mut state = PretextState::new("hello world");
        let widget = PretextWidget::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        widget.render(Rect::new(0, 0, 20, 1), &mut buf, &mut state);
        assert_eq!(buf[(6, 0)].symbol(), "w");

        // Shrink the prepared text, render again — the old "world" cells must
        // be cleared rather than retain glyphs from the previous frame.
        state.set_text("hi");
        widget.render(Rect::new(0, 0, 20, 1), &mut buf, &mut state);
        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(1, 0)].symbol(), "i");
        assert_eq!(buf[(6, 0)].symbol(), " ");
    }
}
