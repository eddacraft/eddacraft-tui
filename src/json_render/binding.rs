//! Data-context binding — resolve `{ "$data": "path" }` prop references against
//! live data before rendering.
//!
//! json-render specs are static, but a dashboard's values are not: a spec
//! expresses *where* a value comes from with a data reference —
//! `{ "value": { "$data": "gates.passRate" } }` — and the host resolves it
//! against the current [`DataContext`] at render time.
//!
//! Binding is a pure **spec → spec** transform ([`bind`]): it returns a new
//! [`RenderSpec`] with every `$data` reference replaced by the resolved value,
//! leaving the [tree renderer](crate::json_render::render_spec) and the
//! [`TuiComponent`](crate::json_render::TuiComponent) trait untouched (they only
//! ever see plain props). The generic path-resolution and transform live here in
//! `eddacraft-tui`; the Anvil-specific loader that builds a [`DataContext`] from
//! `.anvil/` storage lives in `anvil-tui`.
//!
//! Per the module constraint, a binding failure (missing path) resolves to JSON
//! `null` — which the components render as an em dash (`—`), never an error.

use serde_json::{Map, Value};

use crate::json_render::{Element, RenderSpec};

/// The marker key that turns a prop object into a data reference:
/// `{ "$data": "dotted.path" }`.
const DATA_REF_KEY: &str = "$data";

/// A tree of resolved values that `$data` references are looked up against.
///
/// Thin wrapper over a [`serde_json::Value`]; the host populates it (for Anvil,
/// from `.anvil/` storage) and the renderer reads it. Lookups never mutate it,
/// so one context can bind many specs.
#[derive(Debug, Clone, Default)]
pub struct DataContext {
    root: Value,
}

impl DataContext {
    /// Wrap a value tree as a context.
    #[must_use]
    pub fn new(root: Value) -> Self {
        Self { root }
    }

    /// An empty context. Every lookup misses (resolves to `null`), so a spec
    /// with data references still renders — as em dashes.
    #[must_use]
    pub fn empty() -> Self {
        Self { root: Value::Null }
    }

    /// Resolve a dotted `path` against the context.
    ///
    /// Each segment indexes an object key or, if the current node is an array
    /// and the segment parses as an integer, an array element: `"checks.0.name"`
    /// reaches into `checks[0].name`. An empty path returns the root. A segment
    /// that does not resolve yields `None`.
    #[must_use]
    pub fn resolve(&self, path: &str) -> Option<&Value> {
        let mut node = &self.root;
        if path.is_empty() {
            return Some(node);
        }
        for segment in path.split('.') {
            node = match node {
                Value::Object(map) => map.get(segment)?,
                Value::Array(items) => {
                    let index: usize = segment.parse().ok()?;
                    items.get(index)?
                }
                _ => return None,
            };
        }
        Some(node)
    }
}

/// Return a new [`RenderSpec`] with every `$data` reference in every element's
/// props replaced by its resolved value from `ctx`.
///
/// Non-reference props are copied unchanged. References to missing paths become
/// `null`. The element graph (types, children, root) is untouched — only prop
/// *values* are rewritten — so a bound spec validates and renders exactly like a
/// literal one.
#[must_use]
pub fn bind(spec: &RenderSpec, ctx: &DataContext) -> RenderSpec {
    let mut bound = spec.clone();
    for element in bound.elements.values_mut() {
        bind_element(element, ctx);
    }
    bound
}

fn bind_element(element: &mut Element, ctx: &DataContext) {
    for value in element.props.values_mut() {
        *value = resolve_value(value, ctx);
    }
}

/// Recursively resolve `$data` references within a prop value.
///
/// A `{ "$data": "path" }` object resolves to the referenced value (or `null`).
/// Other objects and arrays are walked so references nested inside structured
/// props (e.g. a table's `rows`) are resolved too; scalars pass through.
fn resolve_value(value: &Value, ctx: &DataContext) -> Value {
    match value {
        Value::Object(map) => {
            if let Some(path) = data_ref_path(map) {
                return ctx.resolve(path).cloned().unwrap_or(Value::Null);
            }
            Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), resolve_value(v, ctx)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(|v| resolve_value(v, ctx)).collect()),
        scalar => scalar.clone(),
    }
}

