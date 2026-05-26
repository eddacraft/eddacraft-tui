use eddacraft_tui::prelude::{Container, ContainerVariant, EddaCraftTheme, Theme};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use super::DashboardPickerState;

/// Render the dashboard picker into `area`. The shell chrome (header + footer)
/// is drawn by the surface loop; this fills the content area only.
pub fn render(frame: &mut Frame, area: Rect, state: &DashboardPickerState, theme: &EddaCraftTheme) {
    // The bordered container needs room for its frame plus content; under ~24
    // columns it swallows the entry text entirely. The surface targets an 80x24
    // minimum, but rather than going blank below that we drop the border and
    // render a compact, still-legible list (mirrors plan_dashboard's narrow path).
    if area.width < 24 {
        render_compact(frame, area, state, theme);
        return;
    }

    let container = Container::new(theme)
        .title("Dashboards")
        .variant(ContainerVariant::Primary);
    let inner = container.inner(area);
    frame.render_widget(container, area);

    let body = if state.entries.is_empty() {
        Text::from(vec![
            Line::raw(""),
            Line::styled(
                "  No dashboards available yet.",
                Style::default().fg(theme.muted()),
            ),
        ])
    } else {
        Text::from(entry_lines(state, theme))
    };

    frame.render_widget(Paragraph::new(body), inner);
}

/// Build the per-entry display lines: a title line (with selection + coming-soon
/// markers) followed by an indented, muted description line.
fn entry_lines(state: &DashboardPickerState, theme: &EddaCraftTheme) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(state.entries.len() * 2);
    for (index, entry) in state.entries.iter().enumerate() {
        let selected = index == state.selected;
        let marker = if selected { "> " } else { "  " };
        let title_style = if selected {
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg())
        };

        let mut title_spans = vec![
            Span::styled(marker, title_style),
            Span::styled(entry.title.clone(), title_style),
        ];
        if !entry.available {
            title_spans.push(Span::styled(
                "  (coming soon)",
                Style::default().fg(theme.muted()),
            ));
        }
        lines.push(Line::from(title_spans));
        lines.push(Line::styled(
            format!("    {}", entry.description),
            Style::default().fg(theme.muted()),
        ));
    }
    lines
}

/// Borderless fallback for terminals narrower than the supported minimum:
/// titles only, with the selection marker, so the picker stays usable.
fn render_compact(
    frame: &mut Frame,
    area: Rect,
    state: &DashboardPickerState,
    theme: &EddaCraftTheme,
) {
    if state.entries.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled(
                "No dashboards",
                Style::default().fg(theme.muted()),
            )),
            area,
        );
        return;
    }

    let lines: Vec<Line<'static>> = state
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let marker = if index == state.selected { "> " } else { "  " };
            Line::styled(
                format!("{marker}{}", entry.title),
                Style::default().fg(theme.fg()),
            )
        })
        .collect();
    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

#[cfg(test)]
mod tests {
    use eddacraft_tui::theme::EddaCraftTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::super::{DashboardPickerState, sample_state};
    use super::*;

    fn render_state_to_string(state: &DashboardPickerState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = EddaCraftTheme;

        terminal
            .draw(|frame| render(frame, frame.area(), state, &theme))
            .unwrap();

        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn lists_every_dashboard_title() {
        let rendered = render_state_to_string(&sample_state(), 80, 20);
        assert!(rendered.contains("Architecture Health"), "got:\n{rendered}");
        assert!(rendered.contains("Drift Snapshots"), "got:\n{rendered}");
        assert!(rendered.contains("Suppressions"), "got:\n{rendered}");
    }

    #[test]
    fn marks_unavailable_dashboards_coming_soon() {
        // sample_state entries are all available: false.
        let rendered = render_state_to_string(&sample_state(), 80, 20);
        assert!(rendered.contains("coming soon"), "got:\n{rendered}");
    }

    #[test]
    fn shows_empty_state_when_no_dashboards() {
        let rendered = render_state_to_string(&DashboardPickerState::new(vec![]), 80, 20);
        assert!(rendered.contains("No dashboards"), "got:\n{rendered}");
    }

    #[test]
    fn marks_the_selected_entry() {
        let mut state = sample_state();
        state.selected = 1;
        let rendered = render_state_to_string(&state, 80, 20);
        // Selection marker precedes the highlighted title.
        let drift_line = rendered
            .lines()
            .find(|l| l.contains("Drift Snapshots"))
            .unwrap_or("");
        assert!(drift_line.contains('>'), "selected line: {drift_line:?}");
    }

    #[test]
    fn renders_without_panic_at_minimum_width() {
        // 80x24 minimum terminal — must not panic on a narrow content area.
        let _ = render_state_to_string(&sample_state(), 80, 24);
        let _ = render_state_to_string(&sample_state(), 40, 10);
    }

    #[test]
    fn narrow_terminal_keeps_titles_legible() {
        // Below the bordered minimum the compact path must still show titles,
        // not swallow them inside a collapsed container border.
        let rendered = render_state_to_string(&sample_state(), 20, 10);
        assert!(rendered.contains("Architect"), "got:\n{rendered}");
    }
}
