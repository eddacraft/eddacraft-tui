//! Typed representation of the `@json-render/core` flat element spec format.
//!
//! The spec format and overall approach were inspired by Vercel’s json-render
//! product. These Rust types and the accompanying TUI engine were developed by
//! the eddacraft team as part of the Anvil project.
//!
//! A json-render spec is a *flat* element graph: rather than nesting component
//! objects, every element lives in a single `elements` map addressed by a
//! string id, and parent/child relationships are expressed as id references
//! in each element's [`children`](Element::children) list. A
//! [`root`](RenderSpec::root) id names the entry point.
//!
//! ```json
//! {
//!   "title": "Gate Summary",
//!   "version": "1.0",
//!   "root": "page",
//!   "elements": {
//!     "page":  { "type": "Stack",   "props": { "gap": "lg" }, "children": ["title"] },
//!     "title": { "type": "Heading", "props": { "children": "Gate Run Summary", "level": 2 }, "children": [] }
//!   }
//! }
//! ```
//!
//! The wire-format contract is owned by `@json-render/core` (pinned by
//! `@eddacraft/render` on the web side). These Rust types mirror it so the
//! same authored specs can be rendered in a terminal. No Anvil-specific
//! extensions are added to the core structure.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A single property value on an [`Element`].
///
/// json-render props are component-specific and heterogeneous — a `value` prop
/// is a string on one component and a number on another, `columns` is a number
/// or an array — so the spec layer keeps them as arbitrary JSON. Per-component
/// prop type-checking is the catalogue's job (web-side `Zod` schemas), not the
/// parser's; [`validate`](crate::json_render::validate) only checks that the
/// component *type* is registered.
///
/// This is a deliberate thin alias for [`serde_json::Value`] rather than a
/// closed enum: the renderer (a later TUIDASH item) consumes props *as*
/// `serde_json::Value`, and modelling open per-component props as a fixed Rust
/// enum would be wrong. The `serde_json` dependency is therefore part of the
/// public API of the `json-render` feature — downstream callers that name this
/// type depend on `serde_json` directly.
pub type PropValue = Value;

/// The `props` bag of an [`Element`]: a map of property name to [`PropValue`].
///
/// Like [`PropValue`], this is an intentional alias over [`serde_json::Map`].
pub type Props = Map<String, Value>;

/// A render spec — a flat map of [`Element`]s addressed by id, with a
/// designated [`root`](RenderSpec::root) entry point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSpec {
    /// Human-readable title for the spec/surface.
    pub title: String,
    /// Optional longer description. Omitted from output when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Spec format version (e.g. `"1.0"`).
    pub version: String,
    /// Id of the root [`Element`] within [`elements`](RenderSpec::elements).
    pub root: String,
    /// All elements addressed by id. Map order is not significant for
    /// rendering: traversal is driven by [`root`](RenderSpec::root) and each
    /// element's [`children`](Element::children). A [`BTreeMap`] is used so
    /// re-serialisation and [`validate`](crate::json_render::validate) error
    /// ordering are deterministic (sorted by id), at the cost of not preserving
    /// the authored key order — which is fine because the format does not
    /// ascribe meaning to it.
    pub elements: BTreeMap<String, Element>,
}

/// A single component instance in a [`RenderSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    /// Catalogue component type name, e.g. `"Stack"` or `"MetricCard"`.
    ///
    /// Serialised as `type` to match the json-render wire format; `type` is a
    /// Rust keyword, so the field is named `component_type`.
    #[serde(rename = "type")]
    pub component_type: String,
    /// Component props. Heterogeneous per component — see [`PropValue`].
    /// Defaults to empty when the key is absent.
    #[serde(default)]
    pub props: Props,
    /// Ids of child elements, in render order. Empty for leaf components.
    #[serde(default)]
    pub children: Vec<String>,
    /// Optional visibility condition.
    ///
    /// Authored specs omit this field; the web-side `Zod` normaliser injects an
    /// in-memory `null` only to satisfy its schema. Accordingly JSON `null`
    /// collapses to `None` here and is omitted on re-serialisation — a one-way
    /// normalisation that matches current `@eddacraft/render` behaviour (where
    /// `null` ≡ absent ≡ "always visible"), **not** a byte-identical round-trip
    /// of an explicit `null`. A non-`null` condition expression is preserved
    /// verbatim and round-trips unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<Value>,
}

