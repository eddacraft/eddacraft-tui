//! Drift-snapshots dashboard surface (TDASH-003).
//!
//! Renders the drift snapshot history (`.anvil/snapshots/`) natively: a summary
//! of how many snapshots exist, how many of the latest snapshot's boundary
//! violations are new versus the architecture baseline, the delta between the
//! two most recent snapshots, and a table of the snapshot history. Read-only;
//! the CLI loads the snapshots and baseline and maps them into the render-only
//! view structs here.

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::surface::Surface;

/// One snapshot, flattened for the history table. `pub` because the CLI
/// constructs it across the crate boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftSnapshotRow {
    /// RFC3339 capture timestamp.
    pub created_at: String,
    /// Optional user-given snapshot name.
    pub name: Option<String>,
    pub boundary_violations: usize,
    pub antipattern_count: usize,
    pub suppression_count: usize,
    pub files_analysed: usize,
}

/// Delta between the two most recent snapshots. Added/removed are ID-set diffs;
/// the `net_*` fields are signed metric changes (after − before).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftDelta {
    pub before_created_at: String,
    pub after_created_at: String,
    pub violations_added: usize,
    pub violations_removed: usize,
    pub antipatterns_added: usize,
    pub antipatterns_removed: usize,
    pub net_violations: i64,
    pub net_antipatterns: i64,
}

impl DriftDelta {
    /// Direction of travel: fewer violations + antipatterns is improving, more
    /// is degrading, no net change is stable.
    #[must_use]
    pub fn trend(&self) -> &'static str {
        match (self.net_violations + self.net_antipatterns).cmp(&0) {
            std::cmp::Ordering::Less => "improving",
            std::cmp::Ordering::Greater => "degrading",
            std::cmp::Ordering::Equal => "stable",
        }
    }
}

/// Render-only view of the drift state. Present when at least one snapshot
/// exists; `new_vs_baseline` is `None` when no architecture baseline is on disk
/// (so the surface distinguishes "no baseline" from "zero new violations").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftView {
    /// Snapshots newest-first.
    pub snapshots: Vec<DriftSnapshotRow>,
    /// Boundary violations in the latest snapshot that are not in the
    /// architecture baseline. `None` when no baseline exists.
    pub new_vs_baseline: Option<usize>,
    /// Delta between the two most recent snapshots. `None` with fewer than two.
    pub latest_delta: Option<DriftDelta>,
}

/// Drift-snapshots surface state. `view` is `None` when no snapshots exist.
pub struct DriftDashboardState {
    pub view: Option<DriftView>,
    pub should_quit: bool,
}

impl DriftDashboardState {
    #[must_use]
    pub fn new(view: Option<DriftView>) -> Self {
        Self {
            view,
            should_quit: false,
        }
    }
}

impl Surface for DriftDashboardState {
    fn surface_name(&self) -> &'static str {
        "Drift Snapshots"
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

use eddacraft_tui::prelude::{Container, ContainerVariant, DataTable, Theme};
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

/// Render the drift-snapshots surface body (shell chrome is drawn by the
/// surface loop).
pub fn render(frame: &mut Frame, area: Rect, state: &DriftDashboardState, theme: &EddaCraftTheme) {
    let Some(view) = &state.view else {
        render_empty(frame, area, theme);
        return;
    };

    let chunks =
        ratatui::layout::Layout::vertical([Constraint::Length(7), Constraint::Min(4)]).split(area);

    render_summary(frame, chunks[0], view, theme);
    render_history(frame, chunks[1], view, theme);
}

fn render_empty(frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
    let container = Container::new(theme)
        .title("Drift Snapshots")
        .variant(ContainerVariant::Primary);
    let inner = container.inner(area);
    frame.render_widget(container, area);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::raw(""),
            Line::styled(
                "  No drift snapshots found.",
                Style::default().fg(theme.muted()),
            ),
            Line::styled(
                "  Expected under .anvil/snapshots/.",
                Style::default().fg(theme.muted()),
            ),
            Line::styled(
                "  Run `anvil drift snapshot` to capture one.",
                Style::default().fg(theme.muted()),
            ),
        ])),
        inner,
    );
}

