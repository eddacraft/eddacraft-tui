use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use eddacraft_tui::widgets::editor::Editor;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::discovery::FindingSeverity;
use super::fix::{FixPhase, FixState};

pub fn render(frame: &mut Frame, area: Rect, state: &FixState, theme: &EddaCraftTheme) {
    match state.phase {
        FixPhase::Watching => render_watching(frame, area, state, theme),
        FixPhase::Editing => render_editing(frame, area, state, theme),
        FixPhase::Resolved => render_resolved(frame, area, state, theme),
        FixPhase::TimedOut => render_timed_out(frame, area, state, theme),
    }
}

fn severity_badge(severity: FindingSeverity, theme: &EddaCraftTheme) -> (String, Style) {
    match severity {
        FindingSeverity::Error => (
            " ERR ".to_string(),
            Style::default()
                .fg(theme.error())
                .add_modifier(Modifier::BOLD),
        ),
        FindingSeverity::Warning => (
            "WARN ".to_string(),
            Style::default()
                .fg(theme.warning())
                .add_modifier(Modifier::BOLD),
        ),
        FindingSeverity::Info => ("INFO ".to_string(), Style::default().fg(theme.muted())),
    }
}

fn render_watching(frame: &mut Frame, area: Rect, state: &FixState, theme: &EddaCraftTheme) {
    let title = format!(" Fix: {} ", state.finding.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Layout: header info, separator, context lines, footer.
    let chunks = Layout::vertical([
        Constraint::Length(4), // severity + file + message + suggestion
        Constraint::Length(1), // separator
        Constraint::Min(1),    // context lines
        Constraint::Length(1), // footer
    ])
    .split(inner);

    // ── Header ──────────────────────────────────────────────────────────
    let (badge_text, badge_style) = severity_badge(state.finding.severity, theme);

    let location = match state.finding.line {
        Some(l) => format!("{}:{l}", state.finding.file),
        None => state.finding.file.clone(),
    };

    let header_lines = vec![
        Line::from(vec![
            Span::styled(badge_text, badge_style),
            Span::raw(" "),
            Span::styled(
                location,
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            &state.finding.message,
            Style::default().fg(theme.fg()),
        )),
        Line::from(vec![
            Span::styled("Suggestion: ", Style::default().fg(theme.muted())),
            Span::styled(&state.finding.suggestion, Style::default().fg(theme.fg())),
        ]),
    ];
    frame.render_widget(Paragraph::new(Text::from(header_lines)), chunks[0]);

    // ── Separator ───────────────────────────────────────────────────────
    let sep = "─".repeat(chunks[1].width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            sep,
            Style::default().fg(theme.muted()),
        ))),
        chunks[1],
    );

    // ── Context lines ───────────────────────────────────────────────────
    let warning_line_idx = state
        .finding
        .line
        .map(|l| l.saturating_sub(state.context_start_line));

    let context: Vec<Line> = state
        .context_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_num = state.context_start_line + i;
            let is_warning_line = warning_line_idx == Some(i);

            let num_style = if is_warning_line {
                Style::default()
                    .fg(theme.error())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted())
            };

            let content_style = if is_warning_line {
                Style::default().fg(theme.error())
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(format!("{line_num:>4} "), num_style),
                Span::styled(line, content_style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(context)), chunks[2]);

    // ── Footer ──────────────────────────────────────────────────────────
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Waiting for file change... (press 'e' for inline editor, 's' to skip)",
            Style::default().fg(theme.muted()),
        ))),
        chunks[3],
    );
}

fn render_editing(frame: &mut Frame, area: Rect, state: &FixState, theme: &EddaCraftTheme) {
    let title = format!(" Editing: {} ", state.finding.file);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(title);

    // The Editor widget is a StatefulWidget; we need a mutable reference to
    // EditorState. Since we receive `state` as `&FixState`, we clone the
    // editor state for rendering (the widget only needs it for scroll
    // adjustment — the authoritative state lives in FixState).
    if let Some(ref editor_state) = state.editor {
        let mut editor_clone = editor_state.clone();
        let editor_widget = Editor::new(theme).block(block);
        frame.render_stateful_widget(editor_widget, area, &mut editor_clone);
    } else {
        // Fallback — should not happen, but render the block anyway.
        frame.render_widget(block, area);
    }
}

