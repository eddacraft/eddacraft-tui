use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use super::{TutorialPhase, TutorialState};
use crate::shell::inset_content;

const MAX_OUTPUT_LINES: usize = 5;

/// Some terminals (notably older Windows consoles and a few SSH multiplexers)
/// render the geometric-shape glyphs we use for the step progress indicator
/// as double-wide or as replacement boxes. Setting `ANVIL_ASCII=1` (or
/// `true`, case-insensitive) swaps the progress glyphs for their ASCII
/// counterparts so the layout stays aligned. Any other value — including
/// `ANVIL_ASCII=false` — leaves the Unicode glyphs in place.
///
/// Returns `(complete, current, pending)`.
fn progress_glyphs() -> (&'static str, &'static str, &'static str) {
    if ascii_mode() {
        ("#", ">", "-")
    } else {
        ("\u{25cf}", "\u{25c9}", "\u{25cb}")
    }
}

fn ascii_mode() -> bool {
    std::env::var("ANVIL_ASCII").is_ok_and(|v| {
        let v = v.trim();
        v.eq_ignore_ascii_case("1") || v.eq_ignore_ascii_case("true")
    })
}

/// Fit `title` inside a block whose outer width is `area_width`. A Ratatui
/// block title is rendered between two border cells and a space on each side,
/// so the effective budget is `area_width - 4`. Overflowing titles are
/// truncated with an ellipsis so they never punch through the border line.
fn fit_block_title(title: &str, area_width: u16) -> String {
    let budget = (area_width as usize).saturating_sub(4);
    if budget == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(title) <= budget {
        return title.to_string();
    }
    let ellipsis = "…";
    let ellipsis_w = UnicodeWidthStr::width(ellipsis);
    let target = budget.saturating_sub(ellipsis_w);
    let mut out = String::new();
    let mut w = 0;
    for ch in title.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > target {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push_str(ellipsis);
    out
}

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
    let area = inset_content(area);
    match state.phase {
        TutorialPhase::PathSelect => {
            render_path_select(frame, area, state, theme);
        }
        TutorialPhase::Running if state.editor.is_some() => {
            render_editor(frame, area, state, theme);
        }
        TutorialPhase::Running => {
            let has_notice = state.static_mode || state.resuming_notice.is_some();
            if has_notice {
                // Reserve enough rows for the notice to wrap on narrow
                // terminals — a fixed Length(1) silently clipped the
                // watcher-unavailable notice on SSH / split-pane widths.
                let notice_text = if state.static_mode {
                    state.static_notice.as_deref().unwrap_or("")
                } else {
                    state.resuming_notice.as_deref().unwrap_or("")
                };
                let notice_rows = notice_row_count(notice_text, area.width).max(1);
                let chunks = Layout::vertical([
                    Constraint::Length(notice_rows),
                    Constraint::Length(3), // Progress indicator
                    Constraint::Min(6),    // Content
                ])
                .split(area);

                if state.static_mode {
                    render_static_notice(frame, chunks[0], state, theme);
                } else {
                    render_resuming_notice(frame, chunks[0], state, theme);
                }
                render_step_progress(frame, chunks[1], state, theme);
                render_step_content(frame, chunks[2], state, theme);
            } else {
                let chunks = Layout::vertical([
                    Constraint::Length(3), // Progress indicator
                    Constraint::Min(6),    // Content
                ])
                .split(area);

                render_step_progress(frame, chunks[0], state, theme);
                render_step_content(frame, chunks[1], state, theme);
            }
        }
        TutorialPhase::Complete => {
            render_complete(frame, area, state, theme);
        }
    }
}

