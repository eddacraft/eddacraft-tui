use eddacraft_tui::prelude::{
    Container, ContainerVariant, DataTable, EddaCraftTheme, StatusBadge, Theme,
};
use eddacraft_tui::widgets::status_badge::BadgeStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use super::{PlanDashboardState, PlanModuleRow};

pub fn render(frame: &mut Frame, area: Rect, state: &PlanDashboardState, theme: &EddaCraftTheme) {
    if area.width < 60 {
        render_narrow(frame, area, state, theme);
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(8),
        Constraint::Length(6),
    ])
    .split(area);

    render_summary(frame, chunks[0], state, theme, "anvil APS Work Dashboard");

    let body = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);
    render_modules(frame, body[0], state, theme);
    render_detail(frame, body[1], state, theme);
    render_warnings(frame, chunks[2], state, theme);
}

fn render_narrow(
    frame: &mut Frame,
    area: Rect,
    state: &PlanDashboardState,
    theme: &EddaCraftTheme,
) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    render_summary(frame, chunks[0], state, theme, "APS Work");
    if state.show_detail {
        render_selected_module_detail(frame, chunks[1], state, theme);
    } else {
        render_modules_compact(frame, chunks[1], state, theme);
    }
}

fn render_summary(
    frame: &mut Frame,
    area: Rect,
    state: &PlanDashboardState,
    theme: &EddaCraftTheme,
    title: &str,
) {
    let in_progress = count_modules_by_status(state, "In Progress");
    let ready = count_items_by_status(state, "Ready");
    let blocked = count_items_by_status(state, "Blocked");
    let branch = state.snapshot.branch.as_deref().unwrap_or("unknown branch");

    let mut lines = vec![Line::from(vec![
        Span::styled(
            title,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  In Progress ", Style::default().fg(theme.muted())),
        Span::styled(in_progress.to_string(), Style::default().fg(theme.fg())),
        Span::styled("  Ready ", Style::default().fg(theme.muted())),
        Span::styled(ready.to_string(), Style::default().fg(theme.fg())),
        Span::styled("  Blocked ", Style::default().fg(theme.muted())),
        Span::styled(blocked.to_string(), Style::default().fg(theme.fg())),
        Span::styled("  Warnings ", Style::default().fg(theme.muted())),
        Span::styled(
            state.snapshot.warnings.len().to_string(),
            Style::default().fg(if state.snapshot.warnings.is_empty() {
                theme.success()
            } else {
                theme.error()
            }),
        ),
        Span::styled(format!("  {branch}"), Style::default().fg(theme.muted())),
    ])];

    if state.filter_mode || !state.filter_query.is_empty() || state.show_help {
        lines.push(Line::from(vec![
            Span::styled(
                if state.filter_mode {
                    "Filter: "
                } else {
                    "Filtered: "
                },
                Style::default().fg(theme.muted()),
            ),
            Span::styled(&state.filter_query, Style::default().fg(theme.accent())),
            Span::styled(
                "  keys: j/k modules  / filter  enter details/list  r rescan  q quit",
                Style::default().fg(theme.muted()),
            ),
        ]));
    }

    let container = Container::new(theme)
        .title("Summary")
        .variant(ContainerVariant::Primary);
    let inner = container.inner(area);
    frame.render_widget(container, area);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn count_modules_by_status(state: &PlanDashboardState, status: &str) -> usize {
    state
        .snapshot
        .modules
        .iter()
        .filter(|module| module.status.eq_ignore_ascii_case(status))
        .count()
}

fn count_items_by_status(state: &PlanDashboardState, status: &str) -> usize {
    state
        .snapshot
        .work_items
        .iter()
        .filter(|item| item.status.eq_ignore_ascii_case(status))
        .count()
}

fn render_modules(
    frame: &mut Frame,
    area: Rect,
    state: &PlanDashboardState,
    theme: &EddaCraftTheme,
) {
    let headers = ["", "Scope", "Done", "Status", "Note"];
    let widths = [
        Constraint::Length(3),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(14),
        Constraint::Min(10),
    ];
    let rows: Vec<Vec<String>> = visible_module_window(state, module_row_capacity(area))
        .into_iter()
        .map(|(row_index, (_, module))| {
            vec![
                if module.has_warning { "!" } else { "" }.to_string(),
                if row_index == state.selected_module {
                    format!("> {}", module.scope)
                } else {
                    module.scope.clone()
                },
                module.progress.clone(),
                module.status.clone(),
                module.note.clone(),
            ]
        })
        .collect();

    frame.render_widget(
        DataTable::new(theme, &headers, &rows)
            .widths(&widths)
            .block(
                Container::new(theme)
                    .title("Modules")
                    .variant(ContainerVariant::Secondary)
                    .to_block(),
            ),
        area,
    );
}

fn render_modules_compact(
    frame: &mut Frame,
    area: Rect,
    state: &PlanDashboardState,
    theme: &EddaCraftTheme,
) {
    let lines: Vec<Line> = visible_module_window(state, compact_module_row_capacity(area))
        .into_iter()
        .map(|(row_index, (_, module))| {
            let selected = if row_index == state.selected_module {
                ">"
            } else {
                " "
            };
            let warning = if module.has_warning { "!" } else { " " };
            Line::from(vec![
                Span::styled(selected, Style::default().fg(theme.accent())),
                Span::styled(warning, Style::default().fg(theme.error())),
                Span::styled(
                    format!(" {} ", module.scope),
                    Style::default().fg(theme.fg()),
                ),
                Span::styled(
                    format!("{} ", module.progress),
                    Style::default().fg(theme.muted()),
                ),
                Span::styled(module.status.clone(), Style::default().fg(theme.accent())),
            ])
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .block(
                Container::new(theme)
                    .title("Modules")
                    .variant(ContainerVariant::Secondary)
                    .to_block(),
            ),
        area,
    );
}

fn visible_module_window(
    state: &PlanDashboardState,
    capacity: usize,
) -> Vec<(usize, (usize, &PlanModuleRow))> {
    let visible = state.visible_modules();
    if capacity == 0 {
        return Vec::new();
    }
    if visible.len() <= capacity {
        return visible.into_iter().enumerate().collect();
    }

    let selected = state.selected_module.min(visible.len().saturating_sub(1));
    let max_start = visible.len().saturating_sub(capacity);
    let start = selected.saturating_sub(capacity / 2).min(max_start);
    visible
        .into_iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .collect()
}

fn module_row_capacity(area: Rect) -> usize {
    area.height.saturating_sub(3).into()
}

fn compact_module_row_capacity(area: Rect) -> usize {
    area.height.saturating_sub(2).into()
}

fn render_work_items(
    frame: &mut Frame,
    area: Rect,
    state: &PlanDashboardState,
    theme: &EddaCraftTheme,
) {
    let lines: Vec<Line> = state
        .snapshot
        .work_items
        .iter()
        .map(|item| {
            Line::from(vec![
                Span::styled(format!("{} ", item.id), Style::default().fg(theme.fg())),
                Span::styled(
                    format!("{} ", item.status),
                    Style::default().fg(theme.accent()),
                ),
                Span::styled(item.title.clone(), Style::default().fg(theme.muted())),
            ])
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .block(
                Container::new(theme)
                    .title("Open Items")
                    .variant(ContainerVariant::Secondary)
                    .to_block(),
            ),
        area,
    );
}

fn render_detail(
    frame: &mut Frame,
    area: Rect,
    state: &PlanDashboardState,
    theme: &EddaCraftTheme,
) {
    if state.show_detail {
        render_selected_module_detail(frame, area, state, theme);
    } else {
        render_work_items(frame, area, state, theme);
    }
}

fn render_selected_module_detail(
    frame: &mut Frame,
    area: Rect,
    state: &PlanDashboardState,
    theme: &EddaCraftTheme,
) {
    let Some(scope) = state.selected_module_scope() else {
        frame.render_widget(
            Paragraph::new("No module selected")
                .wrap(Wrap { trim: false })
                .block(
                    Container::new(theme)
                        .title("Detail")
                        .variant(ContainerVariant::Secondary)
                        .to_block(),
                ),
            area,
        );
        return;
    };

    let module = state
        .visible_modules()
        .into_iter()
        .find_map(|(_, module)| (module.scope == scope).then_some(module));
    let lines = module_detail_lines(scope, module, state, theme);

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .block(
                Container::new(theme)
                    .title("Detail")
                    .variant(ContainerVariant::Secondary)
                    .to_block(),
            ),
        area,
    );
}

fn module_detail_lines<'a>(
    scope: &str,
    module: Option<&PlanModuleRow>,
    state: &'a PlanDashboardState,
    theme: &EddaCraftTheme,
) -> Vec<Line<'a>> {
    let mut lines = vec![Line::from(Span::styled(
        scope.to_string(),
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    ))];

    if let Some(module) = module {
        lines.push(Line::from(Span::styled(
            format!("{}  {}", module.progress, module.status),
            Style::default().fg(theme.muted()),
        )));
    }

    for item in state
        .snapshot
        .work_items
        .iter()
        .filter(|item| item.module == scope)
    {
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", item.id), Style::default().fg(theme.fg())),
            Span::styled(
                format!("{} ", item.status),
                Style::default().fg(theme.accent()),
            ),
            Span::styled(item.title.clone(), Style::default().fg(theme.muted())),
        ]));
        if let Some(validation) = &item.validation {
            lines.push(Line::from(Span::styled(
                format!("Validation: {validation}"),
                Style::default().fg(theme.muted()),
            )));
        }
    }

    for warning in state
        .snapshot
        .warnings
        .iter()
        .filter(|warning| warning.module.as_deref() == Some(scope))
    {
        lines.push(Line::from(vec![
            Span::styled("! ", Style::default().fg(theme.error())),
            Span::styled(warning.message.clone(), Style::default().fg(theme.muted())),
        ]));
    }

    lines
}

