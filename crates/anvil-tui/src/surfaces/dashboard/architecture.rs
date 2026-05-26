//! Architecture-health dashboard surface (TDASH-002).
//!
//! Renders the architecture baseline (`.anvil/architecture.json`) natively: a
//! summary of module/layer/boundary counts plus the table of baselined
//! boundary violations. Read-only; the CLI loads the baseline and maps it into
//! the render-only view structs here.

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::surface::Surface;

/// One baselined boundary violation, flattened for the TUI table. `pub` because
/// the CLI constructs it across the crate boundary. The CLI's serializable
/// counterpart (`ViolationRecord` in `commands/dashboard/architecture.rs`)
/// additionally carries `to_file`, which the compact table omits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchViolationRow {
    pub from_layer: String,
    pub to_layer: String,
    pub from_file: String,
    pub import_line: u32,
    pub rule: Option<String>,
}

/// Render-only view of the architecture baseline. `None` summary fields are
/// avoided — the CLI computes the counts before constructing this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitectureView {
    pub created_at: String,
    pub updated_at: String,
    pub module_count: u32,
    pub layer_count: usize,
    pub boundary_count: usize,
    pub entry_point_count: usize,
    pub violations: Vec<ArchViolationRow>,
}

/// Architecture-health surface state. `view` is `None` when no baseline exists.
pub struct ArchitectureDashboardState {
    pub view: Option<ArchitectureView>,
    pub should_quit: bool,
}

impl ArchitectureDashboardState {
    #[must_use]
    pub fn new(view: Option<ArchitectureView>) -> Self {
        Self {
            view,
            should_quit: false,
        }
    }
}

impl Surface for ArchitectureDashboardState {
    fn surface_name(&self) -> &'static str {
        "Architecture Health"
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

/// Render the architecture-health surface body (shell chrome is drawn by the
/// surface loop).
pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &ArchitectureDashboardState,
    theme: &EddaCraftTheme,
) {
    let Some(view) = &state.view else {
        render_empty(frame, area, theme);
        return;
    };

    let chunks = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(6),
        ratatui::layout::Constraint::Min(4),
    ])
    .split(area);

    render_summary(frame, chunks[0], view, theme);
    render_violations(frame, chunks[1], view, theme);
}

fn render_empty(frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
    let container = Container::new(theme)
        .title("Architecture Health")
        .variant(ContainerVariant::Primary);
    let inner = container.inner(area);
    frame.render_widget(container, area);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::raw(""),
            Line::styled(
                "  No architecture baseline found.",
                Style::default().fg(theme.muted()),
            ),
            Line::styled(
                "  Expected at .anvil/architecture.json.",
                Style::default().fg(theme.muted()),
            ),
        ])),
        inner,
    );
}

fn render_summary(frame: &mut Frame, area: Rect, view: &ArchitectureView, theme: &EddaCraftTheme) {
    let container = Container::new(theme)
        .title("Architecture Health")
        .variant(ContainerVariant::Primary);
    let inner = container.inner(area);
    frame.render_widget(container, area);

    let lines = vec![
        Line::from(vec![
            metric_span("Modules", &view.module_count.to_string(), theme),
            Span::raw("   "),
            metric_span("Layers", &view.layer_count.to_string(), theme),
            Span::raw("   "),
            metric_span("Boundaries", &view.boundary_count.to_string(), theme),
            Span::raw("   "),
            metric_span("Entry points", &view.entry_point_count.to_string(), theme),
        ]),
        Line::from(metric_span(
            "Baselined violations",
            &view.violations.len().to_string(),
            theme,
        )),
        Line::styled(
            format!(
                "  baselined {} · updated {}",
                view.created_at, view.updated_at
            ),
            Style::default().fg(theme.muted()),
        ),
    ];
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn metric_span(label: &str, value: &str, theme: &EddaCraftTheme) -> Span<'static> {
    Span::styled(
        format!("{label} {value}"),
        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
    )
}

