//! json-render spec engine (feature `json-render`).
//!
//! This is the generic, Anvil-agnostic engine for the `@json-render/core` flat
//! element spec format, per [ADR-054]: parse a JSON dashboard spec into typed
//! Rust structures and validate it against a component [`Catalog`]. The
//! [`TuiComponent`] trait and [`TuiRegistry`] map component type names to
//! renderers on top of these types; the tree walker and Ratatui widget mappings
//! build on them in later TUIDASH work items.
//!
//! ```
//! use eddacraft_tui::json_render::{self, Catalog};
//!
//! let json = r#"{
//!   "title": "Demo", "version": "1.0", "root": "page",
//!   "elements": {
//!     "page":  { "type": "Stack",   "props": {}, "children": ["hi"] },
//!     "hi":    { "type": "Heading", "props": { "children": "Hello", "level": 1 }, "children": [] }
//!   }
//! }"#;
//!
//! let spec = json_render::parse(json).expect("valid spec");
//! json_render::validate(&spec, &Catalog::base()).expect("known components");
//! assert_eq!(spec.root_element().unwrap().component_type, "Stack");
//! ```
//!
//! [ADR-054]: the json-render TUI engine home decision — generic engine in
//! `eddacraft-tui` behind this feature, Anvil catalogue + surface in `anvil-tui`.

mod component;
mod registry;
mod spec;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use component::TuiComponent;
pub use registry::TuiRegistry;
pub use spec::{Element, PropValue, Props, RenderSpec, parse, to_json_pretty};

/// The set of component type names a [`RenderSpec`] is allowed to reference.
///
/// A spec that names a component outside its catalogue cannot be rendered, so
/// [`validate`] rejects it. [`Catalog::base`] seeds the catalogue shipped by
/// `@eddacraft/render` (the shadcn built-ins plus the custom Anvil
/// components); downstream surfaces extend it with their own component names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    names: BTreeSet<String>,
}

/// Component names registered by the `@eddacraft/render` base catalogue
/// (`src/catalog-registry.ts`): the shadcn built-ins plus the custom Anvil
/// `MetricCard` / `StatusBadge`.
const BASE_COMPONENTS: [&str; 12] = [
    // shadcn built-ins
    "Card",
    "Stack",
    "Grid",
    "Heading",
    "Text",
    "Badge",
    "Separator",
    "Table",
    "Alert",
    "Progress",
    // custom Anvil components
    "MetricCard",
    "StatusBadge",
];

impl Catalog {
    /// The base catalogue shipped by `@eddacraft/render`.
    #[must_use]
    pub fn base() -> Self {
        Self::from_names(BASE_COMPONENTS)
    }

    /// Build a catalogue from an explicit set of component names.
    #[must_use]
    pub fn from_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            names: names.into_iter().map(Into::into).collect(),
        }
    }

    /// Register an additional component name. Returns `true` if it was newly
    /// added (mirrors [`BTreeSet::insert`]).
    pub fn insert(&mut self, name: impl Into<String>) -> bool {
        self.names.insert(name.into())
    }

    /// Whether `name` is a registered component type.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    /// All registered component names, sorted.
    #[must_use = "iterator is lazy and does nothing unless consumed"]
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }
}

