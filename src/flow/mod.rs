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

use std::io;

use ratatui::style::Color;

use crate::shared_mode::SharedMode;

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
/// Mouse capture belongs to the process, so guards may overlap, for example
/// an inner [`FlowSession`] inside an outer one. Every live guard shares the
/// capture: the first guard turns it on, and it is turned off only when the
/// last live guard drops.
///
/// # Stability
///
/// **experimental** (TUIN-014).
pub struct MouseCaptureGuard {
    /// The capture this guard holds and how to turn it off, or `None` when
    /// enabling failed and the guard holds nothing.
    held: Option<(&'static SharedMode, fn())>,
}

/// Live [`MouseCaptureGuard`]s in this process (GH #6557).
static MOUSE_CAPTURE: SharedMode = SharedMode::new();

impl MouseCaptureGuard {
    /// Enable mouse reporting on stdout, or join the capture another live
    /// guard already enabled. Failure to enable is deliberately swallowed — a
    /// terminal without mouse support still has a fully keyboard-driven flow
    /// surface. A failed enable is rolled back best effort and the returned
    /// guard holds nothing, so a later `enable` tries again.
    #[must_use]
    pub fn enable() -> Self {
        Self::enable_on(&MOUSE_CAPTURE, enable_mouse_capture, disable_mouse_capture)
    }

    fn enable_on(
        shared: &'static SharedMode,
        enable: fn() -> io::Result<()>,
        disable: fn(),
    ) -> Self {
        let entered = shared.acquire(|| {
            // The escapes may have been written in part; turn off whatever
            // got through.
            enable().inspect_err(|_| disable())
        });
        Self {
            held: entered.is_ok().then_some((shared, disable)),
        }
    }
}

fn enable_mouse_capture() -> io::Result<()> {
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)
}

fn disable_mouse_capture() {
    let _ = crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture);
}

