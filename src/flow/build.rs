//! Construction helpers: edge lists, role-styled specs, container grids.

use rataflow::{Edge, Flow, Node, StepEdge, Sugiyama, TextContent};
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use super::{edge_id as format_edge_id, flow_theme, role_color};
use crate::theme::{Role, Theme};

/// The stable edge-id convention used by [`container_flow`]: `"from -> to"`.
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

/// A node to place in a themed graph.
///
/// # Stability
///
/// **experimental** (TUIN-017).
#[derive(Debug, Clone)]
pub struct NodeSpec {
    /// Stable node id.
    pub id: String,
    /// Label drawn inside the node.
    pub label: String,
    /// Optional theme role for border/text colour.
    pub role: Option<Role>,
    /// Optional parent container id.
    pub parent: Option<String>,
}

impl NodeSpec {
    /// Build a spec with id and label, no role or parent.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            role: None,
            parent: None,
        }
    }

    /// Assign a theme role.
    #[must_use]
    pub fn with_role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }
}

/// An edge to place in a themed graph.
///
/// # Stability
///
/// **experimental** (TUIN-017).
#[derive(Debug, Clone)]
pub struct EdgeSpec {
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
    /// Optional theme role for the stroke.
    pub role: Option<Role>,
    /// Whether the edge should animate (marching ants).
    pub animated: bool,
}

impl EdgeSpec {
    /// Build a spec between two node ids.
    #[must_use]
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            role: None,
            animated: false,
        }
    }

    /// Assign a theme role.
    #[must_use]
    pub fn with_role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// Animate the edge.
    #[must_use]
    pub fn animated(mut self) -> Self {
        self.animated = true;
        self
    }
}

/// Construct a themed flow graph from directed edges, laid out with a
/// vertical Sugiyama pass. Node ids are the edge labels verbatim.
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

/// Construct a themed flow graph from [`NodeSpec`] / [`EdgeSpec`].
///
/// # Stability
///
/// **experimental** (TUIN-017).
pub fn themed_from_specs<T: Theme + ?Sized>(
    nodes: &[NodeSpec],
    edges: &[EdgeSpec],
    theme: &T,
) -> Result<Flow, rataflow::Error> {
    let pairs: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect();
    let mut flow = if pairs.is_empty() && !nodes.is_empty() {
        let labels: Vec<(&str, &str)> = nodes
            .windows(2)
            .map(|w| (w[0].id.as_str(), w[1].id.as_str()))
            .collect();
        if labels.is_empty() {
            themed_from_edges(&[(nodes[0].id.as_str(), nodes[0].id.as_str())], theme)?
        } else {
            themed_from_edges(&labels, theme)?
        }
    } else {
        themed_from_edges(&pairs, theme)?
    };

    for spec in nodes {
        apply_node_role(&mut flow, &spec.id, spec.role, theme);
        if let Some(content) = flow.node_content_mut(&spec.id) {
            *content = TextContent::from(spec.label.as_str());
            apply_content_role(content, spec.role, theme);
        }
    }
    for spec in edges {
        for id in edge_ids_between(&flow, &spec.from, &spec.to) {
            flow.set_edge_animated(&id, spec.animated);
            apply_edge_role(&mut flow, &id, spec.role, theme);
        }
    }
    Ok(flow)
}

pub(crate) fn edge_ids_between(flow: &Flow, from: &str, to: &str) -> Vec<String> {
    flow.edges()
        .iter()
        .filter(|e| e.source == from && e.target == to)
        .map(|e| e.id.clone())
        .collect()
}

/// Build a themed boundary view: each group becomes a titled, non-selectable
/// parent-container box with its members gridded inside.
///
/// This is deliberately *not* composed with Sugiyama layout.
///
/// # Stability
///
/// **experimental** (TUIN-014).
pub fn container_flow<T: Theme + ?Sized>(
    groups: &[ContainerGroup],
    edges: &[(String, String)],
    theme: &T,
) -> Result<Flow, rataflow::Error> {
    #![allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    const CELL_H: f64 = 5.0;
    const GAP: f64 = 2.0;

    let mut nodes: Vec<Node<TextContent>> = Vec::new();
    let mut present: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
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
            .map(|m| UnicodeWidthStr::width(m.as_str()))
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
            present.insert(member.as_str());
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
        .map(|(a, b)| Edge::new(format_edge_id(a, b), a.clone(), b.clone()))
        .collect();
    Ok(Flow::with_graph(nodes, flow_edges)?.with_theme(flow_theme(theme)))
}

pub(crate) fn apply_node_role<T: Theme + ?Sized>(
    flow: &mut Flow,
    id: &str,
    role: Option<Role>,
    theme: &T,
) {
    if let Some(content) = flow.node_content_mut(id) {
        apply_content_role(content, role, theme);
    }
}

pub(crate) fn apply_content_role<T: Theme + ?Sized>(
    content: &mut TextContent,
    role: Option<Role>,
    theme: &T,
) {
    if let Some(role) = role {
        let color = role_color(theme, role);
        content.border_style = Some(Style::default().fg(color));
        content.text_style = Some(Style::default().fg(color));
    }
}

pub(crate) fn apply_edge_role<T: Theme + ?Sized>(
    flow: &mut Flow,
    id: &str,
    role: Option<Role>,
    theme: &T,
) {
    let Some(role) = role else {
        return;
    };
    let color = role_color(theme, role);
    if let Some(content) = flow.edge_content_mut(id) {
        *content = StepEdge::default().with_style(
            rataflow::EdgeStyle::default().with_stroke_style(Style::default().fg(color)),
        );
    }
}