/// Render the inline editor for a create/edit step. The `Editor` widget is a
/// `StatefulWidget`; we clone the authoritative `EditorState` for rendering
/// because rendering only has `&self`. The clone means any scroll adjustment
/// the widget makes is discarded, so we record the viewport height into
/// `state.editor_viewport` and let the key handler keep the cursor visible on
/// the real state after each move. A footer shows the write target, any write
/// error, and the key hints.
fn render_editor(frame: &mut Frame, area: Rect, state: &TutorialState, theme: &EddaCraftTheme) {
    use eddacraft_tui::widgets::editor::Editor;

    let Some(editor_state) = state.editor.as_ref() else {
        return;
    };

    let target = state.edit_path.as_deref().unwrap_or("file");
    let title = format!(" Editing {target} ");
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .title(title);

    let chunks = Layout::vertical([
        Constraint::Min(3),    // editor
        Constraint::Length(1), // footer (error or hint)
    ])
    .split(area);

    // Record the inner text height (area minus top/bottom border) so the key
    // handler can scroll to keep the cursor visible.
    state
        .editor_viewport
        .set(chunks[0].height.saturating_sub(2));

    let mut editor_clone = editor_state.clone();
    let editor_widget = Editor::new(theme).block(block);
    frame.render_stateful_widget(editor_widget, chunks[0], &mut editor_clone);

    let footer = if let Some(err) = state.edit_error.as_deref() {
        Line::from(Span::styled(
            err.to_string(),
            Style::default()
                .fg(theme.error())
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(
            "ctrl-s save · enter newline · esc cancel",
            Style::default().fg(theme.muted()),
        ))
    };
    frame.render_widget(Paragraph::new(footer), chunks[1]);
}

/// Conservative row estimate for the notice strip. Uses display width so
/// that multi-byte characters (e.g. the em-dash in the watcher-unavailable
/// notice) are counted correctly; wraps whole-string rather than
/// word-by-word to avoid under-allocating when a single long token would
/// push the layout past the reserved rows.
fn notice_row_count(notice: &str, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let display_width = UnicodeWidthStr::width(notice);
    let rows = display_width.div_ceil(width as usize).max(1);
    u16::try_from(rows).unwrap_or(u16::MAX)
}

fn render_static_notice(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    if let Some(notice) = &state.static_notice {
        let line = Line::from(Span::styled(
            notice.as_str(),
            Style::default()
                .fg(theme.warning())
                .add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: false }), area);
    }
}

fn render_resuming_notice(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    if let Some(notice) = &state.resuming_notice {
        let line = Line::from(Span::styled(
            notice.as_str(),
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: false }), area);
    }
}

/// WOW-003: the per-domain finding-count suffix appended to a picker row
/// when real scan findings exist for the path's domain.
fn picker_count_suffix(count: usize) -> String {
    if count == 1 {
        "  ·  1 finding in your repo".to_string()
    } else {
        format!("  ·  {count} findings in your repo")
    }
}

/// Render width (display columns) of a single path's line in the
/// selector. Mirrors the spans built in `render_path_select` so the
/// height calculation stays in sync with what is actually drawn.
fn path_line_width(path: TutorialPath, done: bool, finding_count: Option<usize>) -> u16 {
    // ">> " or "   " indicator
    let mut width: usize = 3;
    if done {
        // "\u{2713} " — checkmark + space
        width += 2;
    }
    width += UnicodeWidthStr::width(path.label());
    // "  " gap between label and description
    width += 2;
    width += UnicodeWidthStr::width(path.description());
    if let Some(count) = finding_count {
        width += UnicodeWidthStr::width(picker_count_suffix(count).as_str());
    }
    if done {
        // "  (redo)"
        width += 8;
    }
    u16::try_from(width).unwrap_or(u16::MAX)
}

