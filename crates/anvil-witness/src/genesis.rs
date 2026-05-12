use std::fmt;

/// The anchor a chain is rooted at. ADR-037 §D-2 defines two:
///
/// - **`GENESIS-FRESH`** — a brand-new repo adopting Anvil with no
///   prior history. The first witness line's `prev_line_hash` is
///   exactly the string `"GENESIS-FRESH"` (no quotes).
/// - **`GENESIS-BASELINED`** — an existing repo adopting Anvil via
///   `anvil baseline`. The first line's `prev_line_hash` is the
///   literal string `"GENESIS-BASELINED"`; the cutoff commit SHA is
///   recorded as a separate `cutoff_commit` field on the line body
///   rather than glued onto the anchor string. This keeps the
///   anchor namespace closed-set and matches ADR-037 §D-2.
///
/// The literal anchor string is what appears on disk in the first
/// line's `prev_line_hash` field, so a downgraded reader that doesn't
/// know about anchors can still detect "this isn't a SHA-256" and
/// treat the line as a chain root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisAnchor {
    Fresh,
    Baselined,
}

impl GenesisAnchor {
    /// The literal string that goes in a line's `prev_line_hash`
    /// field. Stable; do not change without an ADR amendment.
    pub const fn anchor_string(&self) -> &'static str {
        match self {
            Self::Fresh => "GENESIS-FRESH",
            Self::Baselined => "GENESIS-BASELINED",
        }
    }

    /// Parse an anchor string back into the enum.
    ///
    /// Returns `None` if the string does not look like an anchor
    /// (i.e. it should be interpreted as a SHA-256 hex digest
    /// instead). The verifier uses this to discriminate between a
    /// chain-root reference and an ordinary line-hash reference
    /// without a separate marker field.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "GENESIS-FRESH" => Some(Self::Fresh),
            "GENESIS-BASELINED" => Some(Self::Baselined),
            _ => None,
        }
    }
}

impl fmt::Display for GenesisAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.anchor_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_anchor_string() {
        assert_eq!(GenesisAnchor::Fresh.anchor_string(), "GENESIS-FRESH");
    }

    #[test]
    fn baselined_anchor_string() {
        assert_eq!(
            GenesisAnchor::Baselined.anchor_string(),
            "GENESIS-BASELINED"
        );
    }

    #[test]
    fn parse_fresh() {
        assert_eq!(
            GenesisAnchor::parse("GENESIS-FRESH"),
            Some(GenesisAnchor::Fresh)
        );
    }

    #[test]
    fn parse_baselined() {
        assert_eq!(
            GenesisAnchor::parse("GENESIS-BASELINED"),
            Some(GenesisAnchor::Baselined),
        );
    }

    #[test]
    fn parse_rejects_colon_suffix_form() {
        // ADR-037 §D-2 uses bare anchors. The cutoff commit lives on
        // the line body as a separate field, not glued onto the
        // anchor string.
        assert_eq!(GenesisAnchor::parse("GENESIS-BASELINED:abc"), None);
    }

    #[test]
    fn parse_non_anchor_is_none() {
        // Looks like a SHA — not an anchor.
        assert_eq!(
            GenesisAnchor::parse(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            ),
            None,
        );
    }

    #[test]
    fn round_trip() {
        for a in [GenesisAnchor::Fresh, GenesisAnchor::Baselined] {
            assert_eq!(GenesisAnchor::parse(a.anchor_string()), Some(a));
        }
    }
}
