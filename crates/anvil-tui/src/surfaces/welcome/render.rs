use eddacraft_tui::prelude::*;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use super::{QuickStartOption, WelcomeState};

// anvil brandmark — faithful to logos/svg/anvil-brandmark-white.svg
// Two L-shaped corner brackets framing a central anvil body
// (two horizontal bars connected by a short vertical column).
const LOGO_LINES: &[&str] = &[
    "████         ████",
    "██             ██",
    "██  █████████  ██",
    "██     ███     ██   a n v i l",
    "██  █████████  ██",
    "██             ██",
    "████         ████",
];

/// Rows the brandmark occupies — the layout maths below must track the art.
const LOGO_HEIGHT: usize = LOGO_LINES.len();
#[allow(clippy::cast_possible_truncation)]
const LOGO_HEIGHT_U16: u16 = LOGO_HEIGHT as u16;

const TAGLINE: &str = "Structural governance for AI-assisted development";

/// Muted one-line hint shown in compact mode to explain that resizing the
/// terminal restores the per-item descriptions dropped to save space (CIB-179).
const COMPACT_HINT: &str = "resize for descriptions";

/// Muted one-line explanation of the first-run marker (CIB-246). A returning
/// user lands here instead of the first-run walk they remember, and nothing on
/// screen said why; this names the marker behaviour and the way back.
const FIRST_RUN_NOTE: &str =
    "First run is done, so anvil opens this hub. Restart onboarding replays it.";

/// Width the note needs before it is worth rendering: the note itself plus the
/// left pad. Below this it would be clipped mid-sentence, which reads worse
/// than omitting it.
#[allow(clippy::cast_possible_truncation)]
const FIRST_RUN_NOTE_MIN_WIDTH: u16 = FIRST_RUN_NOTE.len() as u16 + PAD_WIDTH;

/// Left padding for content within the welcome screen.
const PAD: &str = "    ";
const PAD_WIDTH: u16 = 4;

pub fn render(frame: &mut Frame, area: Rect, state: &WelcomeState, theme: &EddaCraftTheme) {
    let menu_item_count = QuickStartOption::ALL.len();

    // Select renders each option as a single shared-widget row. Full mode keeps
    // descriptions inline; compact mode drops them and surfaces the resize hint.
    let full_menu_height = menu_item_count;
    let status_height = if state.status_message.is_some() { 2 } else { 0 };
    let full_content_height = LOGO_HEIGHT + 1 + 1 + 2 + full_menu_height + status_height;
    #[allow(clippy::cast_possible_truncation)]
    let compact = (full_content_height as u16) > area.height || area.width < 72;

    let menu_height = full_menu_height;

    // CIB-246: the first-run note is carved out of the tagline spacer rather
    // than added to the layout, so total content height is unchanged and the
    // compact/logo-integrity maths below stays exactly as CIB-179 left it.
    let show_note = !compact && area.width >= FIRST_RUN_NOTE_MIN_WIDTH;
    let note_h = u16::from(show_note);

    // In compact mode, surface a one-line resize hint — but only when the
    // terminal has genuine spare height for it: top padding(1) + logo +
    // blank(1) + the full compact menu + optional status + hint(1). Showing it
    // under contention is unsafe — ratatui holds the `Min(menu_h)` menu at full
    // size and lets the fixed-`Length` logo absorb the shortfall, silently squeezing a
    // row (or more) out of the brandmark. That is the exact defect CIB-179
    // exists to prevent, so the hint must never be traded for logo integrity.
    #[allow(clippy::cast_possible_truncation)]
    let hint_min_height = (1 + LOGO_HEIGHT + 1 + menu_height + status_height + 1) as u16;
    let show_hint = compact && area.height >= hint_min_height;
    let hint_h: u16 = u16::from(show_hint);
    #[allow(clippy::cast_possible_truncation)]
    let status_h = status_height as u16;

    // In compact mode: logo + blank(1) + menu(N) + optional hint(1)
    // In full mode: logo + blank(1) + tagline(1) + gap(2) + menu(N) — one row
    // per option via the shared `Select` widget. The gap below the tagline is
    // a fixed 2 rows whether or not the first-run note shows: the note takes
    // `note_h` of it and the spacer keeps the remaining `2 - note_h`, so this
    // total is independent of `show_note` (CIB-246).
    let content_height = if compact {
        LOGO_HEIGHT + 1 + menu_height + usize::from(show_hint) + status_height
    } else {
        LOGO_HEIGHT + 1 + 1 + 2 + menu_height + status_height
    };

    // Centre vertically — at least 1 row gap from header
    #[allow(clippy::cast_possible_truncation)]
    let content_h = content_height as u16;
    let top_pad = (area.height.saturating_sub(content_h) / 2).max(1);
    #[allow(clippy::cast_possible_truncation)]
    let menu_h = menu_height as u16;

    let chunks = if compact {
        Layout::vertical([
            Constraint::Length(top_pad),         // Top padding
            Constraint::Length(LOGO_HEIGHT_U16), // Logo
            Constraint::Length(1),               // Blank
            Constraint::Min(menu_h),             // Menu items (compact)
            Constraint::Length(hint_h),          // Resize hint (0 when it doesn't fit)
            Constraint::Length(status_h),        // Optional status message
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(top_pad),         // Top padding
            Constraint::Length(LOGO_HEIGHT_U16), // Logo
            Constraint::Length(1),               // Blank
            Constraint::Length(1),               // Tagline
            Constraint::Length(note_h),          // First-run note (0 when too narrow)
            Constraint::Length(2 - note_h),      // Spacer (absorbs the note's row)
            Constraint::Min(menu_h),             // Menu items (flexible — absorbs overflow)
            Constraint::Length(status_h),        // Optional status message
        ])
        .split(area)
    };

    // Logo — block art in EMBER, "a n v i l" text in FG
    let logo_lines: Vec<Line> = LOGO_LINES
        .iter()
        .map(|line| {
            if let Some((before, _)) = line.split_once("a n v i l") {
                Line::from(vec![
                    Span::styled(PAD, Style::default()),
                    Span::styled(before, Style::default().fg(theme.accent())),
                    Span::styled(
                        "a n v i l",
                        Style::default().fg(theme.fg()).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(PAD, Style::default()),
                    Span::styled(*line, Style::default().fg(theme.accent())),
                ])
            }
        })
        .collect();
    let logo = Paragraph::new(Text::from(logo_lines));
    frame.render_widget(logo, chunks[1]);

    // Tagline and first-run note (both hidden in compact mode)
    let menu_chunk_idx = if compact {
        3
    } else {
        frame.render_widget(muted_line(TAGLINE, theme), chunks[3]);
        if show_note {
            frame.render_widget(muted_line(FIRST_RUN_NOTE, theme), chunks[4]);
        }
        6
    };

    render_menu(frame, chunks[menu_chunk_idx], state, theme, compact);

    // Compact-mode resize hint occupies the reserved trailing chunk.
    if show_hint {
        frame.render_widget(muted_line(COMPACT_HINT, theme), chunks[4]);
    }

    if let Some(ref msg) = state.status_message {
        let status_idx = if compact { 5 } else { 7 };
        let status = Paragraph::new(Text::from(vec![
            Line::default(),
            Line::from(vec![
                Span::styled(PAD, Style::default()),
                Span::styled(msg, Style::default().fg(theme.muted())),
            ]),
        ]));
        frame.render_widget(status, chunks[status_idx]);
    }
}

/// A padded, muted one-liner — the shared shape of the tagline, the first-run
/// note, and the compact resize hint.
fn muted_line<'a>(text: &'a str, theme: &EddaCraftTheme) -> Paragraph<'a> {
    Paragraph::new(Line::from(vec![
        Span::styled(PAD, Style::default()),
        Span::styled(text, Style::default().fg(theme.muted())),
    ]))
}

