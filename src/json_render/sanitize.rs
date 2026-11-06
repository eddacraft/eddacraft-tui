//! Sanitise untrusted spec/data strings before they reach the terminal.
//!
//! Specs (and bound data) may come from an untrusted source (LLM output, cloned
//! repo, etc.). Ratatui does **not** strip control bytes: a grapheme that is a
//! lone `ESC`, `BEL`, `CR`, or an OSC introducer is written to the terminal
//! verbatim on flush, so a hostile string prop could emit raw ANSI/OSC escape
//! sequences (display corruption, OSC-52 clipboard writes, title rewrites).
//! Every spec/data-derived string that becomes displayed text is passed through
//! [`sanitize`] first. [`render_spec`](crate::json_render::render_spec) applies
//! [`sanitize_props`] to each element's props before a component sees them.

use std::borrow::Cow;

use serde_json::Value;

use crate::json_render::spec::Props;

/// Ceiling for one array or object while sanitising props.
///
/// Matches the per-frame ceilings in the built-in table (`MAX_TABLE_ROWS`) and
/// charts (`MAX_CHART_POINTS`). A larger collection is truncated before it is
/// copied, so a hostile payload cannot force a full deep copy on every redraw.
const MAX_PROPS_COLLECTION: usize = 1_000;

/// Strip display-hostile characters from `s`, returning a display-safe owned
/// string.
///
/// Drops:
/// - every `char::is_control()` codepoint — the C0 range (incl. `ESC`, `BEL`,
///   `CR`, `LF`, `TAB`), `DEL`, and the C1 range — any of which could reach the
///   terminal as a raw escape; and
/// - Unicode bidi control / zero-width characters —
///   `char::is_control()` returns `false` for these, but a `RIGHT-TO-LEFT
///   OVERRIDE` (U+202E) or zero-width joiner can visually reorder/spoof a label
///   or title (e.g. make `fail` read as `pass`), so they are stripped too.
///
/// Ordinary text — including legitimate non-ASCII (accents, em dash, box-drawing,
/// emoji) — is returned unchanged.
#[must_use]
pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() && !is_bidi_or_zero_width(*c))
        .collect()
}

/// Props safe to hand to a component.
///
/// When every string is already display-safe and no collection exceeds
/// [`MAX_PROPS_COLLECTION`], this borrows `props` and allocates nothing.
/// Otherwise it returns an owned bag: strings are sanitised, arrays and
/// objects are capped, and a key that only matches another key after
/// sanitising does not overwrite the entry already stored.
///
/// [`render_spec`](crate::json_render::render_spec) passes the result to
/// [`TuiComponent::render`](crate::json_render::TuiComponent::render), so a
/// component that paints a raw string prop cannot emit terminal control bytes.
/// Numbers, booleans, and null are unchanged.
#[must_use]
pub(crate) fn sanitize_props(props: &Props) -> Cow<'_, Props> {
    if object_needs_rebuild(props) {
        Cow::Owned(rebuild_object(props))
    } else {
        Cow::Borrowed(props)
    }
}

fn string_needs_sanitize(text: &str) -> bool {
    text.chars()
        .any(|c| c.is_control() || is_bidi_or_zero_width(c))
}

fn value_needs_rebuild(value: &Value) -> bool {
    match value {
        Value::String(text) => string_needs_sanitize(text),
        // Length is enough. Do not walk past the ceiling: that walk is the
        // per-frame cost the built-in caps exist to avoid.
        Value::Array(items) => {
            items.len() > MAX_PROPS_COLLECTION
                || items
                    .iter()
                    .take(MAX_PROPS_COLLECTION)
                    .any(value_needs_rebuild)
        }
        Value::Object(map) => object_needs_rebuild(map),
        _ => false,
    }
}

fn object_needs_rebuild(map: &Props) -> bool {
    map.len() > MAX_PROPS_COLLECTION
        || map
            .iter()
            .any(|(key, value)| string_needs_sanitize(key) || value_needs_rebuild(value))
}

fn rebuild_value(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(sanitize(text)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .take(MAX_PROPS_COLLECTION)
                .map(rebuild_value)
                .collect(),
        ),
        Value::Object(map) => Value::Object(rebuild_object(map)),
        other => other.clone(),
    }
}

fn rebuild_object(map: &Props) -> Props {
    let mut out = Props::new();
    for (key, value) in map.iter().take(MAX_PROPS_COLLECTION) {
        let value = rebuild_value(value);
        if string_needs_sanitize(key) {
            let clean = sanitize(key);
            // A sanitised alias must not replace a key already stored. The
            // clean spelling, when present, is inserted on its own turn and
            // overwrites an alias that sorted earlier.
            if !out.contains_key(&clean) {
                out.insert(clean, value);
            }
        } else {
            out.insert(key.clone(), value);
        }
    }
    out
}