fn render_warnings(
    frame: &mut Frame,
    area: Rect,
    state: &PlanDashboardState,
    theme: &EddaCraftTheme,
) {
    let lines = if state.snapshot.warnings.is_empty() {
        vec![Line::from(Span::styled(
            "No APS warnings",
            Style::default().fg(theme.success()),
        ))]
    } else {
        state
            .snapshot
            .warnings
            .iter()
            .map(|warning| {
                Line::from(vec![
                    Span::styled("! ", Style::default().fg(theme.error())),
                    Span::styled(
                        format!("{} ", warning.target),
                        Style::default().fg(theme.fg()),
                    ),
                    Span::styled(&warning.message, Style::default().fg(theme.muted())),
                ])
            })
            .collect()
    };

    let container = Container::new(theme)
        .title("Warnings")
        .variant(ContainerVariant::Subtle);
    let inner = container.inner(area);
    frame.render_widget(container, area);
    if !state.snapshot.warnings.is_empty() && inner.height > 0 && inner.width > 10 {
        StatusBadge::new(BadgeStatus::Warning, theme)
            .label("Warnings")
            .render(
                Rect::new(inner.x, inner.y, inner.width.min(12), 1),
                frame.buffer_mut(),
            );
        let warning_area = Rect::new(
            inner.x,
            inner.y.saturating_add(1),
            inner.width,
            inner.height.saturating_sub(1),
        );
        frame.render_widget(
            Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
            warning_area,
        );
    } else {
        frame.render_widget(
            Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
            inner,
        );
    }
}

