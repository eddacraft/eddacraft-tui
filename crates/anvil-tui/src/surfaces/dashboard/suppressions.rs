//! Suppressions-overview dashboard surface (TDASH-004).
//!
//! Renders the active suppressions inventory (`.anvil/suppressions.json`, with
//! expired/malformed entries already filtered by the CLI loader): a count
//! summary plus a table of pattern/scope/file/expiry/reason. Read-only; the CLI
//! loads the data and maps it into the render-only view structs here.

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::prelude::{Container, ContainerVariant, DataTable, Theme};
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::surface::Surface;

/// One active suppression, flattened for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuppressionRow {
    pub pattern_id: String,
    pub scope: String,
    pub file: String,
    pub reason: String,
    pub expires_at: Option<String>,
}

/// Render-only view of the active suppressions. An empty `rows` is the
/// empty/clean state. File-absent and present-but-empty are intentionally
/// collapsed (matching `anvil export`) because both mean "no active
/// suppressions"; a read-only dashboard has no write path that would need to
/// tell them apart.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SuppressionsView {
    pub rows: Vec<SuppressionRow>,
}

/// Suppressions-overview surface state.
pub struct SuppressionsDashboardState {
    pub view: SuppressionsView,
    pub should_quit: bool,
}

impl SuppressionsDashboardState {
    #[must_use]
    pub fn new(view: SuppressionsView) -> Self {
        Self {
            view,
            should_quit: false,
        }
    }
}

impl Surface for SuppressionsDashboardState {
    fn surface_name(&self) -> &'static str {
        "Suppressions"
    }

    fn help_text(&self) -> &'static str {
        "esc/q back/quit"
    }

    fn handle_key(&mut self, action: Action) {
        if matches!(action, Action::Back | Action::Quit) {
            self.should_quit = true;
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        render(frame, area, self, theme);
    }
}

/// Render the suppressions surface body (shell chrome is drawn by the loop).
pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &SuppressionsDashboardState,
    theme: &EddaCraftTheme,
) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);
    render_summary(frame, chunks[0], &state.view, theme);
    render_table(frame, chunks[1], &state.view, theme);
}

fn render_summary(frame: &mut Frame, area: Rect, view: &SuppressionsView, theme: &EddaCraftTheme) {
    let container = Container::new(theme)
        .title("Suppressions")
        .variant(ContainerVariant::Primary);
    let inner = container.inner(area);
    frame.render_widget(container, area);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("Active suppressions: {}", view.rows.len()),
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        ))),
        inner,
    );
}

fn render_table(frame: &mut Frame, area: Rect, view: &SuppressionsView, theme: &EddaCraftTheme) {
    if view.rows.is_empty() {
        let container = Container::new(theme)
            .title("Active Suppressions")
            .variant(ContainerVariant::Secondary);
        let inner = container.inner(area);
        frame.render_widget(container, area);
        frame.render_widget(
            Paragraph::new(Line::styled(
                "  No active suppressions.",
                Style::default().fg(theme.success()),
            )),
            inner,
        );
        return;
    }

    let headers = ["Pattern", "Scope", "File", "Expires", "Reason"];
    let widths = [
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Min(16),
        Constraint::Length(12),
        Constraint::Min(16),
    ];
    let rows: Vec<Vec<String>> = view
        .rows
        .iter()
        .map(|row| {
            vec![
                row.pattern_id.clone(),
                row.scope.clone(),
                row.file.clone(),
                // Show the date only — the time component adds noise to an
                // overview and the full RFC 3339 stamp would overflow the
                // column. `--json` keeps the full timestamp.
                row.expires_at.as_deref().map_or_else(
                    || "—".to_string(),
                    |exp| exp.split('T').next().unwrap_or(exp).to_string(),
                ),
                row.reason.clone(),
            ]
        })
        .collect();

    frame.render_widget(
        DataTable::new(theme, &headers, &rows)
            .widths(&widths)
            .block(
                Container::new(theme)
                    .title("Active Suppressions")
                    .variant(ContainerVariant::Secondary)
                    .to_block(),
            ),
        area,
    );
}

#[cfg(test)]
pub(crate) fn sample_view() -> SuppressionsView {
    SuppressionsView {
        rows: vec![
            SuppressionRow {
                pattern_id: "AP-001".to_string(),
                scope: "file".to_string(),
                file: "src/legacy/old.ts".to_string(),
                reason: "legacy module, scheduled for removal".to_string(),
                expires_at: Some("2099-12-31T00:00:00Z".to_string()),
            },
            SuppressionRow {
                pattern_id: "AP-014".to_string(),
                scope: "repo".to_string(),
                file: "*".to_string(),
                reason: "vendored code".to_string(),
                expires_at: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use eddacraft_tui::theme::EddaCraftTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn render_to_string(state: &SuppressionsDashboardState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), state, &theme))
            .unwrap();
        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn renders_active_count_and_rows() {
        let state = SuppressionsDashboardState::new(sample_view());
        let rendered = render_to_string(&state, 100, 20);
        assert!(
            rendered.contains("Active suppressions: 2"),
            "got:\n{rendered}"
        );
        assert!(rendered.contains("AP-001"), "got:\n{rendered}");
        assert!(rendered.contains("src/legacy/old.ts"), "got:\n{rendered}");
        // Expiry renders as a date (not truncated mid-timestamp).
        assert!(rendered.contains("2099-12-31"), "got:\n{rendered}");
    }

    #[test]
    fn no_expiry_renders_as_dash() {
        let state = SuppressionsDashboardState::new(sample_view());
        let rendered = render_to_string(&state, 100, 20);
        // AP-014 has no expiry → shown as an em dash, not blank.
        assert!(rendered.contains("AP-014"), "got:\n{rendered}");
        assert!(
            rendered.contains('—'),
            "expected em dash for no-expiry:\n{rendered}"
        );
    }

    #[test]
    fn empty_view_shows_empty_state() {
        let state = SuppressionsDashboardState::new(SuppressionsView::default());
        let rendered = render_to_string(&state, 100, 20);
        assert!(
            rendered.contains("No active suppressions"),
            "got:\n{rendered}"
        );
    }

    #[test]
    fn quit_action_sets_should_quit() {
        let mut state = SuppressionsDashboardState::new(SuppressionsView::default());
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn renders_without_panic_when_narrow() {
        let state = SuppressionsDashboardState::new(sample_view());
        let _ = render_to_string(&state, 40, 10);
    }
}
