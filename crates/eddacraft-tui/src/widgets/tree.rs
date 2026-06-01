//! Hierarchical tree widget with expand/collapse and a movable cursor.
//!
//! Inspired by Cursive/Textual `Tree`. The widget is purely presentational;
//! application code owns the [`TreeNode`] data and a [`TreeState`] holding the
//! cursor and the set of expanded node IDs.
//!
//! ```rust
//! # use eddacraft_tui::widgets::tree::{Tree, TreeNode, TreeState};
//! # use eddacraft_tui::theme::EddaCraftTheme;
//! # let theme = EddaCraftTheme;
//! let nodes = vec![TreeNode::branch(
//!     "root",
//!     "Root",
//!     vec![TreeNode::leaf("a", "Alpha"), TreeNode::leaf("b", "Beta")],
//! )];
//! let mut state = TreeState::default();
//! state.expand("root");
//! let _ = Tree::new(&theme, &nodes);
//! # let _ = state;
//! ```

use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    #[must_use]
    pub fn leaf(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn branch(
        id: impl Into<String>,
        label: impl Into<String>,
        children: Vec<TreeNode>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children,
        }
    }

    #[must_use]
    pub fn is_branch(&self) -> bool {
        !self.children.is_empty()
    }
}

impl Drop for TreeNode {
    /// Dismantle the subtree iteratively so dropping a deeply-nested tree
    /// cannot overflow the stack.
    ///
    /// The compiler-derived drop is recursive (each node drops its
    /// children, which drop theirs, …), so a long chain blows the stack on
    /// teardown — fatal on Windows' 1 MiB default thread stack, which a
    /// deep real tree (or the `deep_chain_does_not_overflow_stack` test)
    /// hits even though the render walker itself is iterative. Draining
    /// every descendant into a flat work-list keeps teardown bounded to a
    /// single stack frame.
    fn drop(&mut self) {
        let mut pending: Vec<TreeNode> = std::mem::take(&mut self.children);
        while let Some(mut node) = pending.pop() {
            // Move this node's children up to the work-list before it drops,
            // so its own (now-empty) drop does not recurse.
            pending.append(&mut node.children);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TreeState {
    pub(crate) expanded: HashSet<String>,
    pub(crate) cursor: usize,
}

impl TreeState {
    /// Build a state from a pre-existing set of expanded ids. Useful when the
    /// application persists view state.
    #[must_use]
    pub fn from_expanded(ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            expanded: ids.into_iter().collect(),
            cursor: 0,
        }
    }

    /// Current cursor index into the visible-node list.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Snapshot of the expanded-node ids — useful for persistence.
    pub fn expanded_ids(&self) -> impl Iterator<Item = &str> {
        self.expanded.iter().map(String::as_str)
    }

    pub fn expand(&mut self, id: impl Into<String>) {
        self.expanded.insert(id.into());
    }

    pub fn collapse(&mut self, id: &str) {
        self.expanded.remove(id);
    }

    pub fn toggle(&mut self, id: &str) {
        if !self.expanded.remove(id) {
            self.expanded.insert(id.to_string());
        }
    }

    #[must_use]
    pub fn is_expanded(&self, id: &str) -> bool {
        self.expanded.contains(id)
    }

    /// Move the cursor up through the currently visible nodes.
    pub fn move_up(&mut self, visible: usize) {
        if visible == 0 {
            self.cursor = 0;
            return;
        }
        if self.cursor == 0 {
            self.cursor = visible - 1;
        } else {
            self.cursor -= 1;
        }
    }

    /// Move the cursor down through the currently visible nodes.
    pub fn move_down(&mut self, visible: usize) {
        if visible == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = (self.cursor + 1) % visible;
    }
}

pub struct Tree<'a, T: Theme> {
    theme: &'a T,
    nodes: &'a [TreeNode],
}

struct Visible<'a> {
    node: &'a TreeNode,
    depth: usize,
}

/// Iterative walker that produces the visible-node list without recursing on
/// the input tree. Recursion would stack-overflow on attacker-controlled or
/// pathologically deep input (e.g. parsed filesystem paths or JSON).
fn visible_nodes<'a>(
    nodes: &'a [TreeNode],
    expanded: &HashSet<String>,
    out: &mut Vec<Visible<'a>>,
) {
    let mut stack: Vec<(&[TreeNode], usize, usize)> = vec![(nodes, 0, 0)];
    while let Some((slice, index, depth)) = stack.pop() {
        if index >= slice.len() {
            continue;
        }
        let node = &slice[index];
        out.push(Visible { node, depth });
        // Resume with the next sibling at this depth.
        stack.push((slice, index + 1, depth));
        // Then descend into expanded branches before the next sibling.
        if node.is_branch() && expanded.contains(&node.id) {
            stack.push((&node.children, 0, depth + 1));
        }
    }
}