/// If `map` is exactly a data reference (`{ "$data": "<string>" }`), return the
/// path. A `$data` key whose value is not a string, or an object carrying other
/// keys alongside `$data`, is not treated as a reference.
fn data_ref_path(map: &Map<String, Value>) -> Option<&str> {
    if map.len() == 1 {
        map.get(DATA_REF_KEY).and_then(Value::as_str)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_render::parse;
    use serde_json::json;

    fn ctx() -> DataContext {
        DataContext::new(json!({
            "gates": { "passRate": "94%", "score": 92 },
            "checks": [{ "name": "secrets" }, { "name": "lint" }]
        }))
    }

    #[test]
    fn resolves_object_and_array_paths() {
        let c = ctx();
        assert_eq!(c.resolve("gates.passRate"), Some(&json!("94%")));
        assert_eq!(c.resolve("gates.score"), Some(&json!(92)));
        assert_eq!(c.resolve("checks.1.name"), Some(&json!("lint")));
        assert_eq!(c.resolve(""), Some(&c.root));
    }

    #[test]
    fn missing_paths_resolve_to_none() {
        let c = ctx();
        assert!(c.resolve("gates.missing").is_none());
        assert!(c.resolve("checks.9.name").is_none());
        assert!(c.resolve("gates.score.deeper").is_none()); // into a scalar
    }

    #[test]
    fn bind_replaces_data_refs_with_values() {
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "m",
                 "elements": {
                   "m": { "type": "MetricCard",
                          "props": { "label": "Pass Rate", "value": { "$data": "gates.passRate" } },
                          "children": [] }
                 } }"#,
        )
        .expect("parse");
        let bound = bind(&spec, &ctx());
        let props = &bound.element("m").expect("m").props;
        assert_eq!(props.get("value"), Some(&json!("94%")));
        // Literal props are untouched.
        assert_eq!(props.get("label"), Some(&json!("Pass Rate")));
    }

    #[test]
    fn missing_ref_binds_to_null() {
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "m",
                 "elements": {
                   "m": { "type": "MetricCard",
                          "props": { "value": { "$data": "gates.nope" } }, "children": [] }
                 } }"#,
        )
        .expect("parse");
        let bound = bind(&spec, &ctx());
        assert_eq!(
            bound.element("m").expect("m").props.get("value"),
            Some(&Value::Null),
            "a missing path binds to null (renders as an em dash)"
        );
    }

    #[test]
    fn resolves_refs_nested_in_arrays() {
        // A reference inside a structured prop (here, a table row) is resolved.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "t",
                 "elements": {
                   "t": { "type": "Table",
                          "props": { "rows": [[ { "$data": "gates.score" }, "ok" ]] },
                          "children": [] }
                 } }"#,
        )
        .expect("parse");
        let bound = bind(&spec, &ctx());
        assert_eq!(
            bound.element("t").expect("t").props.get("rows"),
            Some(&json!([[92, "ok"]]))
        );
    }

    #[test]
    fn object_with_extra_keys_is_not_a_data_ref() {
        // `$data` alongside other keys is a literal object, not a reference.
        let spec = parse(
            r#"{ "title": "x", "version": "1.0", "root": "m",
                 "elements": {
                   "m": { "type": "Text",
                          "props": { "children": { "$data": "gates.passRate", "fallback": "?" } },
                          "children": [] }
                 } }"#,
        )
        .expect("parse");
        let bound = bind(&spec, &ctx());
        // Not resolved to "94%"; the inner $data string stays as authored.
        assert_eq!(
            bound.element("m").expect("m").props.get("children"),
            Some(&json!({ "$data": "gates.passRate", "fallback": "?" }))
        );
    }
}