/// Number of inner rows the path-select list needs at `inner_width`,
/// accounting for word-wrapped lines whose content is longer than the
/// available column count. Each path always claims at least one row.
fn path_select_inner_rows(state: &TutorialState, inner_width: u16) -> u16 {
    if inner_width == 0 {
        return u16::try_from(state.paths.len()).unwrap_or(u16::MAX);
    }
    let mut rows: u32 = 0;
    for path in &state.paths {
        let done = state.completed_paths.contains(path);
        let width = path_line_width(*path, done, state.picker_finding_count(*path));
        let wraps = width.saturating_sub(1) / inner_width + 1;
        rows = rows.saturating_add(u32::from(wraps));
    }
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// Outer box height for the path-select list, including borders and
/// padding. Clamped to `area.height` so the box never overflows the
/// available space — when the terminal is too short to fit every
/// wrapped path, the inner Paragraph clips internally rather than the
/// last paths silently disappearing because the box was sized for
/// unwrapped rows.
fn path_select_box_height(state: &TutorialState, area: Rect) -> u16 {
    // Block borders eat 2 columns and 2 rows; the explicit `Padding::new(1, 1, 1, 1)`
    // adds 1 row top + 1 row bottom on top of that, leaving 4 rows of chrome.
    const CHROME_ROWS: u16 = 4;
    let inner_width = area.width.saturating_sub(4); // 2 borders + 2 padding cols
    let inner_rows = path_select_inner_rows(state, inner_width).max(1);
    inner_rows.saturating_add(CHROME_ROWS).min(area.height)
}

fn render_path_select(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    // Box height accounts for line-wrapping: when the terminal is
    // narrow enough that path descriptions wrap, each path can occupy
    // 2+ rows. Sizing the box to `paths.len() + 4` (one row per path)
    // clipped the last entries at IDE-side-panel widths.
    let box_height = path_select_box_height(state, area);
    let chunks = Layout::vertical([Constraint::Length(box_height), Constraint::Min(0)]).split(area);
    let box_area = chunks[0];
    let below = chunks[1];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent()))
        .padding(Padding::new(1, 1, 1, 1))
        .title(" Choose a Learning Path ");
    let inner = block.inner(box_area);
    frame.render_widget(block, box_area);

    let items: Vec<Line> = state
        .paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let selected = i == state.path_selected;
            let done = state.completed_paths.contains(path);
            let indicator = if selected { ">> " } else { "   " };
            let name_style = if selected {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg())
            };

            let mut spans = Vec::with_capacity(if done { 5 } else { 3 });
            spans.push(Span::styled(indicator, name_style));
            if done {
                spans.push(Span::styled(
                    "\u{2713} ",
                    Style::default().fg(theme.success()),
                ));
            }
            spans.push(Span::styled(path.label(), name_style));
            spans.push(Span::styled(
                format!("  {}", path.description()),
                Style::default().fg(theme.muted()),
            ));
            // WOW-003: show what this path is worth on the user's own repo.
            // Absent for no-scan, zero-finding, and showcase results.
            if let Some(count) = state.picker_finding_count(*path) {
                spans.push(Span::styled(
                    picker_count_suffix(count),
                    Style::default().fg(theme.accent()),
                ));
            }
            if done {
                spans.push(Span::styled("  (redo)", Style::default().fg(theme.muted())));
            }

            Line::from(spans)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(items)).wrap(Wrap { trim: false }),
        inner,
    );

    // Helpful hint beneath the list to give the negative space purpose.
    // Skip it when no paths are registered — promising a 5-minute tutorial
    // while the picker is empty is worse than showing nothing.
    if below.height >= 2 && !state.paths.is_empty() {
        let hint_chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(below);
        let hint = Paragraph::new(Line::from(Span::styled(
            "Start with scan -> checks -> findings -> gate, then pick the area you want to learn next.",
            Style::default().fg(theme.muted()),
        )));
        frame.render_widget(hint, hint_chunks[1]);
    }
}

