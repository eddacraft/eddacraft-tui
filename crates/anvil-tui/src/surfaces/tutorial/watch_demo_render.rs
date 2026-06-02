use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::surfaces::watch::WatchState;

use super::watch_demo::{OverlayPhase, WatchDemoState};

/// Render the watch demo: the real watch dashboard with a guided overlay.
pub fn render(frame: &mut Frame, area: Rect, state: &WatchDemoState, theme: &EddaCraftTheme) {
    // Render the watch dashboard in the full area using a temporary WatchState.
    let watch_state = WatchState::new(state.data.clone());
    crate::surfaces::watch::render::render(frame, area, &watch_state, theme);

    // Render the overlay while it has any visible reveal — this lets the
    // intro animate in from 0 and the dismiss animate out to 0 instead of
    // popping in/out the moment the phase changes.
    if state.overlay_reveal() > f64::EPSILON {
        render_overlay(frame, area, state, theme);
    }
}

fn render_overlay(frame: &mut Frame, area: Rect, state: &WatchDemoState, theme: &EddaCraftTheme) {
    let text = state.overlay_text();
    if text.is_empty() {
        return;
    }

    let reveal = state.overlay_reveal().clamp(0.0, 1.0);

    // Position the overlay at the bottom of the screen.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let overlay_height = ((3.0 + (2.0 * reveal)).round() as u16).min(area.height.saturating_sub(2));
    let chunks =
        Layout::vertical([Constraint::Min(0), Constraint::Length(overlay_height)]).split(area);

    let overlay_area = chunks[1];

    let (border_color, icon) = match state.overlay {
        OverlayPhase::CycleComplete => (theme.success(), "\u{2714} "),
        OverlayPhase::Hint3 => (theme.accent(), "\u{27a4} "),
        _ => (theme.muted(), "\u{2139} "),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " Watch Demo ",
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        ));

    let content = Paragraph::new(Line::from(vec![
        Span::styled(icon, Style::default().fg(border_color)),
        Span::styled(text, Style::default().fg(theme.fg())),
    ]))
    .wrap(Wrap { trim: true })
    .block(block);

    frame.render_widget(content, overlay_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surfaces::watch::{WatchData, WatchStats, WatchStatus};
    use std::collections::VecDeque;

    fn empty_data() -> WatchData {
        WatchData {
            status: WatchStatus::Idle,
            queue: VecDeque::new(),
            history: Vec::new(),
            stats: WatchStats {
                total_runs: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                files_watched: 0,
            },
            warmup: None,
            last_action: None,
            update_hint: None,
            insights_hint: None,
        }
    }

    #[test]
    fn render_does_not_panic_with_intro() {
        let state = WatchDemoState::new(empty_data());
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render(frame, frame.area(), &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn render_does_not_panic_with_dismissed_overlay() {
        let mut state = WatchDemoState::new(empty_data());
        state.overlay = OverlayPhase::Dismissed;
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render(frame, frame.area(), &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn render_does_not_panic_with_cycle_complete() {
        let mut state = WatchDemoState::new(empty_data());
        state.overlay = OverlayPhase::CycleComplete;
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| {
                render(frame, frame.area(), &state, &theme);
            })
            .unwrap();
    }
}