// Called from inside `debug_assert!` in `Tree::new`. Must NOT be cfg-gated on
// `debug_assertions`: the macro arg is name-resolved unconditionally, so a
// cfg-gated definition fails to compile when downstream consumers build us
// with `debug_assertions = false` (release / `--release-napi` / etc.).
// See issue #29.
fn ids_are_unique(nodes: &[TreeNode]) -> bool {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut stack: Vec<(&[TreeNode], usize)> = vec![(nodes, 0)];
    while let Some((slice, index)) = stack.pop() {
        if index >= slice.len() {
            continue;
        }
        let node = &slice[index];
        if !seen.insert(node.id.as_str()) {
            return false;
        }
        stack.push((slice, index + 1));
        if !node.children.is_empty() {
            stack.push((&node.children, 0));
        }
    }
    true
}

impl<'a, T: Theme> Tree<'a, T> {
    /// Construct a `Tree` over the supplied nodes. Panics in debug builds if
    /// any two nodes share an `id` — `TreeState.expanded` is keyed on `id`,
    /// so duplicates would cause `expand` / `collapse` / `toggle` to act on
    /// every matching node simultaneously.
    pub fn new(theme: &'a T, nodes: &'a [TreeNode]) -> Self {
        debug_assert!(
            ids_are_unique(nodes),
            "TreeNode ids must be unique across the entire tree",
        );
        Self { theme, nodes }
    }

    /// Number of visible rows given the supplied state — useful for clamping
    /// the cursor or sizing scroll regions.
    #[must_use]
    pub fn visible_count(&self, state: &TreeState) -> usize {
        let mut buf = Vec::new();
        visible_nodes(self.nodes, &state.expanded, &mut buf);
        buf.len()
    }

    /// ID of the node currently under the cursor, if any.
    #[must_use]
    pub fn selected_id(&self, state: &TreeState) -> Option<String> {
        let mut buf = Vec::new();
        visible_nodes(self.nodes, &state.expanded, &mut buf);
        buf.get(state.cursor).map(|v| v.node.id.clone())
    }
}

