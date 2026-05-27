use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::discovery::{DiscoveryPhase, DiscoveryState, FindingSeverity};
use crate::shell::inset_content;

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
    let area = inset_content(area);
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

    // Summary at the bottom of the detail pane. When the walker hit the
    // scan cap, say so explicitly — otherwise a large repo looks like it
    // ran clean when really we only looked at the first N files. The
    // "first" prefix is skipped for files_scanned <= 1 so "the first 1
    // file" doesn't leak out in the degenerate case where only one
    // candidate was successfully read.
    let duration_s = results.duration_ms / 1000;
    let scope = if results.truncated && results.files_scanned > 1 {
        "first "
    } else {
        ""
    };
    let truncated_note = if results.truncated {
        " (scan limited)"
    } else {
        ""
    };
    let summary = format!(
        "{total} issue{} in {scope}{} file{}{truncated_note} ({duration_s}s)  —  enter to continue",
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

    // "the first" is suppressed unless at least two files were read — a
    // single-file truncation is a scanner edge case (499 failed reads)
    // and rendering "the first 1 file" reads badly. files_scanned == 0
    // is handled as a distinct branch since we have nothing meaningful
    // to report.
    let summary_line = match &state.results {
        Some(r) if r.files_scanned == 0 => "No files could be scanned.".to_string(),
        Some(r) if !r.findings.is_empty() => {
            let total = r.findings.len();
            let scope = if r.truncated && r.files_scanned > 1 {
                "the first "
            } else {
                ""
            };
            format!(
                "Found {total} issue{} across {scope}{} file{}.",
                if total == 1 { "" } else { "s" },
                r.files_scanned,
                if r.files_scanned == 1 { "" } else { "s" },
            )
        }
        Some(r) => {
            let scope = if r.truncated && r.files_scanned > 1 {
                "the first "
            } else {
                ""
            };
            format!(
                "No issues found in {scope}{} file{}.",
                r.files_scanned,
                if r.files_scanned == 1 { "" } else { "s" },
            )
        }
        None => "Scan skipped.".to_string(),
    };

    // Skip the "re-run on a subdirectory" hint when files_scanned == 0 —
    // the cap is irrelevant if nothing was read; the real failure is
    // elsewhere (permissions, filter) and we don't want to send the
    // user chasing the wrong fix.
    let truncated_note = matches!(&state.results, Some(r) if r.truncated && r.files_scanned > 0);

    // SCAN-004: surface gitignore provenance. The scan honours .gitignore by
    // default, so "no issues found" could mean "clean" or "we never opened the
    // ignored directory that held the secret". scan_project forces this to 0
    // when the scan was truncated or ANVIL_SCAN_ALL was set, so the note only
    // appears when the count is honestly attributable to gitignore.
    let skipped_by_ignore = state
        .results
        .as_ref()
        .map_or(0, |r| r.files_skipped_by_ignore);

    let mut lines = vec![
        Line::default(),
        Line::from(Span::styled(summary_line, Style::default().fg(theme.fg()))),
    ];
    if truncated_note {
        lines.push(Line::from(Span::styled(
            "Scan was limited — re-run on a subdirectory for complete coverage.",
            Style::default().fg(theme.muted()),
        )));
    }
    if skipped_by_ignore > 0 {
        lines.push(Line::from(Span::styled(
            format!(
                "{skipped_by_ignore} file{} skipped by .gitignore — set ANVIL_SCAN_ALL=1 to scan {}.",
                if skipped_by_ignore == 1 { "" } else { "s" },
                if skipped_by_ignore == 1 { "it" } else { "them" },
            ),
            Style::default().fg(theme.muted()),
        )));
    }
    lines.extend([
        Line::default(),
        Line::from(Span::styled(
            "Press Enter to continue",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ]);

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
            warning_id: None,
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
            files_skipped_by_ignore: 0,
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
            files_skipped_by_ignore: 0,
        });
        // Advance to continue phase
        state.handle_key(Action::Select);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn continue_screen_notes_truncation() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.set_results(ScanResults {
            findings: vec![make_finding(FindingSeverity::Error, "e")],
            files_scanned: 500,
            duration_ms: 1500,
            truncated: true,
            files_skipped_by_ignore: 0,
        });
        state.handle_key(Action::Select);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            rendered.contains("the first 500 file"),
            "truncated summary should say `the first N files`: {rendered}"
        );
        assert!(
            rendered.contains("Scan was limited"),
            "truncated continue screen should carry the limited-scan note: {rendered}"
        );
    }

    #[test]
    fn continue_screen_notes_gitignore_skips() {
        // SCAN-004: when discovery dropped candidate files because .gitignore
        // excluded them, the continue screen must say so — otherwise "no
        // issues found" hides the possibility that the secret lived in an
        // ignored directory we never opened.
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.set_results(ScanResults {
            findings: Vec::new(),
            files_scanned: 40,
            duration_ms: 900,
            truncated: false,
            files_skipped_by_ignore: 7,
        });
        state.handle_key(Action::Select);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            rendered.contains("7 file") && rendered.contains(".gitignore"),
            "continue screen should report files skipped by .gitignore: {rendered}"
        );
        assert!(
            rendered.contains("ANVIL_SCAN_ALL"),
            "skip note should point at the ANVIL_SCAN_ALL override: {rendered}"
        );
    }

    #[test]
    fn continue_screen_omits_gitignore_note_when_zero() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.set_results(ScanResults {
            findings: Vec::new(),
            files_scanned: 40,
            duration_ms: 900,
            truncated: false,
            files_skipped_by_ignore: 0,
        });
        state.handle_key(Action::Select);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            !rendered.contains(".gitignore"),
            "no gitignore skips means no skip note should render: {rendered}"
        );
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
    fn detail_summary_notes_truncation() {
        // set_results with findings lands on the Results/detail phase;
        // we intentionally do NOT advance to the continue screen so the
        // detail-pane summary is what renders. Widen the backend so the
        // right-hand Detail pane has room for the full summary line —
        // at 80 cols the pane is ~36 wide and the summary clips before
        // "(scan limited)".
        let backend = TestBackend::new(140, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.set_results(ScanResults {
            findings: vec![make_finding(FindingSeverity::Error, "e")],
            files_scanned: 500,
            duration_ms: 1500,
            truncated: true,
            files_skipped_by_ignore: 0,
        });
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            rendered.contains("first 500 file"),
            "detail-pane summary should say `first N file(s)`: {rendered}"
        );
        assert!(
            rendered.contains("(scan limited)"),
            "detail-pane summary should carry the `(scan limited)` tag: {rendered}"
        );
    }

    #[test]
    fn truncation_note_visible_in_small_area() {
        // Narrow terminals were silently clipping the truncation line.
        // This test doesn't require the full string to fit — just that
        // enough of it lands in the buffer for the user to recognise the
        // warning.
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.set_results(ScanResults {
            findings: Vec::new(),
            files_scanned: 500,
            duration_ms: 1500,
            truncated: true,
            files_skipped_by_ignore: 0,
        });
        state.handle_key(Action::Select);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            rendered.contains("limited"),
            "narrow continue screen should still show the truncation warning: {rendered}"
        );
    }

    #[test]
    fn continue_screen_handles_zero_files_scanned() {
        // Degenerate case: every candidate panicked or failed to read,
        // so files_scanned saturates to 0 even though truncated==true.
        // We should not render "the first 0 files" — that's grammatically
        // broken and semantically wrong (we scanned nothing). Expect a
        // distinct message and no truncation-hint.
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = DiscoveryState::new();
        state.set_results(ScanResults {
            findings: Vec::new(),
            files_scanned: 0,
            duration_ms: 500,
            truncated: true,
            files_skipped_by_ignore: 0,
        });
        state.handle_key(Action::Select);
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            !rendered.contains("the first 0"),
            "zero-files case must not render `the first 0`: {rendered}"
        );
        assert!(
            rendered.contains("No files could be scanned"),
            "zero-files case should surface a distinct message: {rendered}"
        );
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
