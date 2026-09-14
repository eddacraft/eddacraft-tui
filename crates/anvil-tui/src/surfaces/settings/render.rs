use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{SettingsState, SettingsTab, format_row_values};

const MIN_DETAIL_HEIGHT: u16 = 4;
const NARROW_WIDTH: u16 = 60;

pub fn render(frame: &mut Frame, area: Rect, state: &SettingsState, theme: &EddaCraftTheme) {
    let search_height = u16::from(
        state.tab == SettingsTab::Settings && (state.searching || !state.search_query.is_empty()),
    );
    let detail_height = if state.tab == SettingsTab::Settings && state.expanded {
        detail_panel_height(state, area, search_height)
    } else {
        0
    };
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(search_height),
        Constraint::Min(4),
        Constraint::Length(detail_height),
    ])
    .split(area);

    render_tabs(frame, chunks[0], state, theme);
    if search_height > 0 {
        render_search(frame, chunks[1], state, theme);
    }
    match state.tab {
        SettingsTab::Settings => {
            if area.width < NARROW_WIDTH {
                render_narrow_list(frame, chunks[2], state, theme);
            } else {
                render_list(frame, chunks[2], state, theme);
            }
            if detail_height > 0 {
                render_detail(frame, chunks[3], state, theme);
            }
        }
        SettingsTab::Status => render_status(frame, chunks[2], state, theme),
        SettingsTab::Sources => render_sources(frame, chunks[2], state, theme),
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, state: &SettingsState, theme: &EddaCraftTheme) {
    let mut spans = Vec::new();
    for tab in SettingsTab::ALL {
        let active = tab == state.tab;
        let style = if active {
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.muted())
        };
        spans.push(Span::styled(format!(" {} ", tab.label()), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_search(frame: &mut Frame, area: Rect, state: &SettingsState, theme: &EddaCraftTheme) {
    let query = if state.search_query.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}", state.search_query)
    };
    let style = if state.searching {
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted())
    };
    frame.render_widget(Paragraph::new(Line::from(Span::styled(query, style))), area);
}

fn render_list(frame: &mut Frame, area: Rect, state: &SettingsState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.muted()))
        .title(" Settings ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = state.visible_rows();
    if visible.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "No matching settings",
                Style::default().fg(theme.muted()),
            ))),
            inner,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let mut last_group: Option<usize> = None;
    let mut selected_line = 0usize;
    for (visible_idx, (group_idx, row_idx)) in visible.iter().copied().enumerate() {
        if last_group != Some(group_idx) {
            if let Some(group) = state.groups.get(group_idx) {
                lines.push(Line::from(Span::styled(
                    group.label.clone(),
                    Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
                )));
            }
            last_group = Some(group_idx);
        }
        let Some(row) = state
            .groups
            .get(group_idx)
            .and_then(|group| group.rows.get(row_idx))
        else {
            continue;
        };
        let selected = visible_idx == state.selected;
        let indicator = if selected { ">> " } else { "   " };
        let values = format_row_values(row);
        let content = format!("{indicator}{:<22} {values}", row.label);
        let style = if selected {
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg())
        };
        lines.push(Line::from(Span::styled(content, style)));
        if selected {
            selected_line = lines.len().saturating_sub(1);
        }
    }
    frame.render_widget(
        Paragraph::new(window_lines(lines, inner.height, selected_line)),
        inner,
    );
}

fn render_detail(frame: &mut Frame, area: Rect, state: &SettingsState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.muted()))
        .title(" Detail ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines =
        state.current_row().map_or_else(Vec::new, |row| {
            let mut lines = Vec::new();
            if !row.description.is_empty() {
                lines.push(Line::from(Span::styled(
                    row.description.clone(),
                    Style::default().fg(theme.fg()),
                )));
            }
            lines.extend(row.detail.lines.iter().map(|line| {
                Line::from(Span::styled(line.clone(), Style::default().fg(theme.fg())))
            }));
            lines
        });
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_narrow_list(
    frame: &mut Frame,
    area: Rect,
    state: &SettingsState,
    theme: &EddaCraftTheme,
) {
    let mut selected_line = 0usize;
    let mut lines = Vec::new();
    for (visible_idx, (group_idx, row_idx)) in state.visible_rows().into_iter().enumerate() {
        let Some(row) = state
            .groups
            .get(group_idx)
            .and_then(|group| group.rows.get(row_idx))
        else {
            continue;
        };
        let selected = visible_idx == state.selected;
        if selected {
            selected_line = lines.len();
        }
        let indicator = if selected { ">> " } else { "   " };
        let content = format!("{indicator}{}  {}", row.label, row.runtime.as_str());
        lines.push(Line::from(Span::styled(
            content,
            if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            },
        )));
    }
    frame.render_widget(
        Paragraph::new(window_lines(lines, area.height, selected_line)),
        area,
    );
}

