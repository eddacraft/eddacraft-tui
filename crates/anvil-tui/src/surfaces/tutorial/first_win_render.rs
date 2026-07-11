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

    // The explanation and diff share one wrapped paragraph sized by a `Min`
    // constraint so real guidance text (the shipped anti-pattern suggestions
    // run to ~1.5k characters across ~30 authored lines) is neither clipped
    // by a fixed-height chunk nor collapsed into run-on words; the consent
    // chrome keeps a fixed footprint at the bottom.
    let chunks = Layout::vertical([
        Constraint::Min(1),    // explanation + proposed diff, wrapped
        Constraint::Length(9), // shared ACTTUI consent chrome
    ])
    .split(inner);

    // ── Finding header + plain-language explanation ─────────────────────
    let (badge_text, badge_style) = severity_badge(offer.finding.severity, theme);
    let location = match offer.finding.line {
        Some(line) => format!("{}:{line}", offer.finding.file),
        None => offer.finding.file.clone(),
    };
    let body_style = Style::default().fg(theme.fg());
    let mut lines = vec![
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
            Span::styled(&offer.finding.title, body_style),
        ]),
        Line::default(),
    ];
    // Authored newlines in the message and suggestion are paragraph and list
    // structure — preserve them as separate lines instead of flattening them
    // into one run-on string (which loses the breaks entirely once rendered).
    let mut message_lines = offer.finding.message.split('\n');
    lines.push(Line::from(vec![
        Span::styled("Why it matters: ", Style::default().fg(theme.muted())),
        Span::styled(message_lines.next().unwrap_or_default(), body_style),
    ]));
    for message_line in message_lines {
        lines.push(Line::from(Span::styled(message_line, body_style)));
    }
    for suggestion_line in offer.finding.suggestion.split('\n') {
        lines.push(Line::from(Span::styled(suggestion_line, body_style)));
    }

    // ── Proposed diff — shown before any write ───────────────────────────
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(chunks[0].width as usize),
        Style::default().fg(theme.muted()),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            "Proposed change \u{2014} line {} (nothing is written without your consent):",
            offer.preview.line
        ),
        Style::default().fg(theme.muted()),
    )));
    lines.push(Line::from(Span::styled(
        format!("- {}", offer.preview.before),
        Style::default().fg(theme.error()),
    )));
    lines.push(Line::from(Span::styled(
        format!("+ {}", offer.preview.after),
        Style::default().fg(theme.success()),
    )));

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        chunks[0],
    );

    // ── Consent — shared ACTTUI chrome, unticked by default ─────────────
    consent::render(frame, chunks[1], &offer.consent, theme);
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
        draw_sized(state, 100, 30)
    }

    fn draw_sized(state: &FirstWinState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
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
    fn offer_preserves_suggestion_line_breaks() {
        // Authored newlines are paragraph/list structure: flattening them
        // produced run-on words ("shortcutfor"). Each authored line must
        // start on its own rendered row.
        let mut state = offer_state(false);
        state.offer.as_mut().expect("offer").finding.suggestion =
            "Think about what the value actually is. Don't use `any` as a shortcut\nfor \"I don't want to deal with types right now.\"\n\n1. **For API responses:** Define an interface for the response shape."
                .to_string();
        let out = draw(&state);
        assert!(!out.contains("shortcutfor"), "rendered: {out}");
        assert!(
            out.contains("as a shortcut"),
            "first authored line must be visible: {out}"
        );
        assert!(
            out.contains("for \"I don't want to deal with types right now.\""),
            "second authored line must be visible on its own row: {out}"
        );
        assert!(
            out.contains("1. **For API responses:**"),
            "list structure must survive: {out}"
        );
    }

    #[test]
    fn offer_wraps_long_diff_lines_instead_of_clipping() {
        let mut state = offer_state(false);
        let long_tail = "WRAPPED_DIFF_TAIL_MARKER";
        let before = format!(
            "const value: any = veryLongExpression({}); // {long_tail}",
            "argument, ".repeat(12)
        );
        {
            let offer = state.offer.as_mut().expect("offer");
            offer.preview.before = before.clone();
            offer.preview.after = before.replace(": any", ": unknown");
        }
        let out = draw(&state);
        assert!(
            out.contains(long_tail),
            "long diff lines must wrap, not clip: {out}"
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