fn render_menu(
    frame: &mut Frame,
    area: Rect,
    state: &WelcomeState,
    theme: &EddaCraftTheme,
    compact: bool,
) {
    let padded =
        Layout::horizontal([Constraint::Length(PAD_WIDTH), Constraint::Min(0)]).split(area)[1];
    let items = build_menu_items(compact);
    let mut select_state = SelectState {
        selected: state.selected,
        offset: 0,
    };
    frame.render_stateful_widget(Select::new(items, theme), padded, &mut select_state);
}

fn build_menu_items(compact: bool) -> Vec<SelectItem> {
    QuickStartOption::ALL
        .iter()
        .map(|opt| {
            if compact {
                SelectItem::from(opt.label())
            } else {
                SelectItem::new(opt.label(), opt.description())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn snapshot_default_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    crate::surface::Surface::surface_name(&state),
                    crate::surface::Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_status_message() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = WelcomeState::new();
        state.status_message = Some("Ready.".to_string());
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    crate::surface::Surface::surface_name(&state),
                    crate::surface::Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_second_item_selected() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = WelcomeState::new();
        state.selected = 1;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                let content = crate::shell::render_shell(
                    frame,
                    frame.area(),
                    crate::surface::Surface::surface_name(&state),
                    crate::surface::Surface::help_text(&state),
                    &theme,
                );
                render(frame, content, &state, &theme);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn menu_descriptions_share_select_rows() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let text = plain(terminal.backend().buffer());
        assert!(
            text.contains(
                "▸ Review gate decision  See whether the current findings pass your workflow gate"
            ),
            "welcome menu should render label + description through Select rows; got:\n{text}"
        );
    }

    /// Plain-text (style-stripped) render of a buffer for substring assertions.
    fn plain(buf: &ratatui::buffer::Buffer) -> String {
        let area = buf.area;
        let mut s = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn compact_shows_resize_hint() {
        // Compact (< full_content_height) but tall enough that the hint fits
        // below a full logo and full compact menu without contention.
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let text = plain(terminal.backend().buffer());
        assert!(
            text.contains(COMPACT_HINT),
            "compact mode should surface the resize hint; got:\n{text}"
        );
        assert_eq!(
            text.lines().filter(|l| l.contains('\u{2588}')).count(),
            LOGO_LINES.len(),
            "logo must stay intact when the hint is shown; got:\n{text}"
        );
    }

    #[test]
    fn boundary_compact_withholds_hint_keeps_logo_intact() {
        // Height 16 is the boundary that fits a full logo + full compact menu
        // exactly. Adding the hint here would force the fixed-height logo to
        // shed a row, so the hint is withheld and the brandmark stays intact
        // (the precise boundary case CIB-179 must not regress).
        let backend = TestBackend::new(40, 16);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let text = plain(terminal.backend().buffer());
        assert!(
            !text.contains(COMPACT_HINT),
            "hint must be withheld when it would squeeze the logo; got:\n{text}"
        );
        assert_eq!(
            text.lines().filter(|l| l.contains('\u{2588}')).count(),
            LOGO_LINES.len(),
            "logo must stay intact at the boundary compact size; got:\n{text}"
        );
    }

    #[test]
    fn full_mode_omits_resize_hint() {
        // Tall enough for the welcome surface to enter full mode.
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let text = plain(terminal.backend().buffer());
        assert!(
            !text.contains(COMPACT_HINT),
            "full mode should not show the resize hint; got:\n{text}"
        );
    }

    /// CIB-246: a returning user lands on the hub instead of the first-run
    /// walk they remember. The hub has to say why, and say how to get back,
    /// without costing the brandmark a row.
    #[test]
    fn full_mode_names_first_run_marker_behaviour() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let text = plain(terminal.backend().buffer());
        assert!(
            text.contains(FIRST_RUN_NOTE),
            "hub should explain the first-run marker; got:\n{text}"
        );
        assert_eq!(
            logo_rows(&text),
            LOGO_LINES.len(),
            "note must not squeeze the logo; got:\n{text}"
        );
    }

    /// Compact mode already drops the per-item descriptions, so the note goes
    /// with them rather than competing with the menu for rows.
    #[test]
    fn compact_mode_omits_first_run_note() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let text = plain(terminal.backend().buffer());
        assert!(
            !text.contains(FIRST_RUN_NOTE),
            "compact mode should omit the first-run note; got:\n{text}"
        );
    }

    /// The note is carved out of the tagline spacer, so it costs no height —
    /// but it must still be withheld at widths where it would render clipped
    /// mid-sentence, which reads worse than saying nothing.
    #[test]
    fn first_run_note_withheld_when_it_would_clip() {
        let theme = EddaCraftTheme;
        for width in 60u16..=100 {
            let backend = TestBackend::new(width, 30);
            let mut terminal = Terminal::new(backend).unwrap();
            let state = WelcomeState::new();
            terminal
                .draw(|frame| render(frame, frame.area(), &state, &theme))
                .unwrap();

            let text = plain(terminal.backend().buffer());
            if text.contains(FIRST_RUN_NOTE) {
                assert!(
                    width >= FIRST_RUN_NOTE_MIN_WIDTH,
                    "at width {width}: note rendered below its minimum width; got:\n{text}"
                );
            }
            assert_eq!(
                logo_rows(&text),
                LOGO_LINES.len(),
                "at width {width}: logo must stay intact; got:\n{text}"
            );
        }
    }

    #[test]
    fn snapshot_compact_hint() {
        // Compact with enough height that the hint fits below an intact logo.
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn renders_in_small_area() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = WelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    /// Count rows that contain the logo block glyph — the effective rendered
    /// height of the brandmark.
    fn logo_rows(text: &str) -> usize {
        text.lines().filter(|l| l.contains('\u{2588}')).count()
    }

    /// The compact resize hint must never be shown at a size where doing so
    /// squeezes a row out of the fixed-height logo. Under contention ratatui
    /// holds the `Min(menu)` menu at full size and shrinks the `Length(7)`
    /// logo, so an over-eager hint silently degrades the brandmark — the exact
    /// failure mode CIB-179 exists to prevent (regression guard).
    #[test]
    fn compact_hint_never_squeezes_logo() {
        let theme = EddaCraftTheme;
        for height in 8u16..=32 {
            let backend = TestBackend::new(40, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let state = WelcomeState::new();
            terminal
                .draw(|frame| render(frame, frame.area(), &state, &theme))
                .unwrap();

            let text = plain(terminal.backend().buffer());
            let rows = logo_rows(&text);
            let hint_shown = text.contains(COMPACT_HINT);
            assert!(
                !(hint_shown && rows < LOGO_LINES.len()),
                "at height {height}: resize hint shown but logo squeezed to \
                 {rows} rows (expected {}); got:\n{text}",
                LOGO_LINES.len(),
            );
        }
    }
}