fn render_step_progress(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    let path_label = state.chosen_path.map_or("Tutorial", TutorialPath::label);

    let total = state.steps.len();
    let current_human = state.current_step.saturating_add(1).min(total.max(1));

    let (g_complete, g_current, g_pending) = progress_glyphs();
    let spans: Vec<Span> = state
        .steps
        .iter()
        .enumerate()
        .flat_map(|(i, _step)| {
            let (marker, style) = match i.cmp(&state.current_step) {
                std::cmp::Ordering::Less => (g_complete, Style::default().fg(theme.success())),
                std::cmp::Ordering::Equal => (
                    g_current,
                    Style::default()
                        .fg(theme.accent())
                        .add_modifier(Modifier::BOLD),
                ),
                std::cmp::Ordering::Greater => (g_pending, Style::default().fg(theme.muted())),
            };
            let separator = if i + 1 < total { "  " } else { "" };
            vec![
                Span::styled(marker, style),
                Span::styled(separator, Style::default().fg(theme.muted())),
            ]
        })
        .collect();

    let header = Line::from(vec![
        Span::styled(
            path_label,
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  \u{00b7}  ", Style::default().fg(theme.muted())),
        Span::styled(
            format!("Step {current_human} of {total}"),
            Style::default().fg(theme.fg()),
        ),
    ]);

    let lines = vec![header, Line::from(spans)];

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn render_step_content(
    frame: &mut Frame,
    area: Rect,
    state: &TutorialState,
    theme: &EddaCraftTheme,
) {
    // A bordered block with 1-cell horizontal padding consumes 4 columns and
    // 2 rows before any content fits, so anything smaller than a 4x3 area
    // leaves a zero-area `inner` that causes `Paragraph` with wrapping to
    // divide by zero. Bail out rather than render garbage.
    if area.width < 4 || area.height < 3 {
        return;
    }

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
        .padding(Padding::horizontal(1))
        .title(format!(" {} ", fit_block_title(&step.title, area.width)));
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

    // WOW-001: command steps surface the exact command and its declared
    // effect before Enter is pressed, so running-for-real is never a
    // surprise. Static mode keeps the plain walkthrough — Enter doesn't run
    // anything there, so a prompt-styled bar would over-promise.
    // WOW-002: while the reveal is in flight the bar becomes the prompt line
    // the command is being typed into.
    if let Some(cmd) = &step.command
        && !state.static_mode
    {
        lines.push(Line::default());
        if let Some(reveal) = &state.reveal {
            lines.push(reveal_bar_line(reveal, theme));
        } else {
            lines.push(command_bar_line(cmd, step.effect, theme));
        }
    }

    // Show a watching hint when the step has a watch_path and isn't in static mode.
    if step.watch_path.is_some() && !state.static_mode {
        let (_, g_current, _) = progress_glyphs();
        lines.push(Line::from(Span::styled(
            format!("{g_current} Watching for file changes\u{2026}"),
            Style::default()
                .fg(theme.muted())
                .add_modifier(Modifier::ITALIC),
        )));
    }

    if let Some(output) = &step.output {
        push_output_lines(&mut lines, output, theme);
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

/// Append a command's captured status/stdout/stderr to the step content.
fn push_output_lines(
    lines: &mut Vec<Line<'_>>,
    output: &super::CommandOutput,
    theme: &EddaCraftTheme,
) {
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

/// WOW-001: the prompt-styled command bar for a command step. Renders the
/// command visually apart from the prose plus a badge naming its declared
/// effect, so the user knows before pressing Enter whether the step writes
/// to their repo.
fn command_bar_line<'a>(
    command: &'a str,
    effect: Option<super::CommandEffect>,
    theme: &EddaCraftTheme,
) -> Line<'a> {
    let mut spans = vec![
        Span::styled(
            "$ ",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            command,
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        ),
    ];
    match effect {
        Some(super::CommandEffect::MutatesRepo) => spans.push(Span::styled(
            "  [writes to your repo]",
            Style::default()
                .fg(theme.warning())
                .add_modifier(Modifier::BOLD),
        )),
        Some(super::CommandEffect::ReadOnly) => spans.push(Span::styled(
            "  [read-only]",
            Style::default().fg(theme.success()),
        )),
        None => {}
    }
    Line::from(spans)
}

/// WOW-002: the prompt line while a command reveal is in flight — the
/// revealed prefix plus a block cursor, reading as anvil typing the command
/// into the terminal. Deterministic: content depends only on tick count.
fn reveal_bar_line<'a>(reveal: &'a super::CommandReveal, theme: &EddaCraftTheme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            "$ ",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            reveal.visible(),
            Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "\u{258c}",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn path_is_done(state: &TutorialState, path: super::TutorialPath) -> bool {
    state.completed_paths.contains(&path) || state.chosen_path == Some(path)
}

fn build_paths_progress<'a>(state: &'a TutorialState, theme: &'a EddaCraftTheme) -> Vec<Span<'a>> {
    let (g_complete, _, g_pending) = progress_glyphs();
    let total = state.paths.len();
    state
        .paths
        .iter()
        .enumerate()
        .flat_map(|(i, p)| {
            let done = path_is_done(state, *p);
            let marker = if done { g_complete } else { g_pending };
            let style = if done {
                Style::default().fg(theme.success())
            } else {
                Style::default().fg(theme.muted())
            };
            let sep = if i + 1 < total { "  " } else { "" };
            vec![
                Span::styled(marker, style),
                Span::styled(sep, Style::default().fg(theme.muted())),
            ]
        })
        .collect()
}

/// WOW-004: one honest sentence about the re-scan result. The copy names
/// the re-scan (it really ran) and never claims anvil fixed anything.
fn completion_delta_line(delta: super::FindingsDelta, theme: &EddaCraftTheme) -> Line<'static> {
    let super::FindingsDelta { before, after } = delta;
    let (msg, color) = match after.cmp(&before) {
        std::cmp::Ordering::Less => (
            format!(
                "Re-scanned your repo: {before} \u{2192} {after} findings in this domain \u{2014} {} fewer than when you started.",
                before - after
            ),
            theme.success(),
        ),
        std::cmp::Ordering::Equal => (
            format!(
                "Re-scanned your repo: {before} findings in this domain \u{2014} same as when you started."
            ),
            theme.muted(),
        ),
        std::cmp::Ordering::Greater => (
            format!(
                "Re-scanned your repo: {before} \u{2192} {after} findings in this domain \u{2014} {} more than when you started.",
                after - before
            ),
            theme.warning(),
        ),
    };
    Line::from(Span::styled(msg, Style::default().fg(color)))
}

fn render_complete(frame: &mut Frame, area: Rect, state: &TutorialState, theme: &EddaCraftTheme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.success()))
        .padding(Padding::new(2, 2, 1, 1))
        .title(" Well Done ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let path_label = state
        .chosen_path
        .map_or("the tutorial", TutorialPath::label);
    let total = state.steps.len();

    let all_paths = state.paths.len();
    let completed_count = state
        .paths
        .iter()
        .filter(|p| path_is_done(state, **p))
        .count();

    let progress_spans = build_paths_progress(state, theme);

    let next_path = state
        .paths
        .iter()
        .find(|p| !path_is_done(state, **p))
        .copied();

    let mut lines = vec![
        Line::from(Span::styled(
            "\u{2713}  Great work!",
            Style::default()
                .fg(theme.success())
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(Span::styled(
            format!("You finished all {total} steps of the {path_label} tutorial."),
            Style::default().fg(theme.fg()),
        )),
        Line::default(),
    ];

    // WOW-004: what the walk changed in the user's repo, from a read-only
    // re-scan against the session's opening scan. Absent without scan data.
    if let Some(delta) = state.completion_delta {
        lines.push(completion_delta_line(delta, theme));
        lines.push(Line::default());
    }

    lines.extend([
        Line::from(vec![Span::styled(
            format!("Progress: {completed_count} of {all_paths} paths  "),
            Style::default().fg(theme.muted()),
        )]),
        Line::from(progress_spans),
        Line::default(),
    ]);

    if let Some(next) = next_path {
        lines.push(Line::from(vec![
            Span::styled("Up next: ", Style::default().fg(theme.muted())),
            Span::styled(
                next.label(),
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            next.description(),
            Style::default().fg(theme.fg()),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "You've completed every tutorial path. Nice.",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::default());
    let enter_label = if next_path.is_some() {
        "choose another path   "
    } else {
        "back to paths   "
    };
    lines.push(Line::from(vec![
        Span::styled(
            "[enter] ",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(enter_label, Style::default().fg(theme.fg())),
        Span::styled(
            "[q] ",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("quit", Style::default().fg(theme.fg())),
    ]));

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

use super::TutorialPath;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn progress_glyphs_default_is_unicode() {
        temp_env::with_var_unset("ANVIL_ASCII", || {
            let (a, b, c) = progress_glyphs();
            assert_eq!((a, b, c), ("\u{25cf}", "\u{25c9}", "\u{25cb}"));
        });
    }

    #[test]
    fn progress_glyphs_env_var_forces_ascii() {
        temp_env::with_var("ANVIL_ASCII", Some("1"), || {
            let (a, b, c) = progress_glyphs();
            assert_eq!((a, b, c), ("#", ">", "-"));
            for g in [a, b, c] {
                assert_eq!(
                    UnicodeWidthStr::width(g),
                    1,
                    "ASCII fallback glyphs must be single-cell: {g}"
                );
            }
        });
    }

    #[test]
    fn progress_glyphs_empty_env_uses_unicode() {
        temp_env::with_var("ANVIL_ASCII", Some(""), || {
            let (a, b, c) = progress_glyphs();
            assert_eq!((a, b, c), ("\u{25cf}", "\u{25c9}", "\u{25cb}"));
        });
    }

    #[test]
    fn progress_glyphs_zero_env_uses_unicode() {
        temp_env::with_var("ANVIL_ASCII", Some("0"), || {
            let (a, b, c) = progress_glyphs();
            assert_eq!((a, b, c), ("\u{25cf}", "\u{25c9}", "\u{25cb}"));
        });
    }

    #[test]
    fn fit_block_title_passes_through_short_title() {
        let out = fit_block_title("Run gate", 80);
        assert_eq!(out, "Run gate");
    }

    #[test]
    fn fit_block_title_truncates_with_ellipsis() {
        let out = fit_block_title(
            "a-very-long-unbreakable-step-title-that-would-overflow-the-border",
            20,
        );
        // 20 cells, minus 4 for border+padding = 16 budget. Last char is ellipsis.
        assert!(out.ends_with('\u{2026}'));
        assert!(UnicodeWidthStr::width(out.as_str()) <= 16);
    }

    #[test]
    fn fit_block_title_handles_tiny_areas() {
        // Width 4 or less leaves no budget for content.
        assert_eq!(fit_block_title("anything", 4), "");
        // Width 5 → 1 char budget, must be just the ellipsis.
        assert_eq!(fit_block_title("anything", 5), "\u{2026}");
    }

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

    /// Guards against arithmetic underflow, clipped borders, and zero-width
    /// wrap targets flagged by the council review at very small terminal
    /// sizes. Must not panic on any of the three tutorial phases.
    #[test]
    fn renders_without_panic_tiny_terminal() {
        let cases: &[(u16, u16)] = &[(20, 5), (5, 5), (40, 10)];
        for &(w, h) in cases {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let theme = EddaCraftTheme;

            let mut path_select = TutorialState::new();
            terminal
                .draw(|frame| render(frame, frame.area(), &path_select, &theme))
                .unwrap_or_else(|e| panic!("path_select panicked at {w}x{h}: {e}"));

            path_select.load_steps(TutorialPath::Policy);
            terminal
                .draw(|frame| render(frame, frame.area(), &path_select, &theme))
                .unwrap_or_else(|e| panic!("running panicked at {w}x{h}: {e}"));

            let mut complete = TutorialState::new();
            complete.load_steps(TutorialPath::Drift);
            for step in &mut complete.steps {
                step.completed = true;
            }
            complete.phase = TutorialPhase::Complete;
            terminal
                .draw(|frame| render(frame, frame.area(), &complete, &theme))
                .unwrap_or_else(|e| panic!("complete panicked at {w}x{h}: {e}"));
        }
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

    /// Lock in the narrow-terminal layout for the path-select phase at 40x10:
    /// panic-free isn't enough — the menu copy has to stay readable and the
    /// bottom border must not be clipped. Regressions in `box_height` clamping
    /// or the shell gutter logic will shift this snapshot visibly.
    #[test]
    fn snapshot_path_select_narrow_40x10() {
        let backend = TestBackend::new(40, 10);
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

    /// At 20x10 we're past the point where the full menu copy fits, but the
    /// frame must still render without border clipping or overlap. Guards
    /// against layout arithmetic underflow at the small-terminal floor.
    #[test]
    fn snapshot_path_select_tiny_20x10() {
        let backend = TestBackend::new(20, 10);
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

    /// Running phase at 40x10 — locks in the narrow-width content layout
    /// (step progress row, step block, wrapped body) end-to-end through the
    /// shell + render path so regressions in the vertical chunk math or the
    /// step content block surface visibly in the snapshot diff.
    #[test]
    fn snapshot_running_phase_narrow_40x10() {
        let backend = TestBackend::new(40, 10);
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

    /// Running phase at 20x10 is the extreme lower bound — progress dots and
    /// block chrome should still render, even if content wraps aggressively.
    #[test]
    fn snapshot_running_phase_tiny_20x10() {
        let backend = TestBackend::new(20, 10);
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

    /// Real (non-showcase) scan results with findings across domains, for
    /// the WOW-003 picker personalization snapshots.
    fn picker_scan_results() -> super::super::discovery::ScanResults {
        use super::super::discovery::{Finding, FindingSeverity, FindingSource, ScanResults};
        ScanResults {
            findings: vec![
                Finding {
                    file: "src/main.rs".to_string(),
                    line: Some(10),
                    severity: FindingSeverity::Error,
                    source: FindingSource::AntiPattern,
                    title: "anti-pattern".to_string(),
                    message: "test".to_string(),
                    suggestion: "fix".to_string(),
                    warning_id: Some("AP-003".to_string()),
                },
                Finding {
                    file: "src/lib.rs".to_string(),
                    line: Some(20),
                    severity: FindingSeverity::Warning,
                    source: FindingSource::Architecture,
                    title: "boundary".to_string(),
                    message: "test".to_string(),
                    suggestion: "fix".to_string(),
                    warning_id: None,
                },
                Finding {
                    file: "src/auth.rs".to_string(),
                    line: Some(3),
                    severity: FindingSeverity::Error,
                    source: FindingSource::Secret,
                    title: "secret".to_string(),
                    message: "test".to_string(),
                    suggestion: "fix".to_string(),
                    warning_id: None,
                },
            ],
            files_scanned: 120,
            duration_ms: 250,
            truncated: false,
            files_skipped_by_ignore: 0,
            is_showcase: false,
        }
    }

    /// WOW-003: with real scan findings present, each picker row carries its
    /// per-domain count so the choice is grounded in the user's own repo.
    #[test]
    fn snapshot_path_select_with_finding_counts() {
        let backend = TestBackend::new(100, 22);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.set_scan_results(picker_scan_results());
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

    /// WOW-003: a clean repo (scan ran, zero findings) falls back to the
    /// exact pre-personalization picker copy — no counts, no new noise.
    #[test]
    fn snapshot_path_select_clean_repo() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.set_scan_results(super::super::discovery::ScanResults {
            findings: Vec::new(),
            files_scanned: 42,
            duration_ms: 10,
            truncated: false,
            files_skipped_by_ignore: 0,
            is_showcase: false,
        });
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

    /// Build a Running state with one command step of the given effect and
    /// short copy, so the WOW-001 command bar and badge stay inside the
    /// 80x20 snapshot viewport (real path descriptions wrap past it).
    fn command_step_state(effect: super::super::CommandEffect) -> TutorialState {
        let mut state = TutorialState::new();
        state.steps = vec![super::super::TutorialStep {
            title: "Run the verifier".to_string(),
            description: "A short step that runs a real command.".to_string(),
            instruction: "Run: anvil start --verify".to_string(),
            command: Some("anvil start --verify".to_string()),
            effect: Some(effect),
            ..super::super::TutorialStep::default()
        }];
        state.phase = TutorialPhase::Running;
        state.chosen_path = Some(TutorialPath::ProtectionLoop);
        state
    }

    /// WOW-001: a read-only command step must show the prompt-styled command
    /// bar with the `[read-only]` badge and the command-step footer help
    /// BEFORE the command has executed.
    #[test]
    fn snapshot_command_step_read_only_badge() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = command_step_state(super::super::CommandEffect::ReadOnly);
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

    /// WOW-001: a mutating command step must show the `[writes to your repo]`
    /// badge so a repo write is never a surprise.
    #[test]
    fn snapshot_command_step_mutating_badge() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = command_step_state(super::super::CommandEffect::MutatesRepo);
        state.steps[0].title = "Create Policy Directory".to_string();
        state.steps[0].instruction = "Run: mkdir -p .anvil/policies".to_string();
        state.steps[0].command = Some("mkdir -p .anvil/policies".to_string());
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

    /// WOW-002: mid-reveal, the prompt line shows the deterministic prefix
    /// for the tick count (2 ticks × 3 chars = "anvil ") plus the block
    /// cursor. Pins that the reveal is snapshot-stable — no wall-clock.
    #[test]
    fn snapshot_command_step_mid_reveal() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = command_step_state(super::super::CommandEffect::ReadOnly);
        state.handle_key(eddacraft_tui::keyboard::Action::Select);
        state.reveal_tick();
        state.reveal_tick();
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

    /// Multiple paths completed — the progress indicator should show two
    /// filled dots, the "Up next" section should suggest an unfinished path
    /// (not the all-paths-complete copy), and the completed count should
    /// reflect both finishes. Catches regressions in `build_paths_progress`
    /// and `path_is_done` for the multi-completion case.
    #[test]
    fn snapshot_complete_phase_multiple_paths() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Drift);
        for step in &mut state.steps {
            step.completed = true;
        }
        // Policy was finished in a prior session; Drift is finishing now.
        state.completed_paths = vec![TutorialPath::Policy];
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

    /// Every path finished — the "Up next" section should switch to the
    /// all-paths-complete celebration copy and the progress row should be
    /// fully filled.
    #[test]
    fn snapshot_complete_phase_all_paths() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Drift);
        for step in &mut state.steps {
            step.completed = true;
        }
        // All other paths already done before this final one.
        state.completed_paths = state
            .paths
            .iter()
            .copied()
            .filter(|p| *p != TutorialPath::Drift)
            .collect();
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

    /// Build a completed-tutorial state with a WOW-004 findings delta set.
    fn complete_state_with_delta(before: usize, after: usize) -> TutorialState {
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Policy);
        for step in &mut state.steps {
            step.completed = true;
        }
        state.phase = TutorialPhase::Complete;
        state.completion_delta = Some(super::super::FindingsDelta { before, after });
        state
    }

    fn snapshot_complete_with_delta(name: &str, state: &TutorialState) {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    Surface::surface_name(state),
                    Surface::help_text(state),
                    &theme,
                );
                render(frame, content, state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(name, crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    /// WOW-004: fewer findings after the walk — the improved delta line.
    #[test]
    fn snapshot_complete_phase_delta_improved() {
        snapshot_complete_with_delta(
            "snapshot_complete_phase_delta_improved",
            &complete_state_with_delta(5, 3),
        );
    }

    /// WOW-004: same count — honest "unchanged" copy, no false win.
    #[test]
    fn snapshot_complete_phase_delta_unchanged() {
        snapshot_complete_with_delta(
            "snapshot_complete_phase_delta_unchanged",
            &complete_state_with_delta(4, 4),
        );
    }

    /// WOW-004: more findings than at the start — reported, not hidden.
    #[test]
    fn snapshot_complete_phase_delta_regressed() {
        snapshot_complete_with_delta(
            "snapshot_complete_phase_delta_regressed",
            &complete_state_with_delta(2, 4),
        );
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

    #[test]
    fn renders_static_mode_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Policy);
        state.enable_static_mode();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_running_static_mode() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Policy);
        state.enable_static_mode();
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
    fn renders_resuming_notice_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TutorialState::new();
        state.set_completed_paths(vec![TutorialPath::Architecture]);
        state.resume_path(
            TutorialPath::Policy,
            1,
            &[true, false, false, false, false, false],
        );
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    // v0.5.0 — at narrow widths (IDE side panes, dual-column layouts)
    // the path descriptions wrap to a second row, but the path-select
    // box height was computed as `paths.len() + 4`, so the last paths'
    // wrapped rows fell off the bottom of the box.

    #[test]
    fn path_select_inner_rows_unwrapped_at_full_width() {
        let state = TutorialState::new();
        // Wide enough that every label+description fits on one row.
        let rows = path_select_inner_rows(&state, 200);
        assert_eq!(
            rows,
            u16::try_from(state.paths.len()).unwrap_or(u16::MAX),
            "unwrapped layout: one row per path"
        );
    }

    #[test]
    fn path_select_inner_rows_doubles_when_wrapped() {
        let state = TutorialState::new();
        // 50 columns is narrower than any label+description combined,
        // so every path wraps to at least two rows.
        let rows = path_select_inner_rows(&state, 50);
        assert!(
            rows >= (u16::try_from(state.paths.len()).unwrap_or(u16::MAX)) * 2,
            "wrapped layout must reserve >= 2 rows per path; got {rows} for {} paths",
            state.paths.len()
        );
    }

    #[test]
    fn path_select_box_fits_all_paths_in_narrow_terminal() {
        // Reproduces the IDE-side-pane case: 60-col wide terminal,
        // ample height. All four paths (Policy / Architecture / Drift /
        // CI) must remain visible inside the box.
        let backend = TestBackend::new(60, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = TutorialState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered: String = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        for path in &state.paths {
            assert!(
                rendered.contains(path.label()),
                "path label {:?} missing from narrow render — likely clipped by box height",
                path.label()
            );
        }
    }
}
