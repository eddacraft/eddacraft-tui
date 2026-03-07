use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, StatefulWidget, Widget};

use crate::theme::Theme;
use crate::widgets::header::Header;
use crate::widgets::quick_wins_panel::{QuickWinsAnalysis, QuickWinsPanel, QuickWinsPanelState};

#[derive(Debug, Clone, Default)]
pub struct HistoricalAnalysis {
    pub total_commits: usize,
    pub total_violations: usize,
    pub avg_violations_per_commit: f64,
    pub pattern_occurrences: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Default)]
pub struct InitAnalysisResults {
    pub framework: String,
    pub project_root: String,
    pub size: String,
    pub file_count: usize,
    pub monorepo: bool,
    pub ts_strictness: String,
    pub analysis_summary: Vec<(String, usize)>,
    pub quick_wins: QuickWinsAnalysis,
    pub historical: HistoricalAnalysis,
    pub config_path: String,
    pub sample_files: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ResultsDashboardState;

pub struct ResultsDashboard<'a, T: Theme> {
    theme: &'a T,
    results: &'a InitAnalysisResults,
    block: Option<Block<'a>>,
    focused: bool,
}

impl<'a, T: Theme> ResultsDashboard<'a, T> {
    pub fn new(results: &'a InitAnalysisResults, theme: &'a T) -> Self {
        Self {
            theme,
            results,
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

impl<T: Theme> StatefulWidget for ResultsDashboard<'_, T> {
    type State = ResultsDashboardState;

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
            .title(Line::styled("RESULTS DASHBOARD", self.theme.title()));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let sections = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

        Header::new("Initial Analysis", self.theme)
            .subtitle(self.results.project_root.as_str())
            .render(sections[0], buf);

        Line::styled("Analysis completed successfully", self.theme.status_ok())
            .render(sections[1], buf);

        render_metrics_panel(self.results, self.theme, sections[2], buf);

        let mut quick_wins_state = QuickWinsPanelState;
        QuickWinsPanel::new(&self.results.quick_wins, self.theme)
            .focused(self.focused)
            .render(sections[3], buf, &mut quick_wins_state);

        render_historical_panel(self.results, self.theme, sections[4], buf);
        render_next_steps_panel(self.results, self.theme, sections[5], buf);

        Line::styled("[Enter] continue  [q] close", self.theme.disabled()).render(sections[6], buf);
    }
}

fn render_metrics_panel<T: Theme>(
    results: &InitAnalysisResults,
    theme: &T,
    area: Rect,
    buf: &mut Buffer,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_unfocused())
        .title(Line::styled("Metrics", theme.title()));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let monorepo = if results.monorepo { "yes" } else { "no" };
    let lines = [
        format!("Framework: {}", results.framework),
        format!("Size: {} ({} files)", results.size, results.file_count),
        format!("Monorepo: {monorepo}"),
        format!("TypeScript strictness: {}", results.ts_strictness),
        format!("Config: {}", results.config_path),
    ];

    for (index, content) in lines.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let y = inner.y + index as u16;
        if y >= inner.y + inner.height {
            break;
        }
        Line::styled(content.as_str(), theme.base())
            .render(Rect::new(inner.x, y, inner.width, 1), buf);
    }
}

fn render_historical_panel<T: Theme>(
    results: &InitAnalysisResults,
    theme: &T,
    area: Rect,
    buf: &mut Buffer,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_unfocused())
        .title(Line::styled("Historical", theme.title()));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let top_pattern = results.historical.pattern_occurrences.first().map_or_else(
        || "-".to_string(),
        |(name, count)| format!("{name} ({count})"),
    );

    let lines = [
        format!("Commits analysed: {}", results.historical.total_commits),
        format!(
            "Violations: {} (avg {:.1}/commit)",
            results.historical.total_violations, results.historical.avg_violations_per_commit
        ),
        format!("Top pattern: {top_pattern}"),
    ];

    for (index, content) in lines.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let y = inner.y + index as u16;
        if y >= inner.y + inner.height {
            break;
        }
        Line::styled(content, theme.base()).render(Rect::new(inner.x, y, inner.width, 1), buf);
    }
}

fn render_next_steps_panel<T: Theme>(
    results: &InitAnalysisResults,
    theme: &T,
    area: Rect,
    buf: &mut Buffer,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_unfocused())
        .title(Line::styled("Next Steps", theme.title()));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let sample_hint = results.sample_files.first().map_or_else(
        || "No samples available".to_string(),
        |sample| format!("Review sample: {sample}"),
    );

    let line = Line::from(vec![
        Span::styled("1) ", theme.title()),
        Span::styled("Review quick wins. ", theme.base()),
        Span::styled("2) ", theme.title()),
        Span::styled(sample_hint, theme.disabled()),
    ]);
    line.render(inner, buf);
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::StatefulWidget;

    use super::*;
    use crate::theme::EddaCraftTheme;
    use crate::widgets::quick_wins_panel::{BatchGroup, QuickWinType};

    #[test]
    fn renders_with_full_data() {
        let theme = EddaCraftTheme;
        let results = InitAnalysisResults {
            framework: "Nx".to_string(),
            project_root: "/repo".to_string(),
            size: "42 MB".to_string(),
            file_count: 512,
            monorepo: true,
            ts_strictness: "strict".to_string(),
            analysis_summary: vec![("AP-001".to_string(), 12)],
            quick_wins: QuickWinsAnalysis {
                batch_groups: vec![BatchGroup {
                    key: "tests/**".to_string(),
                    pattern_id: "AP-001".to_string(),
                    quick_win_type: QuickWinType::TestFile,
                    count: 10,
                    suggested_reason: "test harness".to_string(),
                }],
                total_warnings: 30,
                suppressable: 20,
                suppressable_percent: 66.0,
            },
            historical: HistoricalAnalysis {
                total_commits: 50,
                total_violations: 120,
                avg_violations_per_commit: 2.4,
                pattern_occurrences: vec![("AP-001".to_string(), 40)],
            },
            config_path: "anvil.toml".to_string(),
            sample_files: vec!["src/main.ts".to_string()],
        };

        let mut state = ResultsDashboardState;
        let mut buf = Buffer::empty(Rect::new(0, 0, 90, 34));
        ResultsDashboard::new(&results, &theme).render(
            Rect::new(0, 0, 90, 34),
            &mut buf,
            &mut state,
        );

        assert_eq!(buf[(1, 0)].symbol(), "R");
        assert_eq!(buf[(1, 1)].symbol(), "━");
    }

    #[test]
    fn renders_with_minimal_data() {
        let theme = EddaCraftTheme;
        let results = InitAnalysisResults::default();

        let mut state = ResultsDashboardState;
        let mut buf = Buffer::empty(Rect::new(0, 0, 70, 28));
        ResultsDashboard::new(&results, &theme).render(
            Rect::new(0, 0, 70, 28),
            &mut buf,
            &mut state,
        );

        assert_eq!(buf[(1, 2)].symbol(), "I");
    }
}