/// A reason a [`RenderSpec`] failed [`validate`].
///
/// `#[non_exhaustive]`: more checks (e.g. unreachable elements) may be added in
/// later work, so downstream `match`es must include a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationError {
    /// An element's `type` is not in the catalogue.
    UnknownComponent {
        /// The element id carrying the unknown type.
        element_id: String,
        /// The unregistered component type name.
        component_type: String,
    },
    /// [`RenderSpec::root`] does not resolve to an element.
    MissingRoot {
        /// The dangling root id.
        root: String,
    },
    /// A `children` entry references an element id that does not exist.
    DanglingChild {
        /// The element whose `children` list holds the bad reference.
        parent_id: String,
        /// The referenced id that is absent from `elements`.
        child_id: String,
    },
    /// The `children` graph contains a cycle. A cycle would make a recursive
    /// tree walker loop forever, so it is rejected here rather than left for
    /// the renderer to guard against.
    CyclicReference {
        /// The element ids forming the cycle, in traversal order, with the
        /// repeated id appearing at both ends (e.g. `["a", "b", "a"]`).
        cycle: Vec<String>,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownComponent {
                element_id,
                component_type,
            } => write!(
                f,
                "element `{element_id}` uses unknown component type `{component_type}`"
            ),
            Self::MissingRoot { root } => {
                write!(f, "root `{root}` does not resolve to an element")
            }
            Self::DanglingChild {
                parent_id,
                child_id,
            } => write!(
                f,
                "element `{parent_id}` references missing child `{child_id}`"
            ),
            Self::CyclicReference { cycle } => {
                write!(f, "cyclic `children` reference: {}", cycle.join(" -> "))
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validate a parsed spec against a component [`Catalog`].
///
/// The reference and catalogue checks accumulate (not fail-fast), so a caller
/// sees every one at once:
///
/// - the [`root`](RenderSpec::root) id resolves to an element;
/// - every element's `type` is registered in `catalog`;
/// - every `children` id reference resolves to an element.
///
/// Finally the `children` graph is checked for cycles; the **first** cycle found
/// is reported as a single [`ValidationError::CyclicReference`] (cycle detection
/// stops at the first back-edge rather than enumerating every distinct cycle).
///
/// This is structural/catalogue validation only; per-component prop schemas are
/// owned by the (web-side) catalogue and are out of scope here.
///
/// A clean result means the graph can be walked from the root without looping.
/// Renderers should still treat that as belt-and-braces and keep their own
/// depth guard, since callers are not obliged to run `validate` first.
///
/// # Errors
/// Returns every [`ValidationError`] found, or `Ok(())` when the spec is clean.
pub fn validate(spec: &RenderSpec, catalog: &Catalog) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if !spec.elements.contains_key(&spec.root) {
        errors.push(ValidationError::MissingRoot {
            root: spec.root.clone(),
        });
    }

    for (id, element) in &spec.elements {
        if !catalog.contains(&element.component_type) {
            errors.push(ValidationError::UnknownComponent {
                element_id: id.clone(),
                component_type: element.component_type.clone(),
            });
        }
        for child in &element.children {
            if !spec.elements.contains_key(child) {
                errors.push(ValidationError::DanglingChild {
                    parent_id: id.clone(),
                    child_id: child.clone(),
                });
            }
        }
    }

    if let Some(cycle) = detect_cycle(spec) {
        errors.push(ValidationError::CyclicReference { cycle });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Find one cycle in the `children` graph, or `None` if it is acyclic.
///
/// Iterative three-colour depth-first search — iterative rather than recursive
/// so a deep (non-cyclic) chain cannot overflow the stack, which would itself
/// breach the "must not panic" constraint. Only resolvable child references are
/// followed (dangling ones are reported separately), so this never indexes a
/// missing key. Returns the cycle members in traversal order with the repeated
/// id at both ends.
fn detect_cycle(spec: &RenderSpec) -> Option<Vec<String>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        InProgress,
        Done,
    }

    let mut marks: BTreeMap<&str, Mark> = BTreeMap::new();

    for start in spec.elements.keys() {
        if marks.contains_key(start.as_str()) {
            continue;
        }
        // Explicit DFS stack of (node, next-child-index).
        let mut stack: Vec<(&str, usize)> = vec![(start.as_str(), 0)];
        marks.insert(start.as_str(), Mark::InProgress);

        while let Some(&(node, idx)) = stack.last() {
            let children = &spec.elements[node].children;
            if let Some(child) = children.get(idx) {
                if let Some((_, next)) = stack.last_mut() {
                    *next = idx + 1;
                }
                // Skip dangling references (reported elsewhere).
                if !spec.elements.contains_key(child) {
                    continue;
                }
                match marks.get(child.as_str()) {
                    Some(Mark::InProgress) => {
                        // Back-edge: build the cycle from where `child` sits on
                        // the current path down to the back-edge, then close it.
                        let mut cycle: Vec<String> = stack
                            .iter()
                            .skip_while(|(n, _)| *n != child.as_str())
                            .map(|(n, _)| (*n).to_owned())
                            .collect();
                        cycle.push(child.clone());
                        return Some(cycle);
                    }
                    Some(Mark::Done) => {}
                    None => {
                        marks.insert(child.as_str(), Mark::InProgress);
                        stack.push((child.as_str(), 0));
                    }
                }
            } else {
                marks.insert(node, Mark::Done);
                stack.pop();
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(json: &str) -> RenderSpec {
        parse(json).expect("spec parses")
    }

    #[test]
    fn base_catalogue_has_the_twelve_known_components() {
        let catalog = Catalog::base();
        assert_eq!(catalog.names().count(), BASE_COMPONENTS.len());
        for name in BASE_COMPONENTS {
            assert!(catalog.contains(name), "{name} should be registered");
        }
        assert!(!catalog.contains("HeatMap"));
    }

    #[test]
    fn valid_spec_passes() {
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": {
                   "page": { "type": "Stack", "props": {}, "children": ["m"] },
                   "m":    { "type": "MetricCard", "props": {}, "children": [] }
                 } }"#,
        );
        assert!(validate(&spec, &Catalog::base()).is_ok());
    }

    #[test]
    fn unknown_component_is_rejected() {
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": {
                   "page": { "type": "Stack", "props": {}, "children": ["h"] },
                   "h":    { "type": "HeatMap", "props": {}, "children": [] }
                 } }"#,
        );
        let errors = validate(&spec, &Catalog::base()).expect_err("HeatMap is unknown");
        assert_eq!(
            errors,
            vec![ValidationError::UnknownComponent {
                element_id: "h".to_string(),
                component_type: "HeatMap".to_string(),
            }]
        );
    }

    #[test]
    fn missing_root_is_rejected() {
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "nope",
                 "elements": { "page": { "type": "Stack", "props": {}, "children": [] } } }"#,
        );
        let errors = validate(&spec, &Catalog::base()).expect_err("root is dangling");
        assert!(errors.contains(&ValidationError::MissingRoot {
            root: "nope".to_string()
        }));
    }

    #[test]
    fn dangling_child_reference_is_rejected() {
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": { "page": { "type": "Stack", "props": {}, "children": ["ghost"] } } }"#,
        );
        let errors = validate(&spec, &Catalog::base()).expect_err("ghost child is missing");
        assert!(errors.contains(&ValidationError::DanglingChild {
            parent_id: "page".to_string(),
            child_id: "ghost".to_string(),
        }));
    }

    #[test]
    fn multiple_problems_are_all_reported() {
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "missing",
                 "elements": {
                   "page": { "type": "Mystery", "props": {}, "children": ["ghost"] }
                 } }"#,
        );
        let errors = validate(&spec, &Catalog::base()).expect_err("three distinct problems");
        // All three distinct variants must be present — not just any three.
        assert!(errors.contains(&ValidationError::MissingRoot {
            root: "missing".to_string()
        }));
        assert!(errors.contains(&ValidationError::UnknownComponent {
            element_id: "page".to_string(),
            component_type: "Mystery".to_string(),
        }));
        assert!(errors.contains(&ValidationError::DanglingChild {
            parent_id: "page".to_string(),
            child_id: "ghost".to_string(),
        }));
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn self_referential_root_is_a_cycle() {
        // `page` lists itself as a child — a recursive walker would loop.
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": { "page": { "type": "Stack", "props": {}, "children": ["page"] } } }"#,
        );
        let errors = validate(&spec, &Catalog::base()).expect_err("self-cycle");
        assert!(matches!(
            errors.as_slice(),
            [ValidationError::CyclicReference { cycle }] if cycle == &["page".to_string(), "page".to_string()]
        ));
    }

    #[test]
    fn mutual_cycle_is_rejected() {
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": {
                   "a": { "type": "Stack", "props": {}, "children": ["b"] },
                   "b": { "type": "Stack", "props": {}, "children": ["a"] }
                 } }"#,
        );
        let errors = validate(&spec, &Catalog::base()).expect_err("a -> b -> a");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::CyclicReference { .. }))
        );
    }

    #[test]
    fn diamond_dag_is_not_a_cycle() {
        // a -> {b, c} -> d : shared child `d` is a DAG, not a cycle.
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": {
                   "a": { "type": "Stack", "props": {}, "children": ["b", "c"] },
                   "b": { "type": "Stack", "props": {}, "children": ["d"] },
                   "c": { "type": "Stack", "props": {}, "children": ["d"] },
                   "d": { "type": "Text",  "props": {}, "children": [] }
                 } }"#,
        );
        assert!(validate(&spec, &Catalog::base()).is_ok());
    }

    #[test]
    fn deep_acyclic_chain_does_not_overflow() {
        // A long linear chain must validate without a stack overflow — proves
        // the cycle detector is iterative, not recursive.
        use std::fmt::Write as _;

        let depth = 50_000;
        let mut elements = String::new();
        for i in 0..depth {
            let child = if i + 1 < depth {
                format!("[\"n{}\"]", i + 1)
            } else {
                "[]".to_string()
            };
            if i > 0 {
                elements.push(',');
            }
            write!(
                elements,
                r#""n{i}": {{ "type": "Stack", "props": {{}}, "children": {child} }}"#
            )
            .expect("writing to a String is infallible");
        }
        let json = format!(
            r#"{{ "title": "x", "version": "1.0", "root": "n0", "elements": {{ {elements} }} }}"#
        );
        let spec = parse_ok(&json);
        assert!(validate(&spec, &Catalog::base()).is_ok());
    }

    #[test]
    fn cycle_error_renders_a_message() {
        let err = ValidationError::CyclicReference {
            cycle: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        };
        assert_eq!(err.to_string(), "cyclic `children` reference: a -> b -> a");
    }

    #[test]
    fn extended_catalogue_accepts_custom_components() {
        let spec = parse_ok(
            r#"{ "title": "x", "version": "1.0", "root": "page",
                 "elements": { "page": { "type": "FlameGraph", "props": {}, "children": [] } } }"#,
        );
        let mut catalog = Catalog::base();
        assert!(catalog.insert("FlameGraph"));
        assert!(!catalog.insert("FlameGraph")); // already present
        assert!(validate(&spec, &catalog).is_ok());
    }

    #[test]
    fn validation_error_renders_a_message() {
        let err = ValidationError::UnknownComponent {
            element_id: "h".to_string(),
            component_type: "HeatMap".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "element `h` uses unknown component type `HeatMap`"
        );
    }
}
