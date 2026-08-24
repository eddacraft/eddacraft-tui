//! Interactive flow-graph rendering built on [`rataflow`].
//!
//! Available behind the `flow` Cargo feature (TUIN-014, extensions TUIN-015..021).
//! Wraps the [`rataflow`] node-graph engine with `EddaCraft` theming and the
//! helpers proven by the anvil `spike-flow` validation spike plus the
//! post-0.5.1 wrapper wave: spotlight, view preservation, role-styled specs,
//! graph diff, session RAII, and elision portals.
//!
//! Consumers should depend on this module rather than on `rataflow`
//! directly; the dependency is pinned exact (`=0.1.0`) and the full
//! upstream API remains reachable through [`raw`] while the wrapper
//! surface is still settling.
//!
//! Two upstream behaviours every consumer must know:
//!
//! - `ratatui::run` (and the `lifecycle` feature's `TerminalGuard`) do **not**
//!   enable mouse reporting. Without [`MouseCaptureGuard`] / [`FlowSession`]
//!   the terminal never sends mouse events.
//! - `Flow::request_fit_view` is deferred to the *next* render, so
//!   programmatic zoom applied before the first frame is silently
//!   overridden. Prefer [`zoom_to_read_after_layout`].

mod build;
mod diff;
mod elide;
mod session;
mod spotlight;
mod view;

use ratatui::style::Color;

/// The full upstream `rataflow` API, for anything the curated surface does
/// not re-export.
///
/// # Stability
///
/// **experimental**. The escape hatch exists precisely because the curated
/// wrapper is still settling; items reached through `raw` carry upstream's
/// 0.x stability, not this crate's.
pub use rataflow as raw;

pub use rataflow::{
    Background, Edge, EventResponse, Flow, FlowEvent, Node, StepEdge, Sugiyama, TextContent,
    Viewport,
};

pub use build::{
    ContainerGroup, EdgeSpec, NodeSpec, container_flow, edge_id, themed_from_edges,
    themed_from_specs,
};
pub use diff::themed_from_diff;
pub use elide::{ElidedGraph, elide_from_edges, elide_from_edges_keeping};
pub use session::FlowSession;
pub use spotlight::{Spotlight, spotlight};
pub use view::{
    ViewState, capture_view, rebuild_preserving_view, restore_view, zoom_to_read,
    zoom_to_read_after_layout,
};

use crate::theme::Theme;

/// Build a [`rataflow::Theme`] from an `EddaCraft` [`Theme`] so flow graphs
/// match the rest of the application chrome.
///
/// The mapping covers rataflow's eight-colour palette from the trait's
/// required colours; `warning` has no rataflow slot today and is applied
/// per-node/edge via [`NodeSpec`] / [`EdgeSpec`] instead.
///
/// # Stability
///
/// **experimental** (TUIN-014).
#[must_use]
pub fn flow_theme<T: Theme + ?Sized>(theme: &T) -> rataflow::Theme {
    rataflow::Theme::Custom(rataflow::Palette {
        canvas_bg: theme.bg(),
        surface: theme.bg(),
        muted: theme.muted(),
        subtle: theme.border(),
        accent: theme.accent(),
        text: theme.fg(),
        success: theme.success(),
        error: theme.error(),
    })
}

/// Turns terminal mouse reporting on for the guard's lifetime and reliably
/// off again on any exit path (quit, error, panic unwind).
///
/// Neither `ratatui::run` nor the `lifecycle` feature's `TerminalGuard`
/// enables mouse capture; a flow surface that wants scroll-zoom, node
/// dragging, or edge creation must hold one of these for the duration of
/// the interactive session.
///
/// # Stability
///
/// **experimental** (TUIN-014).
pub struct MouseCaptureGuard(());

