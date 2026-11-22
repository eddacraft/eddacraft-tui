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

use ratatui::layout::Rect;
use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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
    s.chars().filter(|c| is_display_safe(*c)).collect()
}

/// [`sanitize`] `s`, keeping only the leading text that fits in `max_width`
/// terminal cells.
///
/// Width is display width, measured per grapheme cluster: a wide character
/// takes two cells, a combining mark or emoji modifier stays with its base,
/// and a cluster that does not fit whole is dropped. At most
/// `ZERO_WIDTH_ALLOWANCE` characters per cell of `s` are read, stripped
/// characters included, so a huge string — or a long run of stripped or
/// zero-width characters — costs no more than the text that can be shown.
/// Stripped characters use up no width.
#[must_use]
pub(crate) fn sanitize_to_width(s: &str, max_width: usize) -> String {
    let max_chars = max_width.saturating_mul(ZERO_WIDTH_ALLOWANCE);
    let mut out = sanitize_prefix(s, max_chars);
    let mut used = 0usize;
    let mut end = 0usize;
    for cluster in out.graphemes(true) {
        let width = UnicodeWidthStr::width(cluster);
        if used + width > max_width {
            break;
        }
        used += width;
        end += cluster.len();
    }
    out.truncate(end);
    out
}

/// Characters read per cell of width, at most. Room for a base character and
/// several combining marks.
const ZERO_WIDTH_ALLOWANCE: usize = 4;

/// Floor for [`prop_read_budget`], so short identifier-like props (a variant
/// or status name) survive whole in a tiny area.
const MIN_PROP_READ_BUDGET: usize = 1_024;

/// Characters of each string prop that [`sanitize_props`] reads when the
/// element is drawn into `area`.
///
/// A component cannot show more than one character per cell plus combining
/// marks, so longer text is never visible; reading it would only add work.
#[must_use]
pub(crate) fn prop_read_budget(area: Rect) -> usize {
    usize::from(area.width)
        .saturating_mul(usize::from(area.height))
        .saturating_mul(ZERO_WIDTH_ALLOWANCE)
        .max(MIN_PROP_READ_BUDGET)
}

/// [`sanitize`] at most the first `max_chars` characters of `s`. Stripped
/// characters count towards `max_chars`.
fn sanitize_prefix(s: &str, max_chars: usize) -> String {
    s.chars()
        .take(max_chars)
        .filter(|c| is_display_safe(*c))
        .collect()
}

/// Whether `c` survives [`sanitize`].
fn is_display_safe(c: char) -> bool {
    !c.is_control() && !is_bidi_or_zero_width(c)
}

/// Props safe to hand to a component.
///
/// Reads at most `max_chars` characters of each string and key — see
/// [`prop_read_budget`] — so a huge string costs no more than what the area
/// could show. When every string is already display-safe and within
/// `max_chars`, and no collection exceeds [`MAX_PROPS_COLLECTION`], this
/// borrows `props` and allocates nothing. Otherwise it returns an owned bag:
/// strings are cut to `max_chars` and sanitised, arrays and objects are
/// capped, and a key that only matches another key after sanitising does not
/// overwrite the entry already stored.
///
/// [`render_spec`](crate::json_render::render_spec) passes the result to
/// [`TuiComponent::render`](crate::json_render::TuiComponent::render), so a
/// component that paints a raw string prop cannot emit terminal control bytes.
/// Numbers, booleans, and null are unchanged.
#[must_use]
pub(crate) fn sanitize_props(props: &Props, max_chars: usize) -> Cow<'_, Props> {
    if object_needs_rebuild(props, max_chars) {
        Cow::Owned(rebuild_object(props, max_chars))
    } else {
        Cow::Borrowed(props)
    }
}

/// Whether `text` holds a stripped character or runs past `max_chars`. Reads
/// at most `max_chars + 1` characters.
fn string_needs_rebuild(text: &str, max_chars: usize) -> bool {
    text.chars()
        .enumerate()
        .any(|(index, c)| index >= max_chars || !is_display_safe(c))
}