impl Drop for MouseCaptureGuard {
    fn drop(&mut self) {
        if let Some((shared, disable)) = self.held.take() {
            shared.release_best_effort(disable);
        }
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

    fn edge_pairs(flow: &Flow) -> Vec<(String, String)> {
        flow.edges()
            .iter()
            .map(|e| (e.source.clone(), e.target.clone()))
            .collect()
    }

    fn node_ids(flow: &Flow) -> Vec<String> {
        let mut ids: Vec<String> = flow.nodes().map(|n| n.id.clone()).collect();
        ids.sort();
        ids
    }

    #[test]
    fn specs_without_edges_register_isolated_nodes_only() {
        // Regression (#6605): edge-less specs used to be chained together
        // (or self-looped) so the nodes would materialise.
        let theme = EddaCraftTheme;
        let nodes = [NodeSpec::new("a", "A"), NodeSpec::new("b", "B")];
        let mut flow = themed_from_specs(&nodes, &[], &theme).expect("valid");
        assert_eq!(node_ids(&flow), ["a", "b"]);
        assert!(edge_pairs(&flow).is_empty(), "{:?}", edge_pairs(&flow));
        let a = flow.node_bounds("a").expect("a");
        let b = flow.node_bounds("b").expect("b");
        assert!(!a.intersects(&b), "isolated nodes overlap: {a:?} {b:?}");
        let rendered = draw_plain(&mut flow, 60, 20);
        assert!(
            rendered.contains('A') && rendered.contains('B'),
            "{rendered}"
        );
    }

    #[test]
    fn single_spec_without_edges_has_no_self_loop() {
        let theme = EddaCraftTheme;
        let flow = themed_from_specs(&[NodeSpec::new("solo", "solo")], &[], &theme).expect("valid");
        assert_eq!(node_ids(&flow), ["solo"]);
        assert!(edge_pairs(&flow).is_empty(), "{:?}", edge_pairs(&flow));
    }

    #[test]
    fn specs_register_isolated_node_beside_supplied_edges() {
        let theme = EddaCraftTheme;
        let nodes = [
            NodeSpec::new("a", "a"),
            NodeSpec::new("b", "b"),
            NodeSpec::new("lone", "lone").with_role(Role::Warning),
        ];
        let edges = [EdgeSpec::new("a", "b")];
        let flow = themed_from_specs(&nodes, &edges, &theme).expect("valid");
        assert_eq!(node_ids(&flow), ["a", "b", "lone"]);
        assert_eq!(edge_pairs(&flow), [("a".to_string(), "b".to_string())]);
        let lone = flow.node_bounds("lone").expect("lone");
        for other in ["a", "b"] {
            let bounds = flow.node_bounds(other).expect("laid out");
            assert!(!lone.intersects(&bounds), "lone overlaps {other}");
        }
    }

    #[test]
    fn elide_registers_portal_without_boundary_edges() {
        // Collapsed x/y only touch each other, so no rewritten edge reaches
        // the portal; it must still be a node in the flow.
        let theme = EddaCraftTheme;
        let edges = [("a", "b"), ("b", "c"), ("a", "c"), ("x", "y")];
        let elided = elide_from_edges(&edges, &theme, 4).expect("valid");
        let portal = elided.portal_id.clone().expect("portal");
        assert_eq!(elided.collapsed, ["x", "y"]);
        assert!(elided.flow.node(&portal).is_some(), "portal not registered");
        assert!(
            elided
                .flow
                .edges()
                .iter()
                .all(|e| e.source != portal && e.target != portal)
        );
        assert_eq!(elided.flow.edges().len(), 3);
    }

    #[test]
    fn elide_with_no_rewritten_edges_does_not_synthesise_a_self_loop() {
        let theme = EddaCraftTheme;
        let elided = elide_from_edges(&[("x", "y")], &theme, 1).expect("valid");
        let portal = elided.portal_id.clone().expect("portal");
        assert_eq!(node_ids(&elided.flow), [portal]);
        assert!(edge_pairs(&elided.flow).is_empty());
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

#[cfg(test)]
mod mouse_capture_tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn a_failed_enable_holds_nothing_and_the_next_enable_retries() {
        // Statics local to this test, so no other test shares the capture.
        static SHARED: SharedMode = SharedMode::new();
        static FAIL_NEXT: AtomicBool = AtomicBool::new(true);
        static ON: AtomicBool = AtomicBool::new(false);
        static ENABLES: AtomicUsize = AtomicUsize::new(0);
        static DISABLES: AtomicUsize = AtomicUsize::new(0);

        fn enable() -> io::Result<()> {
            ENABLES.fetch_add(1, Ordering::SeqCst);
            // A partial write: capture is half on when the error arrives.
            ON.store(true, Ordering::SeqCst);
            if FAIL_NEXT.swap(false, Ordering::SeqCst) {
                return Err(io::Error::other("write failed"));
            }
            Ok(())
        }
        fn disable() {
            DISABLES.fetch_add(1, Ordering::SeqCst);
            ON.store(false, Ordering::SeqCst);
        }

        let failed = MouseCaptureGuard::enable_on(&SHARED, enable, disable);
        assert!(failed.held.is_none(), "a failed enable armed the guard");
        assert!(
            !ON.load(Ordering::SeqCst),
            "the partial enable was not rolled back"
        );
        assert_eq!(SHARED.live_holders(), 0, "a failed enable counted a holder");

        // With no holder left behind, the next guard writes the enable again.
        let live = MouseCaptureGuard::enable_on(&SHARED, enable, disable);
        assert!(live.held.is_some());
        assert!(ON.load(Ordering::SeqCst));
        assert_eq!(
            ENABLES.load(Ordering::SeqCst),
            2,
            "the enable was not retried"
        );

        // The failed guard owns nothing, so dropping it leaves the live
        // guard's capture and count alone.
        drop(failed);
        assert!(
            ON.load(Ordering::SeqCst),
            "the failed guard turned capture off"
        );
        assert_eq!(
            SHARED.live_holders(),
            1,
            "the failed guard released a holder"
        );

        drop(live);
        assert!(!ON.load(Ordering::SeqCst));
        assert_eq!(SHARED.live_holders(), 0);
        // One rollback of the failed enable, one release of the live guard.
        assert_eq!(DISABLES.load(Ordering::SeqCst), 2);
    }
}