#[cfg(test)]
mod tests {
    use eddacraft_tui::theme::EddaCraftTheme;
    use ratatui::{Terminal, backend::TestBackend};

    use super::super::sample_state;
    use super::*;

    fn render_to_string(width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = sample_state();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
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

    fn render_state_to_string(state: &PlanDashboardState, width: u16, height: u16) -> String {
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
    fn renders_summary_counts() {
        let rendered = render_to_string(100, 24);

        assert!(rendered.contains("anvil APS Work Dashboard"));
        assert!(rendered.contains("In Progress 2"));
        assert!(rendered.contains("Ready 1"));
        assert!(rendered.contains("Blocked 0"));
        assert!(rendered.contains("Warnings 1"));
    }

    #[test]
    fn renders_module_rows() {
        let rendered = render_to_string(100, 24);

        assert!(rendered.contains("DOCGOV"));
        assert!(rendered.contains("APSCAN"));
        assert!(rendered.contains("1/11"));
    }

    #[test]
    fn renders_warning_marker() {
        let rendered = render_to_string(100, 24);

        assert!(rendered.contains('!'));
        assert!(rendered.contains("needs reconcile"));
    }

    #[test]
    fn collapses_on_narrow_terminal() {
        let rendered = render_to_string(40, 12);

        assert!(rendered.contains("APS Work"));
        assert!(rendered.contains("DOCGOV"));
    }

    #[test]
    fn narrow_terminal_enter_shows_selected_module_detail() {
        let mut state = sample_state();
        state.selected_module = 1;
        state.show_detail = true;

        let rendered = render_state_to_string(&state, 40, 12);

        assert!(rendered.contains("Detail"));
        assert!(rendered.contains("APSCAN-011"));
    }

    #[test]
    fn scrolls_selected_module_into_view() {
        let mut state = sample_state();
        state.snapshot.modules = (0..20)
            .map(|index| PlanModuleRow {
                scope: format!("MOD{index:02}"),
                progress: "0/1".to_string(),
                status: "Ready".to_string(),
                note: "scroll fixture".to_string(),
                has_warning: false,
            })
            .collect();
        state.selected_module = 19;

        let rendered = render_state_to_string(&state, 100, 14);

        assert!(rendered.contains("MOD19"));
        assert!(!rendered.contains("MOD00"));
    }

    #[test]
    fn scrolls_module_text_before_selection_leaves_viewport() {
        let mut state = sample_state();
        state.snapshot.modules = (0..20)
            .map(|index| PlanModuleRow {
                scope: format!("MOD{index:02}"),
                progress: "0/1".to_string(),
                status: "Ready".to_string(),
                note: "scroll fixture".to_string(),
                has_warning: false,
            })
            .collect();
        state.selected_module = 8;

        let rendered = render_state_to_string(&state, 100, 14);

        assert!(rendered.contains("> MOD08"));
        assert!(!rendered.contains("MOD00"));
    }

    #[test]
    fn renders_selected_module_detail() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = sample_state();
        state.selected_module = 1;
        state.show_detail = true;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let buf = terminal.backend().buffer();
        let rendered = (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Detail"));
        assert!(rendered.contains("APSCAN-011"));
        assert!(rendered.contains("Validation:"));
    }

    #[test]
    fn wraps_selected_module_detail_body_text() {
        let mut state = sample_state();
        state.selected_module = 1;
        state.show_detail = true;
        state.snapshot.work_items[0].validation = Some(
            "cargo test -p eddacraft-anvil-tui plan_dashboard --lib with visible wrap marker"
                .to_string(),
        );

        let rendered = render_state_to_string(&state, 80, 24);

        assert!(rendered.contains("wrap marker"));
    }

    #[test]
    fn wraps_open_item_body_titles() {
        let mut state = sample_state();
        state.snapshot.work_items[0].title =
            "Add APS TUI dashboard and keep long title wrap marker visible".to_string();

        let rendered = render_state_to_string(&state, 80, 24);

        assert!(rendered.contains("wrap marker"));
    }
}