fn value_needs_rebuild(value: &Value, max_chars: usize) -> bool {
    match value {
        Value::String(text) => string_needs_rebuild(text, max_chars),
        // Length is enough. Do not walk past the ceiling: that walk is the
        // per-frame cost the built-in caps exist to avoid.
        Value::Array(items) => {
            items.len() > MAX_PROPS_COLLECTION
                || items
                    .iter()
                    .take(MAX_PROPS_COLLECTION)
                    .any(|item| value_needs_rebuild(item, max_chars))
        }
        Value::Object(map) => object_needs_rebuild(map, max_chars),
        _ => false,
    }
}

fn object_needs_rebuild(map: &Props, max_chars: usize) -> bool {
    map.len() > MAX_PROPS_COLLECTION
        || map.iter().any(|(key, value)| {
            string_needs_rebuild(key, max_chars) || value_needs_rebuild(value, max_chars)
        })
}

fn rebuild_value(value: &Value, max_chars: usize) -> Value {
    match value {
        Value::String(text) => Value::String(sanitize_prefix(text, max_chars)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .take(MAX_PROPS_COLLECTION)
                .map(|item| rebuild_value(item, max_chars))
                .collect(),
        ),
        Value::Object(map) => Value::Object(rebuild_object(map, max_chars)),
        other => other.clone(),
    }
}

fn rebuild_object(map: &Props, max_chars: usize) -> Props {
    let mut out = Props::new();
    for (key, value) in map.iter().take(MAX_PROPS_COLLECTION) {
        let value = rebuild_value(value, max_chars);
        if string_needs_rebuild(key, max_chars) {
            let clean = sanitize_prefix(key, max_chars);
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
        let clean = sanitize_props(&props, usize::MAX);
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
        let clean = sanitize_props(&props, usize::MAX);
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
        let clean = sanitize_props(&props, usize::MAX);
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
        let clean = sanitize_props(&props, usize::MAX);
        assert_eq!(clean.len(), 1);
        assert_eq!(clean.get("status").and_then(Value::as_str), Some("fail"));
    }

    #[test]
    fn width_cut_counts_stripped_characters_towards_the_read_budget() {
        // Review regression (#7763): the read cap applies before filtering, so
        // a long stripped prefix cannot force a scan past the limit.
        let hostile = format!("{}ok", "\u{1b}".repeat(100_000));
        assert_eq!(sanitize_to_width(&hostile, 40), "");
        // Within the budget, stripped characters still do not use up width.
        let short = format!("{}ok", "\u{1b}".repeat(100));
        assert_eq!(sanitize_to_width(&short, 40), "ok");
    }

    #[test]
    fn width_cut_keeps_a_whole_grapheme_that_fits() {
        // Review regression (#7763): the skin-tone modifier belongs to its
        // base, and the cluster is two cells wide, so `👍🏽a` fits three cells.
        let thumbs = "\u{1F44D}\u{1F3FD}";
        let text = format!("{thumbs}a");
        assert_eq!(sanitize_to_width(&text, 3), text);
        assert_eq!(sanitize_to_width(&text, 2), thumbs);
        assert_eq!(sanitize_to_width(&text, 1), "");
    }

    #[test]
    fn sanitize_props_cuts_a_clean_string_to_the_read_budget() {
        // Review regression (#7763): a long clean string is cut to the read
        // budget rather than borrowed and scanned in full every frame.
        let mut props = Props::new();
        props.insert("label".to_owned(), Value::String("x".repeat(10_000)));
        props.insert("short".to_owned(), Value::String("ok".to_owned()));
        let clean = sanitize_props(&props, 100);
        assert!(matches!(clean, Cow::Owned(_)));
        let label = clean.get("label").and_then(Value::as_str).expect("label");
        assert_eq!(label.len(), 100);
        assert_eq!(clean.get("short").and_then(Value::as_str), Some("ok"));
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
        let clean = sanitize_props(&props, usize::MAX);
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
