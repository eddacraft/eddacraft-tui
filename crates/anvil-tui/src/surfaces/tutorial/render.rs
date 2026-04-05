use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{TutorialPhase, TutorialState};

const MAX_OUTPUT_LINES: usize = 5;

/// Strip ANSI escape sequences from a string so raw control codes don't garble
/// the Ratatui output. Handles:
/// - CSI sequences: `ESC [ ... <final byte 0x40–0x7E>` (colours, cursor, SGR)
/// - OSC sequences: `ESC ] ... BEL` or `ESC ] ... ESC \` (window titles, hyperlinks)
/// - Other two-byte escapes: `ESC <char>` (consumed as introducer + single byte)
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if let Some(next) = chars.next() {
                if next == '[' {
                    // CSI: consume until final byte 0x40–0x7E.
                    for c in chars.by_ref() {
                        if c.is_ascii() && (0x40..=0x7E).contains(&(c as u8)) {
                            break;
                        }
                    }
                } else if next == ']' {
                    // OSC: consume until BEL (\x07) or ST (ESC \).
                    for c in chars.by_ref() {
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' {
                            // Consume the backslash of ESC \.
                            let _ = chars.next();
                            break;
                        }
                    }
                }
                // Otherwise: two-byte escape — `next` is already consumed.
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Collect up to `MAX_OUTPUT_LINES` from a string, returning the collected
/// lines and whether there were more lines beyond the limit.
fn collect_lines(text: &str) -> (Vec<String>, bool) {
    let mut lines = Vec::with_capacity(MAX_OUTPUT_LINES + 1);
    for line in text.lines().take(MAX_OUTPUT_LINES + 1) {
        lines.push(strip_ansi(line));
    }
    let has_more = lines.len() > MAX_OUTPUT_LINES;
    lines.truncate(MAX_OUTPUT_LINES);
    (lines, has_more)
}

pub fn render(frame: &mut Frame, area: Rect, state: &TutorialState, theme: &EddaCraftTheme) {
    match state.phase {
        TutorialPhase::PathSelect => {
            render_path_select(frame, area, state, theme);
        }
        TutorialPhase::Running => {
            let chunks = Layout::vertical([
                Constraint::Length(3), // Progress indicator
                Constraint::Min(6),    // Content
            ])
            .split(area);

            render_step_progress(frame, chunks[0], state, theme);
            render_step_content(frame, chunks[1], state, theme);
        }
        TutorialPhase::Complete => {
            render_complete(frame, area, state, theme);
        }
    }
}

fn render_path_select(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Choose a Tutorial Path ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<Line> = state
        .paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let selected = i == state.path_selected;
            let indicator = if selected { ">> " } else { "  " };
            let name_style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            Line::from(vec![
                Span::styled(indicator, name_style),
                Span::styled(path.label(), name_style),
                Span::styled(
                    format!("  {}", path.description()),
                    Style::default().fg(theme.muted()),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(items)), inner);
}

fn render_step_progress(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    let path_label = state.chosen_path.map_or("Tutorial", TutorialPath::label);

    let total = state.steps.len();
    let completed = state.steps.iter().filter(|s| s.completed).count();

    let spans: Vec<Span> = state
        .steps
        .iter()
        .enumerate()
        .flat_map(|(i, _step)| {
            let style = match i.cmp(&state.current_step) {
                std::cmp::Ordering::Less => Style::default().fg(theme.success()),
                std::cmp::Ordering::Equal => Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
                std::cmp::Ordering::Greater => Style::default().fg(theme.muted()),
            };
            let marker = match i.cmp(&state.current_step) {
                std::cmp::Ordering::Less => "*",
                std::cmp::Ordering::Equal => ">",
                std::cmp::Ordering::Greater => "o",
            };
            let separator = if i < total - 1 { " - " } else { "" };
            vec![
                Span::styled(marker, style),
                Span::styled(separator, Style::default().fg(theme.muted())),
            ]
        })
        .collect();

    let lines = vec![
        Line::from(Span::styled(
            format!("{path_label}  ({completed}/{total})"),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(spans),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_step_content(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    let Some(step) = state.steps.get(state.current_step) else {
        return;
    };

    let border_color = if step.output.as_ref().is_some_and(|o| !o.success) {
        theme.error()
    } else {
        theme.accent()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ", step.title));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::default(),
        Line::from(Span::styled(
            &step.description,
            Style::default().fg(theme.fg()),
        )),
        Line::default(),
        Line::from(Span::styled(
            &step.instruction,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ];

    if let Some(output) = &step.output {
        lines.push(Line::default());
        let status_color = if output.success {
            theme.success()
        } else {
            theme.error()
        };
        let status_label = if output.success {
            "✓ success"
        } else {
            "✗ failed"
        };
        let exit_label = output
            .exit_code
            .map_or_else(|| " (no exit code)".to_string(), |c| format!(" (exit {c})"));
        lines.push(Line::from(Span::styled(
            format!("{status_label}{exit_label}"),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        )));

        if !output.stdout.is_empty() {
            let (stdout_lines, has_more) = collect_lines(&output.stdout);
            for line in &stdout_lines {
                lines.push(Line::from(Span::styled(
                    line.clone(),
                    Style::default().fg(theme.fg()),
                )));
            }
            if has_more {
                lines.push(Line::from(Span::styled(
                    "… (more lines truncated)",
                    Style::default().fg(theme.muted()),
                )));
            }
        }
        if !output.stderr.is_empty() {
            let (stderr_lines, has_more) = collect_lines(&output.stderr);
            for line in &stderr_lines {
                lines.push(Line::from(Span::styled(
                    line.clone(),
                    Style::default().fg(theme.error()),
                )));
            }
            if has_more {
                lines.push(Line::from(Span::styled(
                    "… (more lines truncated)",
                    Style::default().fg(theme.muted()),
                )));
            }
        }
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_complete(frame: &mut Frame, area: Rect, state: &TutorialState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success()))
        .title(" Well Done ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let path_label = state
        .chosen_path
        .map_or("the tutorial", TutorialPath::label);
    let total = state.steps.len();

    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            format!("You completed all {total} steps of the {path_label} tutorial."),
            Style::default().fg(theme.fg()),
        )),
        Line::default(),
        Line::from(Span::styled(
            "Press enter to choose another tutorial path, or q to quit.",
            Style::default().fg(theme.accent()),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

use super::TutorialPath;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn renders_without_panic_path_select() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = TutorialState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_path_select() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = TutorialState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    Surface::surface_name(&state),
                    Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_running_phase() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Policy);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    Surface::surface_name(&state),
                    Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_complete_phase() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Drift);
        // Complete all steps
        for step in &mut state.steps {
            step.completed = true;
        }
        state.phase = TutorialPhase::Complete;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    Surface::surface_name(&state),
                    Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    // --- strip_ansi tests ---

    #[test]
    fn strip_ansi_removes_csi_colour() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    }

    #[test]
    fn strip_ansi_removes_osc_bel_terminated() {
        // OSC terminated by BEL (\x07)
        assert_eq!(strip_ansi("\x1b]0;title\x07text"), "text");
    }

    #[test]
    fn strip_ansi_removes_osc_st_terminated() {
        // OSC terminated by ST (ESC \)
        assert_eq!(
            strip_ansi("\x1b]8;;https://x.com\x1b\\link\x1b]8;;\x1b\\"),
            "link"
        );
    }

    #[test]
    fn strip_ansi_passthrough_plain_text() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn strip_ansi_bare_esc_at_end() {
        // Trailing ESC with no following char — nothing to consume.
        assert_eq!(strip_ansi("text\x1b"), "text");
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = TutorialState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }
}
