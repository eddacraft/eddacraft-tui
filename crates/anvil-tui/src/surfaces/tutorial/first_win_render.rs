//! Renderer for the WOW-005 first-win surface.
//!
//! The consent block is drawn by the shared ACTTUI chrome
//! (`activation::consent::render`) — this module renders only the finding
//! explanation and the proposed diff around it.

use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::surfaces::activation::consent;

use super::first_win::{FirstWinOffer, FirstWinPhase, FirstWinState};
use super::fix_render::severity_badge;

pub fn render(frame: &mut Frame, area: Rect, state: &FirstWinState, theme: &EddaCraftTheme) {
    match &state.phase {
        FirstWinPhase::Clean { files_scanned } => {
            render_clean(frame, area, *files_scanned, theme);
        }
        FirstWinPhase::Offer => {
            if let Some(offer) = state.offer.as_ref() {
                render_offer(frame, area, offer, theme);
            }
        }
        FirstWinPhase::Done { applied, message } => {
            render_done(frame, area, *applied, message, theme);
        }
    }
}

fn render_clean(frame: &mut Frame, area: Rect, files_scanned: usize, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success()))
        .title(" Scan result ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            format!(
                "\u{2713} Your repository scan came back clean \u{2014} no findings in {files_scanned} scanned files."
            ),
            Style::default()
                .fg(theme.success())
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(Span::styled(
            "The tutorial ahead demonstrates anvil's checks on clearly labelled example",
            Style::default().fg(theme.fg()),
        )),
        Line::from(Span::styled(
            "findings \u{2014} they are examples, not findings from your code.",
            Style::default().fg(theme.fg()),
        )),
        Line::default(),
        Line::from(Span::styled(
            "Press Enter to choose a tutorial path",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_offer(frame: &mut Frame, area: Rect, offer: &FirstWinOffer, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(" Your first win ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(5), // headline + location + why it matters
        Constraint::Length(1), // separator
        Constraint::Length(4), // proposed diff
        Constraint::Min(8),    // shared ACTTUI consent chrome
    ])
    .split(inner);

    // ── Finding header + plain-language explanation ─────────────────────
    let (badge_text, badge_style) = severity_badge(offer.finding.severity, theme);
    let location = match offer.finding.line {
        Some(line) => format!("{}:{line}", offer.finding.file),
        None => offer.finding.file.clone(),
    };
    let header = vec![
        Line::from(vec![
            Span::styled(badge_text, badge_style),
            Span::raw(" "),
            Span::styled(
                location,
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(&offer.finding.title, Style::default().fg(theme.fg())),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled("Why it matters: ", Style::default().fg(theme.muted())),
            Span::styled(&offer.finding.message, Style::default().fg(theme.fg())),
        ]),
        Line::from(Span::styled(
            &offer.finding.suggestion,
            Style::default().fg(theme.fg()),
        )),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(header)).wrap(Wrap { trim: false }),
        chunks[0],
    );

    // ── Separator ────────────────────────────────────────────────────────
    let sep = "\u{2500}".repeat(chunks[1].width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            sep,
            Style::default().fg(theme.muted()),
        ))),
        chunks[1],
    );

    // ── Proposed diff — shown before any write ───────────────────────────
    let diff = vec![
        Line::from(Span::styled(
            format!(
                "Proposed change \u{2014} line {} (nothing is written without your consent):",
                offer.preview.line
            ),
            Style::default().fg(theme.muted()),
        )),
        Line::from(Span::styled(
            format!("- {}", offer.preview.before),
            Style::default().fg(theme.error()),
        )),
        Line::from(Span::styled(
            format!("+ {}", offer.preview.after),
            Style::default().fg(theme.success()),
        )),
    ];
    frame.render_widget(Paragraph::new(Text::from(diff)), chunks[2]);

    // ── Consent — shared ACTTUI chrome, unticked by default ─────────────
    consent::render(frame, chunks[3], &offer.consent, theme);
}

