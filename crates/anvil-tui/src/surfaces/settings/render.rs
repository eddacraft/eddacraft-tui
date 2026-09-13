use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{SettingsState, format_row_values};

const MIN_DETAIL_HEIGHT: u16 = 4;

pub fn render(frame: &mut Frame, area: Rect, state: &SettingsState, theme: &EddaCraftTheme) {
    let search_height = u16::from(state.searching || !state.search_query.is_empty());
    let detail_height = if state.expanded {
        detail_panel_height(state, area, search_height)
    } else {
        0
    };
    let chunks = Layout::vertical([
        Constraint::Length(search_height),
        Constraint::Min(4),
        Constraint::Length(detail_height),
    ])
    .split(area);

    if search_height > 0 {
        render_search(frame, chunks[0], state, theme);
    }
    render_list(frame, chunks[1], state, theme);
    if detail_height > 0 {
        render_detail(frame, chunks[2], state, theme);
    }
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
    }
    frame.render_widget(Paragraph::new(lines), inner);
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
