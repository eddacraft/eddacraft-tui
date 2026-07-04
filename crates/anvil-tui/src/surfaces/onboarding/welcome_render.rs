use eddacraft_tui::prelude::*;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use super::welcome::{OnboardingChoice, OnboardingWelcomeState};
use crate::shell::inset_content;

// anvil brandmark — same logo used on the standard welcome screen.
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

const TAGLINE: &str = "anvil catches architecture drift at save-time";
const SUBTITLE: &str = "Let's get you set up.";

/// Muted one-line hint shown in compact mode to explain that resizing the
/// terminal restores the per-item descriptions dropped to save space (CIB-179).
const COMPACT_HINT: &str = "resize for descriptions";

/// Left padding for content.
const PAD: &str = "    ";

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &OnboardingWelcomeState,
    theme: &EddaCraftTheme,
) {
    let area = inset_content(area);
    let item_count = OnboardingChoice::ALL.len();

    // Full mode: 2 lines per item + 1 blank between = item_count * 3 - 1
    // Compact mode: 1 line per item (no descriptions, no blanks)
    let full_menu_height = item_count * 3 - 1;
    // logo + blank(1) + tagline(1) + subtitle(1) + spacer(1) + menu
    let full_content_height = LOGO_HEIGHT + 1 + 1 + 1 + 1 + full_menu_height;
    #[allow(clippy::cast_possible_truncation)]
    let compact = (full_content_height as u16) > area.height;

    let menu_height = if compact {
        item_count
    } else {
        full_menu_height
    };

    // In compact mode, surface a one-line resize hint — but only when the
    // terminal has genuine spare height for it: top padding(1) + logo +
    // blank(1) + the full compact menu + hint(1). Showing it under contention
    // is unsafe — ratatui holds the `Min(menu_h)` menu at full size and lets
    // the fixed-`Length` logo absorb the shortfall, silently squeezing a
    // row (or more) out of the brandmark. That is the exact defect CIB-179
    // exists to prevent, so the hint must never be traded for logo integrity.
    #[allow(clippy::cast_possible_truncation)]
    let hint_min_height = (1 + LOGO_HEIGHT + 1 + menu_height + 1) as u16;
    let show_hint = compact && area.height >= hint_min_height;
    let hint_h: u16 = u16::from(show_hint);

    // In compact mode: logo + blank(1) + menu + optional hint(1)
    // In full mode: logo + blank(1) + tagline(1) + subtitle(1) + spacer(1) + menu
    let content_height = if compact {
        LOGO_HEIGHT + 1 + menu_height + usize::from(show_hint)
    } else {
        LOGO_HEIGHT + 1 + 1 + 1 + 1 + menu_height
    };

    #[allow(clippy::cast_possible_truncation)]
    let content_h = content_height as u16;
    let top_pad = (area.height.saturating_sub(content_h) / 2).max(1);
    #[allow(clippy::cast_possible_truncation)]
    let menu_h = menu_height as u16;

    let chunks = if compact {
        Layout::vertical([
            Constraint::Length(top_pad),
            Constraint::Length(LOGO_HEIGHT_U16), // Logo
            Constraint::Length(1),               // Blank
            Constraint::Min(menu_h),             // Menu items
            Constraint::Length(hint_h),          // Resize hint (0 when it doesn't fit)
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(top_pad),
            Constraint::Length(LOGO_HEIGHT_U16), // Logo
            Constraint::Length(1),               // Blank
            Constraint::Length(1),               // Tagline
            Constraint::Length(1),               // Subtitle
            Constraint::Length(1),               // Spacer
            Constraint::Min(menu_h),             // Menu items
        ])
        .split(area)
    };

    // Logo — block art in accent colour, "a n v i l" label in fg+bold
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

    // Tagline + subtitle (hidden in compact mode), then menu
    let menu_chunk_idx = if compact {
        3
    } else {
        let tagline = Paragraph::new(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(TAGLINE, Style::default().fg(theme.muted())),
        ]));
        frame.render_widget(tagline, chunks[3]);

        let subtitle = Paragraph::new(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(SUBTITLE, Style::default().fg(theme.fg())),
        ]));
        frame.render_widget(subtitle, chunks[4]);

        // chunks[5] is the spacer — nothing to render
        6
    };

    let menu_lines = build_menu_lines(state, theme, compact);
    let menu = Paragraph::new(Text::from(menu_lines));
    frame.render_widget(menu, chunks[menu_chunk_idx]);

    // Compact-mode resize hint occupies the reserved trailing chunk.
    if show_hint {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(COMPACT_HINT, Style::default().fg(theme.muted())),
        ]));
        frame.render_widget(hint, chunks[4]);
    }
}

fn build_menu_lines<'a>(
    state: &'a OnboardingWelcomeState,
    theme: &'a EddaCraftTheme,
    compact: bool,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    for (i, opt) in OnboardingChoice::ALL.iter().enumerate() {
        if !compact && i > 0 {
            lines.push(Line::raw(""));
        }

        let selected = i == state.selected;
        let indicator = if selected { " \u{25b8} " } else { "   " };
        let label_style = if selected {
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg())
        };

        lines.push(Line::from(vec![
            Span::styled(PAD, Style::default()),
            Span::styled(indicator, label_style),
            Span::styled(opt.label(), label_style),
        ]));

        if !compact {
            let desc_style = Style::default().fg(theme.muted());
            lines.push(Line::from(vec![
                Span::styled(PAD, Style::default()),
                Span::styled("      ", Style::default()),
                Span::styled(opt.description(), desc_style),
            ]));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_without_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = OnboardingWelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_in_small_area() {
        let backend = ratatui::backend::TestBackend::new(40, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = OnboardingWelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
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
        // Compact but tall enough that the hint fits below a full logo and
        // full compact menu without squeezing the fixed-height logo.
        let backend = ratatui::backend::TestBackend::new(40, 16);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = OnboardingWelcomeState::new();
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
        // The smallest height where a full logo + full compact menu fit; adding
        // the hint here would shed a logo row, so it is withheld and the
        // brandmark stays intact (CIB-179 regression guard).
        let backend = ratatui::backend::TestBackend::new(40, 13);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = OnboardingWelcomeState::new();
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
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = OnboardingWelcomeState::new();
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

    #[test]
    fn snapshot_compact_hint() {
        // Compact with enough height that the hint fits below an intact logo.
        let backend = ratatui::backend::TestBackend::new(40, 16);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = OnboardingWelcomeState::new();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(&buf));
    }

    #[test]
    fn renders_second_item_selected() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = OnboardingWelcomeState::new();
        state.selected = 1;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn renders_third_item_selected() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut state = OnboardingWelcomeState::new();
        state.selected = 2;
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    /// The compact resize hint must never be shown at a size where doing so
    /// squeezes a row out of the fixed-height logo. Under contention ratatui
    /// holds the `Min(menu)` menu at full size and shrinks the `Length(7)`
    /// logo, so an over-eager hint silently degrades the brandmark — the exact
    /// failure mode CIB-179 exists to prevent (regression guard).
    #[test]
    fn compact_hint_never_squeezes_logo() {
        let theme = EddaCraftTheme;
        for height in 10u16..=32 {
            let backend = ratatui::backend::TestBackend::new(40, height);
            let mut terminal = ratatui::Terminal::new(backend).unwrap();
            let state = OnboardingWelcomeState::new();
            terminal
                .draw(|frame| render(frame, frame.area(), &state, &theme))
                .unwrap();

            let text = plain(terminal.backend().buffer());
            let rows = text.lines().filter(|l| l.contains('\u{2588}')).count();
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
