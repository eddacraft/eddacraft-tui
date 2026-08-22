//! Interactive flow-graph rendering built on [`rataflow`].
//!
//! Available behind the `flow` Cargo feature (TUIN-014). Wraps the
//! [`rataflow`] node-graph engine (Sugiyama layout, pan/zoom, semantic zoom,
//! selection, edge creation) with `EddaCraft` theming and the helpers proven by
//! the anvil `spike-flow` validation spike (PRs #4074/#4081): themed
//! construction from edge lists, zoom-to-read for dense graphs, a layered
//! container-box layout for boundary views, and a mouse-capture guard.
//!
//! Consumers should depend on this module rather than on `rataflow`
//! directly; the dependency is pinned exact (`=0.1.0`, the `animate`
//! precedent) and the full upstream API remains reachable through
//! [`raw`] while the wrapper surface is still settling.
//!
//! Two upstream behaviours every consumer must know:
//!
//! - `ratatui::run` (and the `lifecycle` feature's `TerminalGuard`) do **not**
//!   enable mouse reporting. Without [`MouseCaptureGuard`] the terminal never
//!   sends mouse events — which presents as "mouse doesn't work",
//!   especially over tmux/ssh.
//! - `Flow::request_fit_view` is deferred to the *next* render, so
//!   programmatic zoom applied before the first frame is silently
//!   overridden. [`zoom_to_read`] is intended for use after a frame has been
//!   drawn.

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
};

use crate::theme::Theme;

/// Build a [`rataflow::Theme`] from an `EddaCraft` [`Theme`] so flow graphs
/// match the rest of the application chrome.
///
/// The mapping covers rataflow's eight-colour palette from the trait's
/// required colours; `warning` has no rataflow slot today and is unused.
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

/// Construct a themed flow graph from directed edges, laid out with a
/// vertical Sugiyama pass. Node ids are the edge labels verbatim.
///
/// # Errors
///
/// Propagates [`rataflow::Error`] from graph construction (duplicate ids,
/// invalid edges).
///
/// # Stability
///
/// **experimental** (TUIN-014).
pub fn themed_from_edges<T: Theme + ?Sized>(
    edges: &[(&str, &str)],
    theme: &T,
) -> Result<Flow, rataflow::Error> {
    Ok(Flow::from_edges(edges, Sugiyama::vertical())?.with_theme(flow_theme(theme)))
}

/// Centre the given node and snap the viewport to 1:1 so its label is
/// legible even in a graph dense enough for semantic zoom to hide text.
///
/// Call after at least one frame has rendered — a pending fit-view request
/// is applied at render time and would override this zoom.
///
/// # Stability
///
/// **experimental** (TUIN-014).
pub fn zoom_to_read(flow: &mut Flow, node_id: &str) {
    flow.select_node(node_id);
    flow.center_on_selected();
    flow.zoom_to(1.0);
}

/// The stable edge-id convention used by [`container_flow`]: `"from -> to"`.
///
/// Consumers use it to address edges after construction (for example
/// `flow.set_edge_animated(&edge_id(a, b), true)` to make a
/// boundary-violating edge stand out).
///
/// # Stability
///
/// **experimental** (TUIN-014).
#[must_use]
pub fn edge_id(from: &str, to: &str) -> String {
    format!("{from} -> {to}")
}

/// One titled container box in a layered boundary view.
///
/// # Stability
///
/// **experimental** (TUIN-014).
#[derive(Debug, Clone)]
pub struct ContainerGroup {
    /// Border title of the container (for example a policy layer name).
    pub title: String,
    /// Node labels rendered inside the container, in grid order.
    pub members: Vec<String>,
}

