//! Sanitise untrusted spec/data strings before they reach the terminal.
//!
//! Specs (and bound data) may come from an untrusted source (LLM output, cloned
//! repo, etc.). Ratatui does **not** strip control bytes: a grapheme that is a
//! lone `ESC`, `BEL`, `CR`, or an OSC introducer is written to the terminal
//! verbatim on flush, so a hostile string prop could emit raw ANSI/OSC escape
//! sequences (display corruption, OSC-52 clipboard writes, title rewrites).
//! Every spec/data-derived string that becomes displayed text is passed through
//! [`sanitize`] first.

/// Strip display-hostile characters from `s`, returning a display-safe owned
/// string.
///
/// Drops:
/// - every `char::is_control()` codepoint — the C0 range (incl. `ESC`, `BEL`,
///   `CR`, `LF`, `TAB`), `DEL`, and the C1 range — any of which could reach the
///   terminal as a raw escape; and
/// - Unicode [bidi control / zero-width characters](is_bidi_or_zero_width) —
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
    fn drops_c0_c1_and_del_but_keeps_ordinary_text() {
        assert_eq!(sanitize("a\tb\nc\rd"), "abcd");
        assert_eq!(sanitize("plain text 92%"), "plain text 92%");
        assert_eq!(sanitize("with\u{7f}del\u{9b}c1"), "withdelc1");
        // Non-ASCII printable text is preserved.
        assert_eq!(sanitize("héllo ▲ —"), "héllo ▲ —");
    }
}
