//! Diff two edge lists, keeping removed nodes as ghosts (TUIN-018).

use std::collections::BTreeSet;

use rataflow::Flow;

use super::{EdgeSpec, NodeSpec, themed_from_specs};
use crate::theme::{Role, Theme};

/// Layout the union of `before` and `after`. Added edges are `Success`,
/// removed edges `Error` (still occupying space), unchanged edges muted.
///
/// # Stability
///
/// **experimental** (TUIN-018).
pub fn themed_from_diff<T: Theme + ?Sized>(
    before: &[(&str, &str)],
    after: &[(&str, &str)],
    theme: &T,
) -> Result<Flow, rataflow::Error> {
    let before_set: BTreeSet<(&str, &str)> = before.iter().copied().collect();
    let after_set: BTreeSet<(&str, &str)> = after.iter().copied().collect();

    let mut node_ids: BTreeSet<&str> = BTreeSet::new();
    for (a, b) in before.iter().chain(after.iter()) {
        node_ids.insert(*a);
        node_ids.insert(*b);
    }
    let after_nodes: BTreeSet<&str> = after.iter().flat_map(|(a, b)| [*a, *b]).collect();

    let nodes: Vec<NodeSpec> = node_ids
        .iter()
        .map(|id| {
            let spec = NodeSpec::new(*id, *id);
            if after_nodes.contains(id) {
                spec
            } else {
                spec.with_role(Role::Error)
            }
        })
        .collect();

    let mut specs: Vec<EdgeSpec> = Vec::new();
    let union: BTreeSet<(&str, &str)> = before_set.union(&after_set).copied().collect();
    for (from, to) in union {
        let in_before = before_set.contains(&(from, to));
        let in_after = after_set.contains(&(from, to));
        let spec = EdgeSpec::new(from, to);
        let spec = if in_after && !in_before {
            spec.with_role(Role::Success)
        } else if in_before && !in_after {
            spec.with_role(Role::Error)
        } else {
            spec.with_role(Role::Secondary)
        };
        specs.push(spec);
    }

    themed_from_specs(&nodes, &specs, theme)
}