fn render_violations(
    frame: &mut Frame,
    area: Rect,
    view: &ArchitectureView,
    theme: &EddaCraftTheme,
) {
    if view.violations.is_empty() {
        let container = Container::new(theme)
            .title("Baselined Violations")
            .variant(ContainerVariant::Secondary);
        let inner = container.inner(area);
        frame.render_widget(container, area);
        frame.render_widget(
            Paragraph::new(Line::styled(
                "  No baselined violations — clean architecture.",
                Style::default().fg(theme.success()),
            )),
            inner,
        );
        return;
    }

    let headers = ["From", "To", "File", "Line", "Rule"];
    let widths = [
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Min(16),
        Constraint::Length(6),
        Constraint::Length(16),
    ];
    let rows: Vec<Vec<String>> = view
        .violations
        .iter()
        .map(|violation| {
            vec![
                violation.from_layer.clone(),
                violation.to_layer.clone(),
                violation.from_file.clone(),
                violation.import_line.to_string(),
                violation.rule.clone().unwrap_or_else(|| "—".to_string()),
            ]
        })
        .collect();

    frame.render_widget(
        DataTable::new(theme, &headers, &rows)
            .widths(&widths)
            .block(
                Container::new(theme)
                    .title("Baselined Violations")
                    .variant(ContainerVariant::Secondary)
                    .to_block(),
            ),
        area,
    );
}

#[cfg(test)]
pub(crate) fn sample_view() -> ArchitectureView {
    ArchitectureView {
        created_at: "2026-05-01".to_string(),
        updated_at: "2026-05-20".to_string(),
        module_count: 42,
        layer_count: 4,
        boundary_count: 6,
        entry_point_count: 3,
        violations: vec![
            ArchViolationRow {
                from_layer: "ui".to_string(),
                to_layer: "db".to_string(),
                from_file: "src/ui/page.tsx".to_string(),
                import_line: 12,
                rule: Some("no-ui-to-db".to_string()),
            },
            ArchViolationRow {
                from_layer: "api".to_string(),
                to_layer: "ui".to_string(),
                from_file: "src/api/handler.ts".to_string(),
                import_line: 88,
                rule: None,
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

    fn render_to_string(state: &ArchitectureDashboardState, width: u16, height: u16) -> String {
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
        let state = ArchitectureDashboardState::new(Some(sample_view()));
        let rendered = render_to_string(&state, 100, 20);
        assert!(rendered.contains("Modules 42"), "got:\n{rendered}");
        assert!(rendered.contains("Layers 4"), "got:\n{rendered}");
        assert!(rendered.contains("Boundaries 6"), "got:\n{rendered}");
    }

    #[test]
    fn renders_violation_rows() {
        let state = ArchitectureDashboardState::new(Some(sample_view()));
        let rendered = render_to_string(&state, 100, 20);
        assert!(rendered.contains("no-ui-to-db"), "got:\n{rendered}");
        assert!(rendered.contains("src/ui/page.tsx"), "got:\n{rendered}");
    }

    #[test]
    fn clean_architecture_shows_no_violations_message() {
        let mut view = sample_view();
        view.violations.clear();
        let state = ArchitectureDashboardState::new(Some(view));
        let rendered = render_to_string(&state, 100, 20);
        assert!(
            rendered.contains("No baselined violations"),
            "got:\n{rendered}"
        );
    }

    #[test]
    fn missing_baseline_shows_empty_state() {
        let state = ArchitectureDashboardState::new(None);
        let rendered = render_to_string(&state, 100, 20);
        assert!(
            rendered.contains("No architecture baseline"),
            "got:\n{rendered}"
        );
        assert!(
            rendered.contains(".anvil/architecture.json"),
            "got:\n{rendered}"
        );
    }

    #[test]
    fn quit_action_sets_should_quit() {
        let mut state = ArchitectureDashboardState::new(None);
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn renders_without_panic_when_narrow() {
        let state = ArchitectureDashboardState::new(Some(sample_view()));
        let _ = render_to_string(&state, 40, 10);
    }
}