fn render_summary(frame: &mut Frame, area: Rect, view: &DriftView, theme: &EddaCraftTheme) {
    let container = Container::new(theme)
        .title("Drift Snapshots")
        .variant(ContainerVariant::Primary);
    let inner = container.inner(area);
    frame.render_widget(container, area);

    let new_edges = match view.new_vs_baseline {
        Some(count) => count.to_string(),
        None => "—".to_string(),
    };

    let mut lines = vec![Line::from(vec![
        metric_span("Snapshots", &view.snapshots.len().to_string(), theme),
        Span::raw("   "),
        metric_span("New vs baseline", &new_edges, theme),
    ])];

    if let Some(delta) = &view.latest_delta {
        lines.push(Line::from(vec![
            metric_span("Latest delta", delta.trend(), theme),
            Span::styled(
                format!(
                    "   violations +{}/-{} · antipatterns +{}/-{}",
                    delta.violations_added,
                    delta.violations_removed,
                    delta.antipatterns_added,
                    delta.antipatterns_removed
                ),
                Style::default().fg(theme.muted()),
            ),
        ]));
        lines.push(Line::styled(
            format!("  {} → {}", delta.before_created_at, delta.after_created_at),
            Style::default().fg(theme.muted()),
        ));
    } else {
        lines.push(Line::styled(
            "  Capture a second snapshot to see drift over time.",
            Style::default().fg(theme.muted()),
        ));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn metric_span(label: &str, value: &str, theme: &EddaCraftTheme) -> Span<'static> {
    Span::styled(
        format!("{label} {value}"),
        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
    )
}

fn render_history(frame: &mut Frame, area: Rect, view: &DriftView, theme: &EddaCraftTheme) {
    if view.snapshots.is_empty() {
        let container = Container::new(theme)
            .title("Snapshot History")
            .variant(ContainerVariant::Secondary);
        let inner = container.inner(area);
        frame.render_widget(container, area);
        frame.render_widget(
            Paragraph::new(Line::styled(
                "  No snapshots captured yet.",
                Style::default().fg(theme.muted()),
            )),
            inner,
        );
        return;
    }

    let headers = [
        "Captured",
        "Name",
        "Violations",
        "Antipatterns",
        "Suppr.",
        "Files",
    ];
    let widths = [
        Constraint::Min(20),
        Constraint::Length(16),
        Constraint::Length(11),
        Constraint::Length(13),
        Constraint::Length(7),
        Constraint::Length(6),
    ];
    let rows: Vec<Vec<String>> = view
        .snapshots
        .iter()
        .map(|snapshot| {
            vec![
                snapshot.created_at.clone(),
                snapshot.name.clone().unwrap_or_else(|| "—".to_string()),
                snapshot.boundary_violations.to_string(),
                snapshot.antipattern_count.to_string(),
                snapshot.suppression_count.to_string(),
                snapshot.files_analysed.to_string(),
            ]
        })
        .collect();

    frame.render_widget(
        DataTable::new(theme, &headers, &rows)
            .widths(&widths)
            .block(
                Container::new(theme)
                    .title("Snapshot History")
                    .variant(ContainerVariant::Secondary)
                    .to_block(),
            ),
        area,
    );
}

#[cfg(test)]
pub(crate) fn sample_view() -> DriftView {
    DriftView {
        snapshots: vec![
            DriftSnapshotRow {
                created_at: "2026-05-20T10:00:00Z".to_string(),
                name: Some("release-1.0".to_string()),
                boundary_violations: 5,
                antipattern_count: 12,
                suppression_count: 3,
                files_analysed: 240,
            },
            DriftSnapshotRow {
                created_at: "2026-05-13T09:00:00Z".to_string(),
                name: None,
                boundary_violations: 4,
                antipattern_count: 15,
                suppression_count: 2,
                files_analysed: 235,
            },
        ],
        new_vs_baseline: Some(2),
        latest_delta: Some(DriftDelta {
            before_created_at: "2026-05-13T09:00:00Z".to_string(),
            after_created_at: "2026-05-20T10:00:00Z".to_string(),
            violations_added: 2,
            violations_removed: 1,
            antipatterns_added: 1,
            antipatterns_removed: 4,
            net_violations: 1,
            net_antipatterns: -3,
        }),
    }
}

#[cfg(test)]
mod tests {
    use eddacraft_tui::theme::EddaCraftTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn render_to_string(state: &DriftDashboardState, width: u16, height: u16) -> String {
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
    fn renders_summary_metrics() {
        let state = DriftDashboardState::new(Some(sample_view()));
        let rendered = render_to_string(&state, 100, 20);
        assert!(rendered.contains("Snapshots 2"), "got:\n{rendered}");
        assert!(rendered.contains("New vs baseline 2"), "got:\n{rendered}");
    }

    #[test]
    fn renders_latest_delta() {
        let state = DriftDashboardState::new(Some(sample_view()));
        let rendered = render_to_string(&state, 100, 20);
        assert!(rendered.contains("Latest delta"), "got:\n{rendered}");
        assert!(rendered.contains("violations +2/-1"), "got:\n{rendered}");
    }

    #[test]
    fn renders_snapshot_rows() {
        let state = DriftDashboardState::new(Some(sample_view()));
        let rendered = render_to_string(&state, 100, 20);
        assert!(rendered.contains("release-1.0"), "got:\n{rendered}");
        assert!(rendered.contains("2026-05-20"), "got:\n{rendered}");
    }

    #[test]
    fn absent_baseline_shows_dash_for_new_vs_baseline() {
        let mut view = sample_view();
        view.new_vs_baseline = None;
        let state = DriftDashboardState::new(Some(view));
        let rendered = render_to_string(&state, 100, 20);
        assert!(rendered.contains("New vs baseline —"), "got:\n{rendered}");
    }

    #[test]
    fn single_snapshot_hides_delta() {
        let mut view = sample_view();
        view.snapshots.truncate(1);
        view.latest_delta = None;
        let state = DriftDashboardState::new(Some(view));
        let rendered = render_to_string(&state, 100, 20);
        assert!(
            rendered.contains("Capture a second snapshot"),
            "got:\n{rendered}"
        );
    }

    #[test]
    fn missing_snapshots_shows_empty_state() {
        let state = DriftDashboardState::new(None);
        let rendered = render_to_string(&state, 100, 20);
        assert!(rendered.contains("No drift snapshots"), "got:\n{rendered}");
        assert!(rendered.contains(".anvil/snapshots/"), "got:\n{rendered}");
    }

    #[test]
    fn trend_classifies_net_change() {
        let mut delta = sample_view().latest_delta.unwrap();
        delta.net_violations = -2;
        delta.net_antipatterns = -1;
        assert_eq!(delta.trend(), "improving");
        delta.net_violations = 3;
        delta.net_antipatterns = 0;
        assert_eq!(delta.trend(), "degrading");
        delta.net_violations = 0;
        delta.net_antipatterns = 0;
        assert_eq!(delta.trend(), "stable");
    }

    #[test]
    fn quit_action_sets_should_quit() {
        let mut state = DriftDashboardState::new(None);
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn renders_without_panic_when_narrow() {
        let state = DriftDashboardState::new(Some(sample_view()));
        let _ = render_to_string(&state, 40, 10);
    }
}
