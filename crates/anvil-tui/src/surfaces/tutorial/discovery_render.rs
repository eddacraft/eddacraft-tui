use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::discovery::{DiscoveryPhase, DiscoveryState, FindingSeverity};

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Compute scroll offset to keep `selected` visible within `visible_rows`.
fn viewport_scroll(selected: usize, total: usize, visible_rows: usize) -> usize {
    if total <= visible_rows || visible_rows == 0 {
        return 0;
    }
    let max_offset = total.saturating_sub(visible_rows);
    selected
        .saturating_sub(visible_rows.saturating_sub(1))
        .min(max_offset)
}

pub fn render(frame: &mut Frame, area: Rect, state: &DiscoveryState, theme: &EddaCraftTheme) {
    match state.phase {
        DiscoveryPhase::Scanning {
            files_scanned,
            spinner_tick,
        } => {
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
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
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

    let sorted = results.sorted_findings();

    // Two-panel horizontal split: findings list (left) + detail (right).
    let panels =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);

    render_findings_list(frame, panels[0], &sorted, selected, theme);
    render_finding_detail(frame, panels[1], &sorted, selected, results, theme);
}

fn render_findings_list(
    frame: &mut Frame,
    area: Rect,
    sorted: &[&super::discovery::Finding],
    selected: usize,
    theme: &EddaCraftTheme,
) {
    let total = sorted.len();
    let list_title = if total == 0 {
        " Findings ".to_string()
    } else {
        format!(" Findings ({}/{total}) ", selected + 1)
    };
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(list_title)
        .title_style(
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        );
    let list_inner = list_block.inner(area);
    frame.render_widget(list_block, area);

    let visible_rows = list_inner.height as usize;
    let scroll_offset = viewport_scroll(selected, total, visible_rows);

    let finding_lines: Vec<Line> = sorted
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let is_selected = i == selected;
            let badge = f.severity.label();
            let badge_style = match f.severity {
                FindingSeverity::Error => Style::default()
                    .fg(theme.error())
                    .add_modifier(Modifier::BOLD),
                FindingSeverity::Warning => Style::default()
                    .fg(theme.warning())
                    .add_modifier(Modifier::BOLD),
                FindingSeverity::Info => Style::default().fg(theme.muted()),
            };

            let location = match f.line {
                Some(l) => format!("{}:{l}", f.file),
                None => f.file.clone(),
            };

            let indicator = if is_selected { ">> " } else { "   " };

            if is_selected {
                Line::from(vec![
                    Span::styled(
                        indicator,
                        Style::default()
                            .fg(theme.accent())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{badge:<5} "), badge_style),
                    Span::styled(format!("{location}  "), theme.highlighted()),
                    Span::styled(&f.title, theme.highlighted()),
                ])
            } else {
                let loc_style = match f.severity {
                    FindingSeverity::Error => Style::default().fg(theme.error()),
                    FindingSeverity::Warning => Style::default().fg(theme.warning()),
                    FindingSeverity::Info => Style::default().fg(theme.muted()),
                };
                Line::from(vec![
                    Span::styled(indicator, Style::default()),
                    Span::styled(format!("{badge:<5} "), badge_style),
                    Span::styled(location, loc_style),
                    Span::styled("  ", Style::default()),
                    Span::styled(f.title.clone(), Style::default().fg(theme.fg())),
                ])
            }
        })
        .collect();

    #[allow(clippy::cast_possible_truncation)]
    let list_para = Paragraph::new(Text::from(finding_lines)).scroll((scroll_offset as u16, 0));
    frame.render_widget(list_para, list_inner);
}

fn render_finding_detail(
    frame: &mut Frame,
    area: Rect,
    sorted: &[&super::discovery::Finding],
    selected: usize,
    results: &super::discovery::ScanResults,
    theme: &EddaCraftTheme,
) {
    let total = sorted.len();
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Detail ")
        .title_style(
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        );
    let detail_inner = detail_block.inner(area);
    frame.render_widget(detail_block, area);

    let Some(finding) = sorted.get(selected) else {
        return;
    };

    let location = match finding.line {
        Some(l) => format!("{}:{l}", finding.file),
        None => finding.file.clone(),
    };

    let sev_colour = match finding.severity {
        FindingSeverity::Error => theme.error(),
        FindingSeverity::Warning => theme.warning(),
        FindingSeverity::Info => theme.muted(),
    };

    let mut lines = vec![
        Line::from(Span::styled(
            &finding.title,
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("Severity: ", Style::default().fg(theme.muted())),
            Span::styled(
                finding.severity.label(),
                Style::default().fg(sev_colour).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Source:   ", Style::default().fg(theme.muted())),
            Span::styled(finding.source.label(), Style::default().fg(theme.fg())),
        ]),
        Line::from(vec![
            Span::styled("Location: ", Style::default().fg(theme.muted())),
            Span::styled(location, Style::default().fg(theme.accent())),
        ]),
        Line::default(),
        Line::from(Span::styled(
            &finding.message,
            Style::default().fg(theme.fg()),
        )),
    ];

    if !finding.suggestion.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "Suggestion",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            &finding.suggestion,
            Style::default().fg(theme.fg()),
        )));
    }

    // Summary at the bottom of the detail pane.
    let duration_s = results.duration_ms / 1000;
    let summary = format!(
        "{total} issue{} in {} file{} ({duration_s}s)  —  enter to continue",
        if total == 1 { "" } else { "s" },
        results.files_scanned,
        if results.files_scanned == 1 { "" } else { "s" },
    );

    // Fill space then add summary at the bottom.
    let used_lines = lines.len();
    let available = detail_inner.height as usize;
    if available > used_lines + 1 {
        for _ in 0..(available - used_lines - 1) {
            lines.push(Line::default());
        }
    }
    lines.push(Line::from(Span::styled(
        summary,
        Style::default().fg(theme.muted()),
    )));

    frame.render_widget(Paragraph::new(Text::from(lines)), detail_inner);
}

fn render_continue(frame: &mut Frame, area: Rect, state: &DiscoveryState, theme: &EddaCraftTheme) {
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
        Line::from(Span::styled(summary_line, Style::default().fg(theme.fg()))),
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
            truncated: false,
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
            truncated: false,
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
        let s0: String = buf0
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        let s1: String = buf1
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
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