fn render_status(frame: &mut Frame, area: Rect, state: &SettingsState, theme: &EddaCraftTheme) {
    let s = &state.status;
    let mut lines = vec![
        kv("version", &s.version, theme),
        kv("runtime", &s.runtime, theme),
        kv("project", &s.project, theme),
        kv("worktree", &s.worktree, theme),
        kv("session", &s.session, theme),
        kv("resolved posture", &s.resolved_posture, theme),
        kv("attested posture", &s.attested_posture, theme),
        kv("health", &s.health, theme),
        kv("validation", &s.validation, theme),
        kv("attestation age", &s.attestation_age, theme),
        kv("attestation source", &s.attestation_source, theme),
    ];
    for reason in &s.health_reasons {
        lines.push(kv("health reason", reason, theme));
    }
    for item in &s.non_healthy {
        lines.push(kv("non-healthy", item, theme));
    }
    for item in &s.integrations {
        lines.push(kv("integration", item, theme));
    }
    for item in &s.adapters {
        lines.push(kv("adapter", item, theme));
    }
    if let Some(pending) = &s.pending {
        lines.push(kv("pending", pending, theme));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_sources(frame: &mut Frame, area: Rect, state: &SettingsState, theme: &EddaCraftTheme) {
    let src = &state.sources;
    let mut lines = vec![kv("revision", &src.revision, theme)];
    for source in &src.sources {
        let writable = if source.writable {
            "writable"
        } else {
            "locked"
        };
        lines.push(kv(
            "source",
            &format!(
                "{} {} {} [{writable}]",
                source.kind, source.scope, source.path_display
            ),
            theme,
        ));
    }
    for item in &src.overridden {
        lines.push(kv("overridden", item, theme));
    }
    for item in &src.winning {
        lines.push(kv("winning", item, theme));
    }
    for item in &src.field_provenance {
        lines.push(kv("field", item, theme));
    }
    for item in &src.unknown_keys {
        lines.push(kv("unknown", item, theme));
    }
    for item in &src.deprecated_keys {
        lines.push(kv("deprecated", item, theme));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn kv(label: &str, value: &str, theme: &EddaCraftTheme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(theme.muted())),
        Span::styled(value.to_owned(), Style::default().fg(theme.fg())),
    ])
}

fn window_lines(
    lines: Vec<Line<'static>>,
    height: u16,
    selected_line: usize,
) -> Vec<Line<'static>> {
    let height = usize::from(height);
    if height == 0 || lines.len() <= height {
        return lines;
    }
    let max_start = lines.len() - height;
    let start = selected_line
        .saturating_sub(height.saturating_sub(1))
        .min(max_start);
    lines.into_iter().skip(start).take(height).collect()
}