impl<T: Theme> StatefulWidget for Tree<'_, T> {
    type State = TreeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        buf.set_style(area, self.theme.base());

        let mut visible = Vec::new();
        visible_nodes(self.nodes, &state.expanded, &mut visible);

        if !visible.is_empty() && state.cursor >= visible.len() {
            state.cursor = visible.len() - 1;
        }

        let usable_rows = usize::from(area.height);
        let cursor = state.cursor;
        let scroll = if cursor >= usable_rows {
            cursor + 1 - usable_rows
        } else {
            0
        };

        let label_style = self.theme.base();
        let highlight_style = self.theme.highlighted();
        let branch_style = self.theme.title();
        let leaf_style = self.theme.disabled();

        for (row_index, vis) in visible.iter().enumerate().skip(scroll).take(usable_rows) {
            let y = area
                .y
                .saturating_add(u16::try_from(row_index - scroll).unwrap_or(0));
            let indent = "  ".repeat(vis.depth);
            let glyph = if vis.node.is_branch() {
                if state.expanded.contains(&vis.node.id) {
                    "▼ "
                } else {
                    "▶ "
                }
            } else {
                "· "
            };
            let glyph_style = if vis.node.is_branch() {
                branch_style
            } else {
                leaf_style
            };

            let mut line = Line::from(vec![
                Span::styled(indent, label_style),
                Span::styled(glyph, glyph_style),
                Span::styled(vis.node.label.clone(), label_style),
            ]);
            if row_index == cursor {
                line = line.style(highlight_style);
            }
            line.render(Rect::new(area.x, y, area.width, 1), buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::EddaCraftTheme;

    fn sample_tree() -> Vec<TreeNode> {
        vec![
            TreeNode::branch(
                "root",
                "Root",
                vec![
                    TreeNode::leaf("a", "Alpha"),
                    TreeNode::branch("nested", "Nested", vec![TreeNode::leaf("b", "Beta")]),
                ],
            ),
            TreeNode::leaf("c", "Gamma"),
        ]
    }

    #[test]
    fn collapsed_root_shows_only_top_level() {
        let theme = EddaCraftTheme;
        let nodes = sample_tree();
        let state = TreeState::default();
        let tree = Tree::new(&theme, &nodes);
        assert_eq!(tree.visible_count(&state), 2); // Root, Gamma
    }

    #[test]
    fn expand_reveals_children() {
        let theme = EddaCraftTheme;
        let nodes = sample_tree();
        let mut state = TreeState::default();
        state.expand("root");
        let tree = Tree::new(&theme, &nodes);
        assert_eq!(tree.visible_count(&state), 4); // Root, Alpha, Nested, Gamma
    }

    #[test]
    fn nested_expand_reveals_grandchildren() {
        let theme = EddaCraftTheme;
        let nodes = sample_tree();
        let mut state = TreeState::default();
        state.expand("root");
        state.expand("nested");
        let tree = Tree::new(&theme, &nodes);
        assert_eq!(tree.visible_count(&state), 5);
    }

    #[test]
    fn toggle_round_trips() {
        let mut state = TreeState::default();
        state.toggle("root");
        assert!(state.is_expanded("root"));
        state.toggle("root");
        assert!(!state.is_expanded("root"));
    }

    #[test]
    fn cursor_navigation_wraps() {
        let mut state = TreeState::default();
        state.move_down(3);
        assert_eq!(state.cursor, 1);
        state.move_down(3);
        state.move_down(3);
        assert_eq!(state.cursor, 0);
        state.move_up(3);
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn selected_id_resolves_visible_cursor() {
        let theme = EddaCraftTheme;
        let nodes = sample_tree();
        let mut state = TreeState::default();
        state.expand("root");
        state.cursor = 1; // Alpha
        let tree = Tree::new(&theme, &nodes);
        assert_eq!(tree.selected_id(&state), Some("a".to_string()));
    }

    #[test]
    fn render_displays_labels_with_indent_and_glyph() {
        let theme = EddaCraftTheme;
        let nodes = sample_tree();
        let mut state = TreeState::default();
        state.expand("root");
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(Tree::new(&theme, &nodes), area, &mut buf, &mut state);
        let row0: String = (0..30).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert!(row0.contains("▼"), "row0={row0:?}");
        assert!(row0.contains("Root"), "row0={row0:?}");
        let row1: String = (0..30).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert!(row1.contains("Alpha"), "row1={row1:?}");
    }

    #[test]
    fn render_scrolls_when_cursor_below_viewport() {
        let theme = EddaCraftTheme;
        let nodes = vec![
            TreeNode::leaf("1", "one"),
            TreeNode::leaf("2", "two"),
            TreeNode::leaf("3", "three"),
            TreeNode::leaf("4", "four"),
            TreeNode::leaf("5", "five"),
        ];
        let mut state = TreeState {
            cursor: 4,
            ..TreeState::default()
        };
        let area = Rect::new(0, 0, 10, 2);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(Tree::new(&theme, &nodes), area, &mut buf, &mut state);
        // Cursor was at index 4, visible rows are 2 -> rows 3 and 4 visible.
        let row1: String = (0..10).map(|x| buf[(x, 1)].symbol().to_string()).collect();
        assert!(row1.contains("five"), "row1={row1:?}");
    }

    #[test]
    fn cursor_clamped_to_visible_count() {
        let theme = EddaCraftTheme;
        let nodes = sample_tree();
        // Set cursor beyond visible (only 2 visible without expansion).
        let mut state = TreeState {
            cursor: 99,
            ..TreeState::default()
        };
        let area = Rect::new(0, 0, 10, 2);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(Tree::new(&theme, &nodes), area, &mut buf, &mut state);
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn deep_chain_does_not_overflow_stack() {
        // Construct a 10,000-deep linear chain. Recursive walkers would blow
        // the host process stack here; the iterative walker is bounded by
        // heap.
        let depth = 10_000;
        let mut current = TreeNode::leaf(format!("n{depth}"), "leaf");
        for i in (0..depth).rev() {
            current = TreeNode::branch(format!("n{i}"), "branch", vec![current]);
        }
        let nodes = vec![current];
        let theme = EddaCraftTheme;
        let mut state = TreeState::default();
        for i in 0..depth {
            state.expand(format!("n{i}"));
        }
        let tree = Tree::new(&theme, &nodes);
        // visible_count must include the root + every expanded ancestor +
        // the leaf at the bottom.
        assert_eq!(tree.visible_count(&state), depth + 1);
    }

    #[test]
    fn from_expanded_round_trips_state() {
        let state = TreeState::from_expanded(["root".to_string(), "nested".to_string()]);
        assert!(state.is_expanded("root"));
        assert!(state.is_expanded("nested"));
        assert!(!state.is_expanded("missing"));
    }

    #[test]
    fn empty_nodes_renders_nothing() {
        let theme = EddaCraftTheme;
        let nodes: Vec<TreeNode> = Vec::new();
        let mut state = TreeState::default();
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        StatefulWidget::render(Tree::new(&theme, &nodes), area, &mut buf, &mut state);
        for y in 0..3 {
            for x in 0..10 {
                assert_eq!(buf[(x, y)].symbol(), " ");
            }
        }
    }
}
