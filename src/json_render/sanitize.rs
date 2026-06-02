//! Sanitise untrusted spec/data strings before they reach the terminal.
//!
//! Dashboard specs and `.anvil/` data may come from an untrusted source (a
//! cloned repo). Ratatui does **not** strip control bytes: a grapheme that is a
//! lone `ESC`, `BEL`, `CR`, or an OSC introducer is written to the terminal
//! verbatim on flush, so a hostile string prop could emit raw ANSI/OSC escape
//! sequences to the operator's terminal (display corruption, OSC-52 clipboard
//! writes, title rewrites). Every spec/data-derived string that becomes
//! displayed text is passed through [`sanitize`] first.

/// Strip control characters from `s`, returning a display-safe owned string.
///
/// Drops every `char::is_control()` codepoint — the C0 range (incl. `ESC`,
/// `BEL`, `CR`, `LF`, `TAB`), `DEL`, and the C1 range — none of which carry
/// meaning in the terse single-/few-line widgets dashboards render, and any of
/// which could otherwise reach the terminal as a raw escape. Ordinary text is
/// returned unchanged.
#[must_use]
pub fn sanitize(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
