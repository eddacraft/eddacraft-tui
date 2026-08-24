//! Collapse over-budget graphs into a portal node (TUIN-020).

use std::collections::{BTreeMap, BTreeSet};

use rataflow::Flow;

use super::themed_from_edges;
use crate::theme::Theme;

/// Result of eliding a graph that exceeded the visible-node budget.
///
/// # Stability
///
/// **experimental** (TUIN-020).
#[derive(Debug)]
pub struct ElidedGraph {
    /// The (possibly portal-containing) flow.
    pub flow: Flow,
    /// Portal node id when anything was collapsed.
    pub portal_id: Option<String>,
    /// Node ids hidden inside the portal.
    pub collapsed: Vec<String>,
}

/// Build a themed graph, collapsing lowest-degree nodes into a portal when
/// the unique-node count exceeds `max_visible`.
///
/// # Stability
///
/// **experimental** (TUIN-020).
pub fn elide_from_edges<T: Theme + ?Sized>(
    edges: &[(&str, &str)],
    theme: &T,
    max_visible: usize,
) -> Result<ElidedGraph, rataflow::Error> {
    elide_from_edges_keeping(edges, theme, max_visible, &[])
}

/// Like [`elide_from_edges`], but `always_keep` ids are never collapsed
/// (used to expand a portal in place).
///
/// # Stability
///
/// **experimental** (TUIN-020).
pub fn elide_from_edges_keeping<T: Theme + ?Sized>(
    edges: &[(&str, &str)],
    theme: &T,
    max_visible: usize,
    always_keep: &[&str],
) -> Result<ElidedGraph, rataflow::Error> {
    let mut degree: BTreeMap<String, usize> = BTreeMap::new();
    for (a, b) in edges {
        *degree.entry((*a).to_string()).or_default() += 1;
        *degree.entry((*b).to_string()).or_default() += 1;
    }
    let all: Vec<String> = degree.keys().cloned().collect();
    if all.len() <= max_visible {
        return Ok(ElidedGraph {
            flow: themed_from_edges(edges, theme)?,
            portal_id: None,
            collapsed: Vec::new(),
        });
    }

    let keep_forced: BTreeSet<String> = always_keep.iter().map(|s| (*s).to_string()).collect();
    let mut ranked: Vec<(usize, String)> =
        degree.iter().map(|(id, deg)| (*deg, id.clone())).collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let mut keep: BTreeSet<String> = keep_forced.clone();
    for (_, id) in ranked {
        if keep.len() >= max_visible.saturating_sub(1) {
            break;
        }
        keep.insert(id);
    }

    let collapsed: Vec<String> = all.into_iter().filter(|id| !keep.contains(id)).collect();
    if collapsed.is_empty() {
        return Ok(ElidedGraph {
            flow: themed_from_edges(edges, theme)?,
            portal_id: None,
            collapsed: Vec::new(),
        });
    }

    let portal_id = format!("… {} more", collapsed.len());
    let collapsed_set: BTreeSet<&str> = collapsed.iter().map(String::as_str).collect();
    let mut rewritten: Vec<(String, String)> = Vec::new();
    for (a, b) in edges {
        let a_keep = keep.contains(*a);
        let b_keep = keep.contains(*b);
        if a_keep && b_keep {
            rewritten.push(((*a).to_string(), (*b).to_string()));
        } else if a_keep && collapsed_set.contains(b) {
            rewritten.push(((*a).to_string(), portal_id.clone()));
        } else if b_keep && collapsed_set.contains(a) {
            rewritten.push((portal_id.clone(), (*b).to_string()));
        }
    }
    let pairs: Vec<(&str, &str)> = rewritten
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    Ok(ElidedGraph {
        flow: themed_from_edges(&pairs, theme)?,
        portal_id: Some(portal_id),
        collapsed,
    })
}