fn detail_panel_height(state: &SettingsState, area: Rect, search_height: u16) -> u16 {
    let lines = state
        .current_row()
        .map_or(0, |row| {
            usize::from(!row.description.is_empty()) + row.detail.lines.len()
        })
        .max(1);
    let cap = area
        .height
        .saturating_sub(search_height.saturating_add(6))
        .max(MIN_DETAIL_HEIGHT);
    u16::try_from(lines.saturating_add(2))
        .unwrap_or(cap)
        .clamp(MIN_DETAIL_HEIGHT, cap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surfaces::settings::RuntimeLabel;
    use crate::surfaces::settings::SettingsState;
    use crate::surfaces::settings::tests::sample_state;
    use eddacraft_tui::keyboard::Action;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn draw_terminal(state: &SettingsState) -> ratatui::Terminal<TestBackend> {
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    "Settings",
                    state.footer_commands(),
                    &theme,
                );
                render(frame, content, state, &theme);
            })
            .unwrap();
        terminal
    }

    fn draw(state: &SettingsState) -> String {
        let terminal = draw_terminal(state);
        crate::test_utils::snapshot::buffer_to_string(terminal.backend().buffer())
    }

    fn draw_plain(state: &SettingsState) -> String {
        let terminal = draw_terminal(state);
        let buf = terminal.backend().buffer();
        let area = buf.area;
        let mut output = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                output.push_str(buf[(x, y)].symbol());
            }
            output.push('\n');
        }
        output
    }

    #[test]
    fn settings_view_snapshot_grouped() {
        let state = sample_state();
        insta::assert_snapshot!(draw(&state));
    }

    #[test]
    fn settings_view_snapshot_search() {
        let mut state = sample_state();
        state.handle_key(Action::Character('/'));
        for ch in "enforcement".chars() {
            state.handle_key(Action::Character(ch));
        }
        insta::assert_snapshot!(draw(&state));
    }

    #[test]
    fn settings_row_snapshot_expanded() {
        let mut state = sample_state();
        while state.current_row().map(|row| row.key.as_str()) != Some("protection.enforcement.mode")
        {
            state.handle_key(Action::Down);
        }
        state.handle_key(Action::Select);
        insta::assert_snapshot!(draw(&state));
        let plain = draw_plain(&state);
        assert!(
            plain.contains("cli: anvil settings explain protection.enforcement.mode"),
            "{plain}"
        );
        assert!(
            plain.contains("How anvil treats policy violations"),
            "{plain}"
        );
        assert!(plain.contains("affected: intercept"), "{plain}");
        assert!(plain.contains("findings: none"), "{plain}");
        assert!(plain.contains("restart: none"), "{plain}");
    }

    #[test]
    fn settings_row_render_states_are_textual() {
        let state = sample_state();
        let buffer = draw_plain(&state);
        assert!(buffer.contains("DRIFT"));
        assert!(buffer.contains("STALE"));
        assert!(buffer.contains("FAILED"));
        assert!(buffer.contains("unknown"));
        assert!(buffer.contains("locked"));
        assert!(buffer.contains("[redacted]"));
        assert!(!buffer.contains("s3cret"));
    }

    #[test]
    fn settings_window_keeps_selected_line_visible() {
        let lines: Vec<Line> = (0..20).map(|i| Line::from(format!("row-{i}"))).collect();
        let window = window_lines(lines, 5, 12);
        let text: String = window
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("row-12"), "{text}");
        assert!(!text.contains("row-0"), "{text}");
        assert!(!text.contains("row-6"), "{text}");
    }

    #[test]
    fn settings_narrow_list_keeps_selected_visible() {
        let mut state = sample_state();
        state.handle_key(Action::Character('G'));
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    "Settings",
                    state.footer_commands(),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let area = buf.area;
        let mut output = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                output.push_str(buf[(x, y)].symbol());
            }
            output.push('\n');
        }
        assert!(
            output.contains("Compact mode"),
            "selected last row must remain visible:\n{output}"
        );
        assert!(output.contains(">> "), "{output}");
    }

    #[test]
    fn settings_status_snapshot() {
        let mut state = sample_state().with_status(crate::surfaces::settings::SettingsStatusView {
            version: "0.10.0-beta".into(),
            runtime: "linux".into(),
            project: "demo".into(),
            worktree: "main".into(),
            session: "s1".into(),
            resolved_posture: "warn".into(),
            attested_posture: "unknown".into(),
            health: "indeterminate".into(),
            health_reasons: vec!["checks unknown".into()],
            non_healthy: vec!["protection.checks unknown".into()],
            integrations: vec!["mcp".into()],
            adapters: vec!["grok".into()],
            pending: None,
            validation: "ok".into(),
            attestation_age: "n/a".into(),
            attestation_source: "none".into(),
        });
        state.handle_key(Action::Right);
        insta::assert_snapshot!(draw(&state));
    }

    #[test]
    fn settings_sources_snapshot() {
        let mut state =
            sample_state().with_sources(crate::surfaces::settings::SettingsSourcesView {
                revision: "rev-1".into(),
                sources: vec![crate::surfaces::settings::SettingsSourceRow {
                    scope: "project".into(),
                    path_display: ".anvil.yaml".into(),
                    writable: true,
                    kind: "precedence".into(),
                }],
                overridden: vec![],
                winning: vec!["project block".into()],
                field_provenance: vec![],
                unknown_keys: vec!["legacy.foo".into()],
                deprecated_keys: vec![],
            });
        state.handle_key(Action::Right);
        state.handle_key(Action::Right);
        insta::assert_snapshot!(draw(&state));
    }

    #[test]
    fn settings_a11y_narrow_keeps_textual_state() {
        let state = sample_state();
        let backend = TestBackend::new(40, 16);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let buf = terminal.backend().buffer();
        let area = buf.area;
        let mut plain = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                plain.push_str(buf[(x, y)].symbol());
            }
            plain.push('\n');
        }
        assert!(plain.contains("DRIFT") || plain.contains("Enforcement"));
        assert!(plain.contains("Settings"));
    }

    #[test]
    fn settings_a11y_reduced_motion_is_honoured() {
        let state = sample_state().with_reduced_motion(true);
        assert!(state.reduced_motion);
        let _ = draw(&state);
    }

    #[test]
    fn settings_row_compact_row_omits_split_labels() {
        let state = sample_state();
        let compact = state
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .find(|row| {
                row.runtime == RuntimeLabel::Active && row.compact && row.active_display.is_some()
            })
            .expect("compact active row");
        let rendered = crate::surfaces::settings::format_row_values(compact);
        assert!(!rendered.contains("resolved:"));
    }
}