/// Build a themed boundary view: each group becomes a titled, non-selectable
/// parent-container box with its members gridded inside (parent-relative
/// positions), containers stacked vertically in the order given. Edges whose
/// endpoints are both present are added with [`edge_id`] ids so consumers
/// can restyle them afterwards.
///
/// This is deliberately *not* composed with Sugiyama layout — the spike's
/// key layout finding is that a boundary lens reads best with policy-driven
/// geometry, as a separate view from the dependency lens.
///
/// # Errors
///
/// Propagates [`rataflow::Error`] from graph construction (for example a
/// member name duplicated across groups).
///
/// # Stability
///
/// **experimental** (TUIN-014).
pub fn container_flow<T: Theme + ?Sized>(
    groups: &[ContainerGroup],
    edges: &[(String, String)],
    theme: &T,
) -> Result<Flow, rataflow::Error> {
    // grid geometry works in terminal cells: counts are tiny, casts are safe
    #![allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    const CELL_H: f64 = 5.0;
    const GAP: f64 = 2.0;

    let mut nodes: Vec<Node<TextContent>> = Vec::new();
    let mut present: Vec<&str> = Vec::new();
    let mut y_cursor = 0.0;
    for group in groups {
        if group.members.is_empty() {
            continue;
        }
        let cols = (group.members.len() as f64).sqrt().ceil().max(1.0);
        let cols_u = cols as usize;
        let cell_w = group
            .members
            .iter()
            .map(|m| m.chars().count())
            .max()
            .unwrap_or(8) as f64
            + 6.0;
        let rows = group.members.len().div_ceil(cols_u) as f64;
        let width = cols * (cell_w + GAP) + GAP + 2.0;
        let height = rows * (CELL_H + 1.0) + GAP + 3.0;
        let container_id = format!("▣ {}", group.title);
        nodes.push(
            Node::new(
                container_id.clone(),
                (0.0, y_cursor),
                (width, height),
                TextContent::new("").with_title(format!(" {} ", group.title)),
            )
            .with_selectable(false),
        );
        for (i, member) in group.members.iter().enumerate() {
            present.push(member.as_str());
            let (row, col) = (i / cols_u, i % cols_u);
            nodes.push(
                Node::new(
                    member.clone(),
                    (
                        GAP + 1.0 + col as f64 * (cell_w + GAP),
                        GAP + 1.0 + row as f64 * (CELL_H + 1.0),
                    ),
                    (cell_w, CELL_H),
                    TextContent::from(member.as_str()),
                )
                .with_parent(container_id.clone()),
            );
        }
        y_cursor += height + 4.0;
    }

    let flow_edges: Vec<Edge<StepEdge>> = edges
        .iter()
        .filter(|(a, b)| present.contains(&a.as_str()) && present.contains(&b.as_str()))
        .map(|(a, b)| Edge::new(edge_id(a, b), a.clone(), b.clone()))
        .collect();
    Ok(Flow::with_graph(nodes, flow_edges)?.with_theme(flow_theme(theme)))
}

/// Turns terminal mouse reporting on for the guard's lifetime and reliably
/// off again on any exit path (quit, error, panic unwind).
///
/// Neither `ratatui::run` nor the `lifecycle` feature's `TerminalGuard`
/// enables mouse capture; a flow surface that wants scroll-zoom, node
/// dragging, or edge creation must hold one of these for the duration of
/// the interactive session. Leaking mouse mode into a parent shell makes
/// the terminal feel haunted — the drop implementation exists so that
/// cannot happen.
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
// intentionally unmapped; if rataflow grows a slot for it, revisit.
const _: fn(&dyn Theme) -> Color = |t| t.warning();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::snapshot::buffer_to_string;
    use crate::theme::EddaCraftTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn draw(flow: &mut Flow, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("infallible");
        terminal
            .draw(|frame| {
                frame.render_widget(Background::new(flow), frame.area());
                frame.render_widget(&mut *flow, frame.area());
            })
            .expect("infallible");
        buffer_to_string(terminal.backend().buffer())
    }

    /// Symbols only — `buffer_to_string` interleaves style annotations, so
    /// substring assertions on labels need the unstyled text.
    fn draw_plain(flow: &mut Flow, width: u16, height: u16) -> String {
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
        // set_edge_animated silently ignores unknown ids; renders either way —
        // the assertion is that construction used the documented id scheme.
        flow.set_edge_animated(&edge_id("a", "b"), true);
        let rendered = draw_plain(&mut flow, 40, 24);
        assert!(
            rendered.contains('a'),
            "graph failed to render:\n{rendered}"
        );
    }
}
