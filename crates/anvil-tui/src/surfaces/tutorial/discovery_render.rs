use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::discovery::{DiscoveryPhase, DiscoveryState, FindingSeverity};

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub fn render(frame: &mut Frame, area: Rect, state: &DiscoveryState, theme: &EddaCraftTheme) {
    match state.phase {
        DiscoveryPhase::Scanning { files_scanned, spinner_tick } => {
            render_scanning(frame, area, files_scanned, spinner_tick, theme);
        }
        DiscoveryPhase::Results { selected } => {
            render_results(frame, area, state, selected, theme);
        }
        DiscoveryPhase::Continue => {
            render_continue(frame, area, state, theme);
        }
    }
}

fn render_scanning(
    frame: &mut Frame,
    area: Rect,
    files_scanned: usize,
    spinner_tick: usize,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Scanning Project ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let spinner_char = SPINNER_FRAMES[spinner_tick % SPINNER_FRAMES.len()];

    let lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled(
                format!("{spinner_char} "),
                Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Scanning project...",
                Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            format!("{files_scanned} files scanned"),
            Style::default().fg(theme.muted()),
        )),
        Line::default(),
        Line::from(Span::styled(
            "Press 's' to skip",
            Style::default().fg(theme.muted()),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_results(
    frame: &mut Frame,
    area: Rect,
    state: &DiscoveryState,
    selected: usize,
    theme: &EddaCraftTheme,
) {
    let Some(results) = &state.results else {
        return;
    };

    let total_issues = results.findings.len();
    let top = results.top_findings(super::discovery::MAX_FINDINGS_SHOWN);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Scan Results ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Reserve bottom row for summary line.
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    // Build findings list.
    let finding_lines: Vec<Line> = top
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let is_selected = i == selected;
            let (badge, badge_style) = match f.severity {
                FindingSeverity::Error => (
                    " ERR ",
                    Style::default().fg(theme.error()).add_modifier(Modifier::BOLD),
                ),
                FindingSeverity::Warning => (
                    "WARN ",
                    Style::default().fg(theme.warning()).add_modifier(Modifier::BOLD),
                ),
                FindingSeverity::Info => (
                    "INFO ",
                    Style::default().fg(theme.muted()),
                ),
            };

            let location = match f.line {
                Some(l) => format!("{}:{}", f.file, l),
                None => f.file.clone(),
            };

            if is_selected {
                let title_style = theme.highlighted();
                Line::from(vec![
                    Span::styled(badge, badge_style),
                    Span::raw(" "),
                    Span::styled(format!("{location}  {}", f.title), title_style),
                ])
            } else {
                let (loc_style, title_style) = match f.severity {
                    FindingSeverity::Error => (
                        Style::default().fg(theme.error()),
                        Style::default().fg(theme.fg()),
                    ),
                    FindingSeverity::Warning => (
                        Style::default().fg(theme.warning()),
                        Style::default().fg(theme.fg()),
                    ),
                    FindingSeverity::Info => (
                        Style::default().fg(theme.muted()),
                        Style::default().fg(theme.muted()),
                    ),
                };
                Line::from(vec![
                    Span::styled(badge, badge_style),
                    Span::raw(" "),
                    Span::styled(location, loc_style),
                    Span::raw("  "),
                    Span::styled(f.title.clone(), title_style),
                ])
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(finding_lines)),
        chunks[0],
    );

    // Summary line.
    let duration_s = results.duration_ms / 1000;
    let summary = format!(
        "Found {total_issues} issue{} in {} file{} ({duration_s}s)  —  enter to continue",
        if total_issues == 1 { "" } else { "s" },
        results.files_scanned,
        if results.files_scanned == 1 { "" } else { "s" },
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            summary,
            Style::default().fg(theme.muted()),
        ))),
        chunks[1],
    );
}

fn render_continue(
    frame: &mut Frame,
    area: Rect,
    state: &DiscoveryState,
    theme: &EddaCraftTheme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success()))
        .title(" Scan Complete ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let summary_line = match &state.results {
        Some(r) if !r.findings.is_empty() => {
            let total = r.findings.len();
            format!(
                "Found {total} issue{} across {} file{}.",
                if total == 1 { "" } else { "s" },
                r.files_scanned,
                if r.files_scanned == 1 { "" } else { "s" },
            )
        }
        Some(r) => format!(
            "No issues found in {} file{}.",
            r.files_scanned,
            if r.files_scanned == 1 { "" } else { "s" },
        ),
        None => "Scan skipped.".to_string(),
    };

    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            summary_line,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use eddacraft_tui::keyboard::Action;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::super::discovery::{Finding, FindingSource, ScanResults};

    fn make_finding(severity: FindingSeverity, title: &str) -> Finding {
        Finding {
            file: "src/main.rs".to_string(),
            line: Some(42),
            severity,
            source: FindingSource::AntiPattern,
            title: title.to_string(),
            message: "test message".to_string(),
            suggestion: "fix it".to_string(),
        }
    }

    #[test]
    fn renders_scanning_phase_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = DiscoveryState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_results_phase_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.set_results(ScanResults {
            findings: vec![
                make_finding(FindingSeverity::Error, "Secret key exposed"),
                make_finding(FindingSeverity::Warning, "TODO left in code"),
                make_finding(FindingSeverity::Info, "Formatting style"),
            ],
            files_scanned: 120,
            duration_ms: 3200,
        });
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_continue_phase_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.skip_scan();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_continue_with_findings_summary() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.set_results(ScanResults {
            findings: vec![make_finding(FindingSeverity::Error, "e")],
            files_scanned: 50,
            duration_ms: 1500,
        });
        // Advance to continue phase
        state.handle_key(Action::Select);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_in_small_area_without_panic() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = DiscoveryState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn spinner_advances_visually() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        // Render at tick 0
        let mut state = DiscoveryState::new();
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let buf0 = terminal.backend().buffer().clone();

        // Render at tick 1
        state.tick();
        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
        let buf1 = terminal.backend().buffer().clone();

        // The spinner character changes between ticks.
        let s0: String = buf0.content().iter().map(|c| c.symbol()).collect();
        let s1: String = buf1.content().iter().map(|c| c.symbol()).collect();
        assert_ne!(s0, s1, "spinner should visually change between ticks");
    }

    #[test]
    fn surface_render_delegates_correctly() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = DiscoveryState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                Surface::render(&state, frame, frame.area(), &theme);
            })
            .unwrap();
    }
}

