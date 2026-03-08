use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, StatefulWidget, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickWinType {
    TestFile,
    TypeDefinition,
    ConfigFile,
    GeneratedCode,
    Migration,
    ThirdParty,
    LegacyCode,
}

#[derive(Debug, Clone)]
pub struct BatchGroup {
    pub key: String,
    pub pattern_id: String,
    pub quick_win_type: QuickWinType,
    pub count: usize,
    pub suggested_reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct QuickWinsAnalysis {
    pub batch_groups: Vec<BatchGroup>,
    pub total_warnings: usize,
    pub suppressable: usize,
    pub suppressable_percent: f64,
}

#[derive(Debug, Default)]
pub struct QuickWinsPanelState;

pub struct QuickWinsPanel<'a, T: Theme> {
    theme: &'a T,
    analysis: &'a QuickWinsAnalysis,
    block: Option<Block<'a>>,
    focused: bool,
}

impl<'a, T: Theme> QuickWinsPanel<'a, T> {
    pub fn new(analysis: &'a QuickWinsAnalysis, theme: &'a T) -> Self {
        Self {
            theme,
            analysis,
            block: None,
            focused: false,
        }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block.into();
        self
    }

    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<T: Theme> StatefulWidget for QuickWinsPanel<'_, T> {
    type State = QuickWinsPanelState;

    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let border_type = if self.focused {
            BorderType::Double
        } else {
            BorderType::Plain
        };
        let border_style = if self.focused {
            self.theme.border_focused()
        } else {
            self.theme.border_unfocused()
        };

        let mut block = self
            .block
            .unwrap_or_else(|| Block::default().borders(Borders::ALL));
        block = block
            .border_type(border_type)
            .border_style(border_style)
            .title(Line::styled(
                format!("QUICK WINS ({})", self.analysis.suppressable),
                self.theme.title(),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let sections = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

        let progress = render_progress(
            self.analysis.suppressable,
            self.analysis.total_warnings,
            self.analysis.suppressable_percent,
            usize::from(sections[0].width),
        );
        Line::styled(progress, self.theme.base()).render(sections[0], buf);

        if self.analysis.batch_groups.is_empty() {
            Line::styled("No quick wins recognised", self.theme.disabled())
                .render(sections[1], buf);
        } else {
            for (index, group) in self.analysis.batch_groups.iter().take(5).enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                let y = sections[1].y + index as u16;
                if y >= sections[1].y + sections[1].height {
                    break;
                }

                let line = Line::from(vec![
                    Span::styled(
                        type_icon(group.quick_win_type).to_string(),
                        type_style(group.quick_win_type, self.theme),
                    ),
                    Span::raw(" "),
                    Span::styled(format!("{:>3}", group.count), self.theme.title()),
                    Span::raw(" "),
                    Span::styled(group.key.as_str(), self.theme.base()),
                ]);
                line.render(Rect::new(sections[1].x, y, sections[1].width, 1), buf);
            }
        }

        Line::styled(
            "Tip: batch suppressions by pattern for fastest clean-up",
            self.theme.disabled(),
        )
        .render(sections[2], buf);
    }
}

fn type_icon(quick_win_type: QuickWinType) -> char {
    match quick_win_type {
        QuickWinType::TestFile => 'T',
        QuickWinType::TypeDefinition => 'D',
        QuickWinType::ConfigFile => 'C',
        QuickWinType::GeneratedCode => 'G',
        QuickWinType::Migration => 'M',
        QuickWinType::ThirdParty => 'P',
        QuickWinType::LegacyCode => 'L',
    }
}

fn type_style<T: Theme>(quick_win_type: QuickWinType, theme: &T) -> Style {
    match quick_win_type {
        QuickWinType::TestFile => Style::default().fg(theme.success()),
        QuickWinType::TypeDefinition => Style::default().fg(theme.accent()),
        QuickWinType::ConfigFile | QuickWinType::Migration => Style::default().fg(theme.warning()),
        QuickWinType::GeneratedCode | QuickWinType::ThirdParty => theme.disabled(),
        QuickWinType::LegacyCode => Style::default().fg(theme.error()),
    }
}

fn render_progress(
    suppressable: usize,
    total: usize,
    suppressable_percent: f64,
    width: usize,
) -> String {
    if total == 0 {
        return "0/0 suppressable".to_string();
    }

    let clamped_percent = suppressable_percent.clamp(0.0, 100.0);

    let bar_width = width.saturating_sub(20).max(8);
    let filled = bar_width * suppressable.min(total) / total;
    let bar = "#".repeat(filled) + &"-".repeat(bar_width.saturating_sub(filled));
    format!("[{bar}] {suppressable}/{total} ({clamped_percent:.0}%)")
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::StatefulWidget;

    use super::*;
    use crate::theme::EddaCraftTheme;

    #[test]
    fn renders_with_data() {
        let theme = EddaCraftTheme;
        let analysis = QuickWinsAnalysis {
            batch_groups: vec![BatchGroup {
                key: "tests/**".to_string(),
                pattern_id: "AP-001".to_string(),
                quick_win_type: QuickWinType::TestFile,
                count: 12,
                suggested_reason: "Test harness".to_string(),
            }],
            total_warnings: 20,
            suppressable: 12,
            suppressable_percent: 60.0,
        };

        let mut state = QuickWinsPanelState;
        let mut buf = Buffer::empty(Rect::new(0, 0, 48, 8));
        QuickWinsPanel::new(&analysis, &theme).render(Rect::new(0, 0, 48, 8), &mut buf, &mut state);

        assert_eq!(buf[(1, 0)].symbol(), "Q");
    }

    #[test]
    fn renders_empty_state() {
        let theme = EddaCraftTheme;
        let analysis = QuickWinsAnalysis::default();

        let mut state = QuickWinsPanelState;
        let mut buf = Buffer::empty(Rect::new(0, 0, 48, 8));
        QuickWinsPanel::new(&analysis, &theme).render(Rect::new(0, 0, 48, 8), &mut buf, &mut state);

        assert_eq!(buf[(1, 2)].symbol(), "N");
    }
}