/// Whether `c` is a Unicode bidirectional-control or zero-width codepoint that
/// can spoof or corrupt terminal display without being a C0/C1 control.
fn is_bidi_or_zero_width(c: char) -> bool {
    // Deliberate deny-list (these are Unicode category `Cf` "format" chars that
    // `char::is_control()` does not catch). Periodically review as Unicode
    // allocates new format characters.
    matches!(c,
        // Arabic letter mark (a bidi control).
        '\u{061C}'
        // Zero-width space/joiner/non-joiner + LRM/RLM.
        | '\u{200B}'..='\u{200F}'
        // Bidi embeddings, overrides, and pop (incl. U+202E RTL OVERRIDE).
        | '\u{202A}'..='\u{202E}'
        // Word joiner + invisible math operators.
        | '\u{2060}'..='\u{2064}'
        // Bidi isolates (LRI/RLI/FSI/PDI).
        | '\u{2066}'..='\u{2069}'
        // Deprecated formatting (symmetric-swap / shaping / digit-shape).
        | '\u{206A}'..='\u{206F}'
        // Zero-width no-break space / BOM.
        | '\u{FEFF}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_bidi_override_and_zero_width() {
        // A RTL override could make "fail" render as "liaf"; it must be removed.
        let spoof = "score \u{202e}001/09\u{202c} ok\u{200b}\u{feff}";
        let clean = sanitize(spoof);
        assert!(!clean.contains('\u{202e}'), "RTL override removed");
        assert!(!clean.contains('\u{202c}'), "pop-directional removed");
        assert!(!clean.contains('\u{200b}'), "zero-width space removed");
        assert!(!clean.contains('\u{feff}'), "BOM removed");
        assert_eq!(clean, "score 001/09 ok");
        // Legitimate non-ASCII text is preserved.
        assert_eq!(sanitize("café ▲ — 92%"), "café ▲ — 92%");
    }

    #[test]
    fn strips_escape_and_osc_sequences() {
        // An OSC-52 clipboard-write attempt and a title rewrite must not survive.
        let evil = "\u{1b}]52;c;cHdwbmVk\u{07}label\u{1b}]0;pwned\u{07}";
        let clean = sanitize(evil);
        assert!(!clean.contains('\u{1b}'), "ESC removed");
        assert!(!clean.contains('\u{07}'), "BEL removed");
        assert_eq!(clean, "]52;c;cHdwbmVklabel]0;pwned");
    }

    #[test]
    fn sanitize_props_walks_nested_strings_and_keys() {
        let props = serde_json::json!({
            "label\u{1b}": ["ok\u{07}", { "text": "x\u{1b}y" }],
            "n": 1,
            "flag": true
        })
        .as_object()
        .expect("object")
        .clone();
        let clean = sanitize_props(&props);
        assert!(clean.contains_key("label"));
        assert!(!clean.keys().any(|key| key.chars().any(char::is_control)));
        let items = clean
            .get("label")
            .expect("label")
            .as_array()
            .expect("array");
        assert_eq!(items[0], serde_json::json!("ok"));
        assert_eq!(items[1].get("text").expect("text"), "xy");
        assert_eq!(clean.get("n").expect("n"), &serde_json::json!(1));
        assert_eq!(clean.get("flag").expect("flag"), &serde_json::json!(true));
    }

    #[test]
    fn drops_c0_c1_and_del_but_keeps_ordinary_text() {
        assert_eq!(sanitize("a\tb\nc\rd"), "abcd");
        assert_eq!(sanitize("plain text 92%"), "plain text 92%");
        assert_eq!(sanitize("with\u{7f}del\u{9b}c1"), "withdelc1");
        // Non-ASCII printable text is preserved.
        assert_eq!(sanitize("héllo ▲ —"), "héllo ▲ —");
    }

    #[test]
    fn sanitize_props_borrows_props_that_need_no_rebuild() {
        let props = serde_json::json!({"label": "ok", "n": [1, 2, 3]})
            .as_object()
            .expect("object")
            .clone();
        let clean = sanitize_props(&props);
        assert!(matches!(clean, Cow::Borrowed(_)));
        assert!(std::ptr::eq(clean.as_ref(), &raw const props));
    }

    #[test]
    fn sanitize_props_keeps_a_clean_key_over_its_sanitised_alias() {
        // `status` plus a trailing ESC must not replace the real `status`.
        // BTree order visits the clean key first; the alias is then dropped.
        let props = serde_json::json!({
            "status": "fail",
            "status\u{1b}": "pass"
        })
        .as_object()
        .expect("object")
        .clone();
        let clean = sanitize_props(&props);
        assert_eq!(clean.len(), 1);
        assert_eq!(clean.get("status").and_then(Value::as_str), Some("fail"));
    }

    #[test]
    fn sanitize_props_clean_key_replaces_an_alias_that_sorted_earlier() {
        // ESC sorts before 's', so the alias is inserted first and the clean
        // key must overwrite it.
        let props = serde_json::json!({
            "\u{1b}status": "pass",
            "status": "fail"
        })
        .as_object()
        .expect("object")
        .clone();
        let clean = sanitize_props(&props);
        assert_eq!(clean.len(), 1);
        assert_eq!(clean.get("status").and_then(Value::as_str), Some("fail"));
    }

    #[test]
    fn sanitize_props_caps_an_oversized_array_and_sanitises_the_prefix() {
        let mut rows = Vec::with_capacity(MAX_PROPS_COLLECTION + 1);
        rows.push(Value::String("\u{1b}kept".to_owned()));
        rows.extend(
            std::iter::repeat_with(|| Value::String("ok".to_owned()))
                .take(MAX_PROPS_COLLECTION - 1),
        );
        rows.push(Value::String("\u{1b}dropped".to_owned()));
        let mut props = Props::new();
        props.insert("rows".to_owned(), Value::Array(rows));
        let clean = sanitize_props(&props);
        let kept = clean.get("rows").and_then(Value::as_array).expect("rows");
        assert_eq!(kept.len(), MAX_PROPS_COLLECTION);
        assert_eq!(kept[0], Value::String("kept".to_owned()));
        assert!(kept.iter().all(|value| value.as_str() != Some("dropped")));
        assert!(kept.iter().all(|value| {
            value
                .as_str()
                .is_none_or(|text| !text.chars().any(char::is_control))
        }));
    }
}