impl RenderSpec {
    /// Look up an element by id.
    #[must_use]
    pub fn element(&self, id: &str) -> Option<&Element> {
        self.elements.get(id)
    }

    /// The root element, if [`root`](RenderSpec::root) resolves to one.
    ///
    /// A `None` here is the [`MissingRoot`](crate::json_render::ValidationError::MissingRoot)
    /// condition that [`validate`](crate::json_render::validate) reports.
    #[must_use]
    pub fn root_element(&self) -> Option<&Element> {
        self.element(&self.root)
    }
}

/// Parse a json-render spec from a JSON string.
///
/// Duplicate object keys follow serde/JSON semantics (last value wins): a
/// repeated element id in `elements`, or a repeated prop key, silently keeps
/// the last occurrence. No input-size limit is imposed here; `serde_json` does
/// cap nesting depth (returning an error rather than overflowing the stack), so
/// callers parsing genuinely untrusted input should still bound the input
/// length themselves.
///
/// # Errors
/// Returns the underlying [`serde_json::Error`] when `json` is not a
/// structurally valid spec (bad JSON, or a missing required field such as
/// `title` / `version` / `root` / `elements`). Catalogue and reference checks
/// are separate — see [`validate`](crate::json_render::validate).
pub fn parse(json: &str) -> Result<RenderSpec, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serialise a spec back to pretty-printed JSON.
///
/// # Errors
/// Returns the underlying [`serde_json::Error`] on serialisation failure
/// (effectively unreachable for an in-memory [`RenderSpec`]).
pub fn to_json_pretty(spec: &RenderSpec) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A compact spec exercising every prop shape the real templates use:
    /// string / number / bool props, an array-of-strings, an array-of-arrays
    /// (table rows), nested children references, and a leaf element.
    const SAMPLE: &str = r#"{
      "title": "Sample",
      "description": "covers every prop shape",
      "version": "1.0",
      "root": "page",
      "elements": {
        "page":   { "type": "Stack",   "props": { "direction": "vertical", "gap": "lg" }, "children": ["heading", "grid", "table"] },
        "heading":{ "type": "Heading", "props": { "children": "Title", "level": 2 }, "children": [] },
        "grid":   { "type": "Grid",    "props": { "columns": 3, "dense": true }, "children": ["metric"] },
        "metric": { "type": "MetricCard", "props": { "label": "Score", "value": "92", "trend": "up", "format": "number" }, "children": [] },
        "table":  { "type": "Table",   "props": { "columns": ["A", "B"], "rows": [["1", "2"], ["3", "4"]] }, "children": [] }
      }
    }"#;

    #[test]
    fn parse_reads_top_level_fields() {
        let spec = parse(SAMPLE).expect("parse");
        assert_eq!(spec.title, "Sample");
        assert_eq!(spec.description.as_deref(), Some("covers every prop shape"));
        assert_eq!(spec.version, "1.0");
        assert_eq!(spec.root, "page");
        assert_eq!(spec.elements.len(), 5);
    }

    #[test]
    fn root_and_child_lookups_resolve() {
        let spec = parse(SAMPLE).expect("parse");
        let root = spec.root_element().expect("root resolves");
        assert_eq!(root.component_type, "Stack");
        assert_eq!(root.children, ["heading", "grid", "table"]);
        assert_eq!(
            spec.element("metric").expect("metric").component_type,
            "MetricCard"
        );
        assert!(spec.element("missing").is_none());
    }

    #[test]
    fn heterogeneous_prop_shapes_round_trip() {
        let spec = parse(SAMPLE).expect("parse");
        let grid = spec.element("grid").expect("grid");
        // number prop
        assert_eq!(grid.props.get("columns"), Some(&json!(3)));
        // bool prop
        assert_eq!(grid.props.get("dense"), Some(&json!(true)));
        let table = spec.element("table").expect("table");
        // array-of-strings prop
        assert_eq!(table.props.get("columns"), Some(&json!(["A", "B"])));
        // array-of-arrays prop
        assert_eq!(
            table.props.get("rows"),
            Some(&json!([["1", "2"], ["3", "4"]]))
        );
        // `props.children` (a string) is distinct from the element-level
        // `children` id list (a Vec<String>).
        let heading = spec.element("heading").expect("heading");
        assert_eq!(heading.props.get("children"), Some(&json!("Title")));
        assert!(heading.children.is_empty());
    }

    #[test]
    fn semantic_round_trip_is_stable() {
        let spec = parse(SAMPLE).expect("parse");
        let reserialised = to_json_pretty(&spec).expect("serialise");
        let reparsed = parse(&reserialised).expect("reparse");
        assert_eq!(spec, reparsed);
    }

    #[test]
    fn missing_required_field_is_an_error() {
        // no `root`
        let err = parse(r#"{ "title": "x", "version": "1.0", "elements": {} }"#);
        assert!(err.is_err());
    }

    #[test]
    fn non_string_children_entries_are_a_parse_error() {
        // `children` is a list of element-id strings; non-strings must not be
        // silently coerced.
        let err = parse(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": { "a": { "type": "Stack", "children": [1, null] } } }"#,
        );
        assert!(err.is_err());
    }

    #[test]
    fn duplicate_element_id_is_last_wins() {
        // Documents serde/JSON last-wins for a repeated `elements` key: the
        // first `dup` (a Heading) is dropped, the last (a Text) survives.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "dup",
                 "elements": {
                   "dup": { "type": "Heading", "props": { "level": 1 }, "children": [] },
                   "dup": { "type": "Text",    "props": {}, "children": [] }
                 } }"#,
        )
        .expect("parse");
        assert_eq!(spec.elements.len(), 1);
        assert_eq!(spec.element("dup").expect("dup").component_type, "Text");
    }

    #[test]
    fn optional_props_and_children_default_to_empty() {
        // An element may omit `props` and `children` entirely.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": { "a": { "type": "Separator" } } }"#,
        )
        .expect("parse");
        let a = spec.element("a").expect("a");
        assert!(a.props.is_empty());
        assert!(a.children.is_empty());
        assert!(a.visible.is_none());
    }

    #[test]
    fn absent_visible_is_not_serialised() {
        let spec = parse(SAMPLE).expect("parse");
        let out = to_json_pretty(&spec).expect("serialise");
        assert!(
            !out.contains("visible"),
            "absent `visible` must not be emitted"
        );
        assert!(!out.contains("description\": null"));
    }

    #[test]
    fn explicit_null_visible_collapses_to_absent() {
        // serde maps JSON `null` for an `Option<T>` to `None`. Authored specs
        // never carry `visible`; the web validator injects `null` only as an
        // in-memory "always visible" marker, so `null` ≡ absent and collapsing
        // it to `None` (and omitting it on output) is lossless.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": { "a": { "type": "Text", "props": {}, "children": [], "visible": null } } }"#,
        )
        .expect("parse");
        assert_eq!(spec.element("a").expect("a").visible, None);
        let out = to_json_pretty(&spec).expect("serialise");
        assert!(!out.contains("visible"));
    }

    #[test]
    fn non_null_visible_condition_is_preserved() {
        // A real visibility condition (anything other than `null`) is kept
        // verbatim and round-trips.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "a",
                 "elements": { "a": { "type": "Text", "props": {}, "children": [], "visible": { "field": "showAdvanced" } } } }"#,
        )
        .expect("parse");
        assert_eq!(
            spec.element("a").expect("a").visible,
            Some(json!({ "field": "showAdvanced" }))
        );
        let out = to_json_pretty(&spec).expect("serialise");
        assert!(out.contains("visible"));
        assert_eq!(spec, parse(&out).expect("reparse"));
    }
}
