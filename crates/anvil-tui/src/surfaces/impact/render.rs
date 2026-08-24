//! Impact-view rendering: breadcrumb + counts header, the flow graph, and
//! honest degraded states.

use eddacraft_tui::flow::Background;
use eddacraft_tui::theme::{EddaCraftTheme, Role, Theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Wrap};

use super::ImpactState;

pub fn render(frame: &mut Frame, area: Rect, state: &ImpactState, theme: &EddaCraftTheme) {
    let [header, body] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);

    render_header(frame, header, state, theme);
    if let Some(err) = state.degraded() {
        render_degraded(frame, body, &err.to_string(), theme);
    } else if let Some(flow) = state.flow() {
        let mut flow = flow.borrow_mut();
        frame.render_widget(Background::new(&*flow), body);
        frame.render_widget(&mut *flow, body);
    }
}

fn render_header(frame: &mut Frame, area: Rect, state: &ImpactState, theme: &EddaCraftTheme) {
    let edges = state.current_edges();
    let mut nodes = std::collections::BTreeSet::new();
    for (a, b) in &edges {
        nodes.insert(a);
        nodes.insert(b);
    }
    let mut spans = vec![
        ratatui::text::Span::styled(
            format!(" {} ", state.breadcrumb()),
            theme.role_style(Role::Primary),
        ),
        ratatui::text::Span::styled(
            format!("· {} crates, {} edges ", nodes.len(), edges.len()),
            theme.role_style(Role::Secondary),
        ),
    ];
    if !state.status().is_empty() {
        spans.push(ratatui::text::Span::styled(
            format!("· {} ", state.status()),
            theme.role_style(Role::HighlightInactive),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_degraded(frame: &mut Frame, area: Rect, message: &str, theme: &EddaCraftTheme) {
    let height = 6.min(area.height);
    let width = area.width.saturating_sub(8).clamp(20, 76);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 3,
        width,
        height,
    };
    let lines = vec![
        Line::styled("impact view unavailable", theme.role_style(Role::Warning)),
        Line::raw(""),
        Line::styled(message.to_string(), theme.base()),
        Line::styled(
            "a warm snapshot appears after the anvil daemon has scanned this repo",
            theme.role_style(Role::Secondary),
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::bordered().border_style(theme.border_unfocused())),
        popup,
    );
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use eddacraft_tui::surface::Surface;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::surfaces::impact::data::{ImpactGraph, RawGraph};
    use crate::surfaces::impact::{ImpactState, ImpactView};
    use crate::test_utils::snapshot::buffer_to_string;
    use eddacraft_tui::keyboard::Action;
    use eddacraft_tui::theme::EddaCraftTheme;

    fn fixture_state() -> ImpactState {
        let files: BTreeSet<String> = [
            "crates/cli/src/lib.rs",
            "crates/kernel/src/lib.rs",
            "crates/kernel/src/graph.rs",
            "crates/types/src/lib.rs",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect();
        let raw_edges = vec![
            (
                "crates/kernel/src/lib.rs".to_string(),
                "crate::graph::Node".to_string(),
            ),
            (
                "crates/kernel/src/lib.rs".to_string(),
                "types::Id".to_string(),
            ),
            (
                "crates/cli/src/lib.rs".to_string(),
                "kernel::Engine".to_string(),
            ),
        ];
        let crate_edges = vec![
            ("cli".to_string(), "kernel".to_string()),
            ("kernel".to_string(), "types".to_string()),
        ];
        ImpactState::from_graph(ImpactGraph::from_parts(
            PathBuf::from("/repo"),
            crate_edges,
            RawGraph {
                edges: raw_edges,
                files,
            },
        ))
    }

    fn draw(state: &ImpactState) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("infallible");
        terminal
            .draw(|frame| state.render(frame, frame.area(), &EddaCraftTheme))
            .expect("infallible");
        buffer_to_string(terminal.backend().buffer())
    }

    #[test]
    fn renders_without_panic() {
        let state = fixture_state();
        let _ = draw(&state);
    }

    #[test]
    fn snapshot_crate_level() {
        let state = fixture_state();
        insta::assert_snapshot!(draw(&state));
    }

    #[test]
    fn snapshot_internals_drilldown() {
        let mut state = fixture_state();
        state.push_view(ImpactView::Internals("kernel".into()));
        assert_eq!(state.breadcrumb(), "all › kernel internals");
        insta::assert_snapshot!(draw(&state));
    }

    #[test]
    fn snapshot_degraded_no_snapshot() {
        // a root no daemon has ever scanned
        let state = ImpactState::load(std::path::Path::new("/"));
        insta::assert_snapshot!(draw(&state));
    }

    #[test]
    fn drill_and_back_restores_camera_and_selection() {
        use eddacraft_tui::flow::capture_view;

        let mut state = fixture_state();
        {
            let flow = state.flow().expect("loaded");
            let mut f = flow.borrow_mut();
            f.select_node("cli");
            f.zoom_to(1.5);
        }
        let before = capture_view(&state.flow().expect("loaded").borrow());

        state.push_view(ImpactView::Focus("kernel".into()));
        assert_eq!(
            state
                .flow()
                .expect("loaded")
                .borrow()
                .first_selected_node_id()
                .as_deref(),
            Some("kernel"),
            "drilled view should carry the drilled node as selection"
        );

        state.handle_key(Action::Back);
        let after = capture_view(&state.flow().expect("loaded").borrow());
        assert_eq!(
            after, before,
            "backing out should restore the parent's camera and selection"
        );
    }

    #[test]
    fn back_at_root_requests_exit() {
        let mut state = fixture_state();
        state.handle_key(Action::Back);
        assert!(state.should_back());
    }

    #[test]
    fn renders_in_small_area() {
        let state = fixture_state();
        let mut terminal = Terminal::new(TestBackend::new(40, 12)).expect("infallible");
        terminal
            .draw(|frame| state.render(frame, frame.area(), &EddaCraftTheme))
            .expect("infallible");
    }
}