fn render_done(
    frame: &mut Frame,
    area: Rect,
    applied: bool,
    message: &str,
    theme: &EddaCraftTheme,
) {
    let (border, headline, headline_style) = if applied {
        (
            theme.success(),
            "\u{2713} Fix applied \u{2014} your first win on this repository.",
            Style::default()
                .fg(theme.success())
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            theme.warning(),
            "The fix was not applied.",
            Style::default()
                .fg(theme.warning())
                .add_modifier(Modifier::BOLD),
        )
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(" Your first win ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let lines = vec![
        Line::default(),
        Line::from(Span::styled(headline, headline_style)),
        Line::default(),
        Line::from(Span::styled(message, Style::default().fg(theme.fg()))),
        Line::default(),
        Line::from(Span::styled(
            "Press Enter to choose a tutorial path",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::super::discovery::{Finding, FindingSeverity, FindingSource};
    use super::super::first_win::{FirstWinState, FixPreview};
    use super::*;
    use crate::surface::Surface;

    fn draw(state: &FirstWinState) -> String {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;
        terminal
            .draw(|frame| state.render(frame, frame.area(), &theme))
            .unwrap();
        let buf = terminal.backend().buffer();
        let area = buf.area;
        let mut out = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn offer_state(project_writes_gated: bool) -> FirstWinState {
        FirstWinState::offer(
            Finding {
                file: "src/app.ts".to_string(),
                line: Some(3),
                severity: FindingSeverity::Warning,
                source: FindingSource::AntiPattern,
                title: "Avoid `any` type annotations".to_string(),
                message: "`any` disables type checking for this value".to_string(),
                suggestion: "Use `unknown` and narrow the type explicitly.".to_string(),
                warning_id: Some("AP-003".to_string()),
            },
            FixPreview {
                line: 3,
                before: "const value: any = source;".to_string(),
                after: "const value: unknown = source;".to_string(),
            },
            project_writes_gated,
        )
    }

    #[test]
    fn offer_renders_finding_diff_and_shared_consent_chrome() {
        let out = draw(&offer_state(false));
        assert!(out.contains("Your first win"), "rendered: {out}");
        assert!(out.contains("src/app.ts:3"), "rendered: {out}");
        assert!(out.contains("Why it matters"), "rendered: {out}");
        // The diff is shown before any write.
        assert!(
            out.contains("- const value: any = source;"),
            "rendered: {out}"
        );
        assert!(
            out.contains("+ const value: unknown = source;"),
            "rendered: {out}"
        );
        // Shared ACTTUI consent chrome: unticked row + Consent block title.
        assert!(out.contains("Consent"), "rendered: {out}");
        assert!(
            out.contains("[ ] Apply this fix to src/app.ts:3"),
            "rendered: {out}"
        );
    }

    #[test]
    fn gated_offer_renders_suppression_reason() {
        let out = draw(&offer_state(true));
        assert!(
            out.contains("ANVIL_HOME gates project writes"),
            "rendered: {out}"
        );
    }

    #[test]
    fn clean_result_is_honest_about_examples() {
        let out = draw(&FirstWinState::clean(42));
        assert!(out.contains("came back clean"), "rendered: {out}");
        assert!(out.contains("42 scanned files"), "rendered: {out}");
        assert!(
            out.contains("not findings from your code"),
            "rendered: {out}"
        );
    }

    #[test]
    fn done_renders_applied_summary() {
        let mut state = offer_state(false);
        state.mark_outcome(true, "Applied fix in src/app.ts:3");
        let out = draw(&state);
        assert!(out.contains("your first win"), "rendered: {out}");
        assert!(
            out.contains("Applied fix in src/app.ts:3"),
            "rendered: {out}"
        );
    }

    #[test]
    fn done_renders_failure_honestly() {
        let mut state = offer_state(false);
        state.mark_outcome(false, "Line 3 is out of range for src/app.ts");
        let out = draw(&state);
        assert!(out.contains("was not applied"), "rendered: {out}");
        assert!(out.contains("out of range"), "rendered: {out}");
    }
}