fn render_resolved(frame: &mut Frame, area: Rect, state: &FixState, theme: &EddaCraftTheme) {
    let title = format!(" Fix: {} ", state.finding.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success()))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            "\u{2713} Warning resolved!",
            Style::default()
                .fg(theme.success())
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(Span::styled(
            format!("Fixed: {}", state.finding.title),
            Style::default().fg(theme.fg()),
        )),
        Line::default(),
        Line::from(Span::styled(
            "Press Enter to continue",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_timed_out(frame: &mut Frame, area: Rect, state: &FixState, theme: &EddaCraftTheme) {
    let title = format!(" Fix: {} ", state.finding.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.warning()))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            "Time limit reached",
            Style::default()
                .fg(theme.warning())
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(Span::styled(
            format!(
                "The fix for '{}' was not applied in time.",
                state.finding.title
            ),
            Style::default().fg(theme.fg()),
        )),
        Line::from(Span::styled(
            "You can revisit this later.",
            Style::default().fg(theme.muted()),
        )),
        Line::default(),
        Line::from(Span::styled(
            "Press Enter or 's' to skip",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::super::discovery::{Finding, FindingSeverity, FindingSource};

    fn make_finding() -> Finding {
        Finding {
            file: "src/main.rs".to_string(),
            line: Some(10),
            severity: FindingSeverity::Error,
            source: FindingSource::AntiPattern,
            title: "hardcoded secret".to_string(),
            message: "API key found in source".to_string(),
            suggestion: "Move the secret to an environment variable".to_string(),
            warning_id: None,
        }
    }

    fn make_context_lines() -> Vec<String> {
        vec![
            "fn main() {".to_string(),
            "    let config = load_config();".to_string(),
            "    let db = connect(&config);".to_string(),
            "    let key = \"sk-1234567890\";".to_string(),
            "    let client = Client::new(key);".to_string(),
            "    start_server(client);".to_string(),
            "}".to_string(),
        ]
    }

    fn state_with_context() -> FixState {
        let mut state = FixState::new(make_finding());
        state.set_context(make_context_lines(), 7);
        state
    }

    #[test]
    fn renders_watching_phase_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = state_with_context();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_editing_phase_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = state_with_context();
        state.open_editor();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_resolved_phase_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = FixState::new(make_finding());
        state.set_check_result(true);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_timed_out_phase_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = FixState::new(make_finding());
        state.timeout_ticks = 1;
        state.tick();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_in_small_area_without_panic() {
        let backend = TestBackend::new(30, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = state_with_context();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_with_no_context_lines() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = FixState::new(make_finding());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_warning_severity_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = FixState::new(Finding {
            file: "src/lib.rs".to_string(),
            line: Some(5),
            severity: FindingSeverity::Warning,
            source: FindingSource::Architecture,
            title: "boundary violation".to_string(),
            message: "cross-module import".to_string(),
            suggestion: "use the public API".to_string(),
            warning_id: None,
        });
        state.set_context(make_context_lines(), 1);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_finding_without_line_number() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = FixState::new(Finding {
            file: "README.md".to_string(),
            line: None,
            severity: FindingSeverity::Info,
            source: FindingSource::AntiPattern,
            title: "formatting issue".to_string(),
            message: "inconsistent style".to_string(),
            suggestion: "run formatter".to_string(),
            warning_id: None,
        });
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn surface_render_delegates_correctly() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = state_with_context();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                Surface::render(&state, frame, frame.area(), &theme);
            })
            .unwrap();
    }

    #[test]
    fn renders_zero_size_area_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = state_with_context();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let zero = Rect::new(0, 0, 0, 0);
                render(frame, zero, &state, &theme);
            })
            .unwrap();
    }
}