impl MouseCaptureGuard {
    /// Enable mouse reporting on stdout. Failure to enable is deliberately
    /// swallowed — a terminal without mouse support still has a fully
    /// keyboard-driven flow surface.
    #[must_use]
    pub fn enable() -> Self {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
        Self(())
    }
}

impl Drop for MouseCaptureGuard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    }
}

// Compile-time link so the doc mapping above stays truthful: `warning` is
// intentionally unmapped on the rataflow palette; role-styled specs carry it.
const _: fn(&dyn Theme) -> Color = |t| t.warning();

pub(crate) fn role_color<T: Theme + ?Sized>(theme: &T, role: crate::theme::Role) -> Color {
    theme.role_style(role).fg.unwrap_or_else(|| theme.fg())
}

#[cfg(test)]
pub(crate) fn draw(flow: &mut Flow, width: u16, height: u16) -> String {
    use crate::test_utils::snapshot::buffer_to_string;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("infallible");
    terminal
        .draw(|frame| {
            frame.render_widget(Background::new(flow), frame.area());
            frame.render_widget(&mut *flow, frame.area());
        })
        .expect("infallible");
    buffer_to_string(terminal.backend().buffer())
}

#[cfg(test)]
pub(crate) fn draw_plain(flow: &mut Flow, width: u16, height: u16) -> String {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("infallible");
    terminal
        .draw(|frame| {
            frame.render_widget(Background::new(flow), frame.area());
            frame.render_widget(&mut *flow, frame.area());
        })
        .expect("infallible");
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{EddaCraftTheme, Role};

    #[test]
    fn themed_from_edges_renders_labels() {
        let theme = EddaCraftTheme;
        let mut flow =
            themed_from_edges(&[("alpha", "beta"), ("beta", "gamma")], &theme).expect("valid");
        let rendered = draw_plain(&mut flow, 60, 30);
        assert!(
            rendered.contains("alpha"),
            "missing node label:\n{rendered}"
        );
        assert!(
            rendered.contains("gamma"),
            "missing node label:\n{rendered}"
        );
    }

    #[test]
    fn snapshot_container_flow() {
        let theme = EddaCraftTheme;
        let groups = [
            ContainerGroup {
                title: "surface".into(),
                members: vec!["cli".into(), "tui".into()],
            },
            ContainerGroup {
                title: "engine".into(),
                members: vec!["kernel".into()],
            },
        ];
        let edges = vec![
            ("cli".to_string(), "kernel".to_string()),
            ("tui".to_string(), "kernel".to_string()),
        ];
        let mut flow = container_flow(&groups, &edges, &theme).expect("valid");
        insta::assert_snapshot!(draw(&mut flow, 60, 32));
    }

    #[test]
    fn edge_id_round_trip_addresses_edges() {
        let theme = EddaCraftTheme;
        let groups = [ContainerGroup {
            title: "g".into(),
            members: vec!["a".into(), "b".into()],
        }];
        let edges = vec![("a".to_string(), "b".to_string())];
        let mut flow = container_flow(&groups, &edges, &theme).expect("valid");
        flow.set_edge_animated(&edge_id("a", "b"), true);
        let rendered = draw_plain(&mut flow, 40, 24);
        assert!(
            rendered.contains('a'),
            "graph failed to render:\n{rendered}"
        );
    }

    #[test]
    fn spotlight_unknown_id_is_noop() {
        let theme = EddaCraftTheme;
        let mut flow =
            themed_from_edges(&[("alpha", "beta"), ("beta", "gamma")], &theme).expect("valid");
        spotlight(&mut flow, "missing", Spotlight::Both, &theme);
        let ab = flow
            .edges()
            .iter()
            .find(|e| e.source == "alpha" && e.target == "beta")
            .expect("edge");
        assert!(!ab.animated);
    }

    #[test]
    fn spotlight_downstream_animates_cone_edges() {
        let theme = EddaCraftTheme;
        let mut flow =
            themed_from_edges(&[("alpha", "beta"), ("beta", "gamma")], &theme).expect("valid");
        spotlight(&mut flow, "alpha", Spotlight::Downstream, &theme);
        let animated = |from: &str, to: &str| {
            flow.edges()
                .iter()
                .find(|e| e.source == from && e.target == to)
                .expect("edge")
                .animated
        };
        assert!(animated("alpha", "beta"));
        assert!(animated("beta", "gamma"));
    }

    #[test]
    fn spotlight_mutes_complement_edges() {
        let theme = EddaCraftTheme;
        let mut flow =
            themed_from_edges(&[("alpha", "beta"), ("omega", "beta")], &theme).expect("valid");
        spotlight(&mut flow, "alpha", Spotlight::Downstream, &theme);
        let animated = |from: &str, to: &str| {
            flow.edges()
                .iter()
                .find(|e| e.source == from && e.target == to)
                .expect("edge")
                .animated
        };
        assert!(animated("alpha", "beta"));
        assert!(!animated("omega", "beta"));
    }

    #[test]
    fn rebuild_preserving_view_keeps_selection_and_zoom() {
        let theme = EddaCraftTheme;
        let mut original =
            themed_from_edges(&[("alpha", "beta"), ("beta", "gamma")], &theme).expect("valid");
        original.select_node("beta");
        original.zoom_to(1.0);
        let rebuilt = rebuild_preserving_view(&original, || {
            themed_from_edges(&[("alpha", "beta"), ("beta", "delta")], &theme).expect("valid")
        });
        assert_eq!(rebuilt.first_selected_node_id().as_deref(), Some("beta"));
        assert!((rebuilt.viewport.zoom - original.viewport.zoom).abs() < f64::EPSILON);
    }

    #[test]
    fn zoom_to_read_after_layout_selects_and_sets_1_to_1() {
        let theme = EddaCraftTheme;
        let mut flow =
            themed_from_edges(&[("alpha", "beta"), ("beta", "gamma")], &theme).expect("valid");
        flow.request_fit_view();
        zoom_to_read_after_layout(&mut flow, "gamma", 60, 30);
        assert_eq!(flow.first_selected_node_id().as_deref(), Some("gamma"));
        assert!((flow.viewport.zoom - 1.0).abs() < 0.05);
        // A later same-size frame must not re-apply the consumed fit request.
        let _ = draw_plain(&mut flow, 60, 30);
        assert!((flow.viewport.zoom - 1.0).abs() < 0.05);
    }

    #[test]
    fn container_flow_uses_display_width_for_wide_glyphs() {
        let theme = EddaCraftTheme;
        let groups = [ContainerGroup {
            title: "g".into(),
            members: vec!["日本語".into(), "ab".into()],
        }];
        let mut flow = container_flow(&groups, &[], &theme).expect("valid");
        let rendered = draw_plain(&mut flow, 80, 24);
        assert!(
            rendered.contains("日本語") || rendered.contains("日"),
            "wide-glyph label missing:\n{rendered}"
        );
    }

    #[test]
    fn role_styled_edge_is_accepted() {
        let theme = EddaCraftTheme;
        let nodes = [
            NodeSpec::new("a", "a").with_role(Role::Success),
            NodeSpec::new("b", "b").with_role(Role::Error),
        ];
        let edges = [EdgeSpec::new("a", "b").with_role(Role::Warning)];
        let mut flow = themed_from_specs(&nodes, &edges, &theme).expect("valid");
        let rendered = draw_plain(&mut flow, 40, 20);
        assert!(rendered.contains('a'), "missing a:\n{rendered}");
        assert!(rendered.contains('b'), "missing b:\n{rendered}");
    }

    #[test]
    fn graph_diff_keeps_removed_nodes_as_ghosts() {
        let theme = EddaCraftTheme;
        let before = [("a", "b"), ("b", "c")];
        let after = [("a", "b")];
        let mut flow = themed_from_diff(&before, &after, &theme).expect("valid");
        let rendered = draw_plain(&mut flow, 60, 30);
        assert!(rendered.contains('c'), "ghost node c missing:\n{rendered}");
        let ghost = flow
            .edges()
            .iter()
            .find(|e| e.source == "b" && e.target == "c")
            .expect("ghost edge");
        assert!(!ghost.hidden, "ghost edge should occupy space");
    }

    #[test]
    fn flow_session_drop_does_not_panic() {
        let session = FlowSession::enter();
        drop(session);
    }

    #[test]
    fn elide_collapses_over_budget_nodes_to_a_portal() {
        let theme = EddaCraftTheme;
        let edges = [("a", "b"), ("b", "c"), ("c", "d"), ("d", "e")];
        let elided = elide_from_edges(&edges, &theme, 3).expect("valid");
        assert!(elided.portal_id.is_some(), "expected a portal");
        assert!(
            elided.collapsed.len() >= 2,
            "expected collapsed members, got {:?}",
            elided.collapsed
        );
        let mut flow = elided.flow;
        let rendered = draw_plain(&mut flow, 80, 30);
        assert!(
            rendered.contains('…') || rendered.contains("crates"),
            "portal label missing:\n{rendered}"
        );
    }

    #[test]
    fn expand_elision_restores_collapsed_members() {
        let theme = EddaCraftTheme;
        let edges = [("a", "b"), ("b", "c"), ("c", "d")];
        let elided = elide_from_edges(&edges, &theme, 2).expect("valid");
        let keep: Vec<&str> = elided.collapsed.iter().map(String::as_str).collect();
        let expanded = elide_from_edges_keeping(&edges, &theme, 8, &keep).expect("valid");
        assert!(expanded.portal_id.is_none());
        let mut flow = expanded.flow;
        let rendered = draw_plain(&mut flow, 80, 30);
        assert!(
            rendered.contains('d') || rendered.contains('c'),
            "{rendered}"
        );
    }

    #[test]
    fn elide_unknown_always_keep_does_not_shrink_budget() {
        let theme = EddaCraftTheme;
        let edges = [("a", "b"), ("b", "c"), ("c", "d"), ("d", "e")];
        let with_ghost = elide_from_edges_keeping(&edges, &theme, 3, &["missing"]).expect("valid");
        let plain = elide_from_edges(&edges, &theme, 3).expect("valid");
        assert_eq!(with_ghost.collapsed.len(), plain.collapsed.len());
    }

    #[test]
    fn elide_portal_id_is_reserved_and_label_is_crates() {
        let theme = EddaCraftTheme;
        let edges = [("a", "b"), ("b", "c"), ("c", "d"), ("… 2 crates", "a")];
        let elided = elide_from_edges(&edges, &theme, 3).expect("valid");
        let portal = elided.portal_id.as_deref().expect("portal");
        assert!(
            portal.contains("flow-portal"),
            "expected reserved portal id, got {portal}"
        );
        assert_ne!(portal, "… 2 crates");
        let mut flow = elided.flow;
        let rendered = draw_plain(&mut flow, 80, 30);
        assert!(
            rendered.contains("crates"),
            "display label missing:\n{rendered}"
        );
    }

    #[test]
    fn morph_lerp_is_monotonic() {
        // TUIN-021 spike: lerp of two viewports is well-defined without a
        // public morph API. Coupling flow to animate-core is deferred.
        let a = Viewport::new(0.0, 0.0, 1.0);
        let b = Viewport::new(10.0, 4.0, 2.0);
        let mid = Viewport::new(
            a.x + (b.x - a.x) * 0.5,
            a.y + (b.y - a.y) * 0.5,
            a.zoom + (b.zoom - a.zoom) * 0.5,
        );
        assert!((mid.x - 5.0).abs() < f64::EPSILON);
        assert!((mid.zoom - 1.5).abs() < f64::EPSILON);
    }
}
