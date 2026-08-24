//! Spotlight cone over a selected node (TUIN-015).

use std::collections::{BTreeSet, VecDeque};

use rataflow::Flow;

use crate::theme::{Role, Theme};

/// Which direction(s) of the graph the spotlight walks.
///
/// # Stability
///
/// **experimental** (TUIN-015).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spotlight {
    /// Ancestors of the seed (incoming edges, recursively).
    Upstream,
    /// Descendants of the seed (outgoing edges, recursively).
    Downstream,
    /// Both directions.
    Both,
}

/// Mute the complement of `node_id`'s cone and animate remaining edges.
///
/// Unknown ids are a no-op. Roles resolve through `theme` so a custom
/// palette is not overwritten by [`crate::theme::EddaCraftTheme`].
///
/// # Stability
///
/// **experimental** (TUIN-015).
pub fn spotlight<T: Theme + ?Sized>(
    flow: &mut Flow,
    node_id: &str,
    direction: Spotlight,
    theme: &T,
) {
    if flow.node(node_id).is_none() {
        return;
    }

    let edges: Vec<(String, String, String)> = flow
        .edges()
        .iter()
        .map(|e| (e.id.clone(), e.source.clone(), e.target.clone()))
        .collect();

    let mut cone: BTreeSet<String> = BTreeSet::new();
    cone.insert(node_id.to_string());
    let mut queue = VecDeque::new();
    queue.push_back(node_id.to_string());

    while let Some(current) = queue.pop_front() {
        for (_, source, target) in &edges {
            let next = match direction {
                Spotlight::Downstream if source == &current && cone.insert(target.clone()) => {
                    Some(target.clone())
                }
                Spotlight::Upstream if target == &current && cone.insert(source.clone()) => {
                    Some(source.clone())
                }
                Spotlight::Both => {
                    if source == &current && cone.insert(target.clone()) {
                        Some(target.clone())
                    } else if target == &current && cone.insert(source.clone()) {
                        Some(source.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(n) = next {
                queue.push_back(n);
            }
        }
    }

    for (id, source, target) in &edges {
        let in_cone = cone.contains(source) && cone.contains(target);
        flow.set_edge_animated(id, in_cone);
        if in_cone {
            super::build::apply_edge_role(flow, id, Some(Role::Accent), theme);
        } else {
            super::build::apply_edge_role(flow, id, Some(Role::Secondary), theme);
        }
    }

    let ids: Vec<String> = flow.nodes().map(|n| n.id.clone()).collect();
    for id in ids {
        let role = if cone.contains(&id) {
            Role::Accent
        } else {
            Role::Secondary
        };
        super::build::apply_node_role(flow, &id, Some(role), theme);
    }
}
