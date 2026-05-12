use std::fmt;

/// The chain anchor a chain rooted at. ADR-037 §D-2 defines two:
///
/// - **`GENESIS-FRESH`** — a brand-new repo adopting Anvil with no
///   prior history. The first witness line's `prev_line_hash` is
///   exactly the string `"GENESIS-FRESH"` (ASCII bytes, no quotes).
/// - **`GENESIS-BASELINED:<commit_sha>`** — an existing repo adopting
///   Anvil via `anvil baseline`. The cutoff commit is recorded so a
///   verifier can prove the chain begins at the documented baseline,
///   not at the start of repo history.
///
/// The literal anchor string is what appears on disk in the first
/// line's `prev_line_hash` field, so a downgraded reader that doesn't
/// know about anchors can still detect "this isn't a SHA-256" and
/// treat the line as a chain root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisAnchor {
    Fresh,
    Baselined { cutoff_commit_sha: String },
}

impl GenesisAnchor {
    /// The literal string that goes in a line's `prev_line_hash`
    /// field. Stable; do not change without an ADR amendment.
    pub fn anchor_string(&self) -> String {
        match self {
            Self::Fresh => "GENESIS-FRESH".to_string(),
            Self::Baselined { cutoff_commit_sha } => {
                format!("GENESIS-BASELINED:{cutoff_commit_sha}")
            }
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
        if s == "GENESIS-FRESH" {
            return Some(Self::Fresh);
        }
        if let Some(rest) = s.strip_prefix("GENESIS-BASELINED:") {
            // We accept any non-empty suffix — the verifier can decide
            // whether it's a valid commit SHA. Storing the raw string
            // means an unusual baseline (e.g. a tag) still survives
            // round-tripping.
            if !rest.is_empty() {
                return Some(Self::Baselined {
                    cutoff_commit_sha: rest.to_string(),
                });
            }
        }
        None
    }
}

impl fmt::Display for GenesisAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.anchor_string())
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
        let a = GenesisAnchor::Baselined {
            cutoff_commit_sha: "abc123".to_string(),
        };
        assert_eq!(a.anchor_string(), "GENESIS-BASELINED:abc123");
    }

    #[test]
    fn parse_fresh() {
        assert_eq!(GenesisAnchor::parse("GENESIS-FRESH"), Some(GenesisAnchor::Fresh));
    }

    #[test]
    fn parse_baselined() {
        assert_eq!(
            GenesisAnchor::parse("GENESIS-BASELINED:deadbeef"),
            Some(GenesisAnchor::Baselined {
                cutoff_commit_sha: "deadbeef".to_string(),
            }),
        );
    }

    #[test]
    fn parse_baselined_empty_suffix_is_none() {
        assert_eq!(GenesisAnchor::parse("GENESIS-BASELINED:"), None);
    }

    #[test]
    fn parse_non_anchor_is_none() {
        // Looks like a SHA — not an anchor.
        assert_eq!(
            GenesisAnchor::parse("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            None,
        );
    }

    #[test]
    fn round_trip() {
        for a in [
            GenesisAnchor::Fresh,
            GenesisAnchor::Baselined {
                cutoff_commit_sha: "abc".to_string(),
            },
        ] {
            assert_eq!(GenesisAnchor::parse(&a.anchor_string()), Some(a));
        }
    }
}
