//! Budget-bounded context slicing (GCTX-022).
//!
//! Turns graph-selected symbols (each with an optional extracted snippet) into
//! the smallest useful, deterministic context under a token budget. Pure and
//! IO-free: snippet extraction (GCTX-021) and token estimation (GCTX-020) happen
//! upstream; this module only orders and packs.
//!
//! Candidates are ordered by ascending graph `distance` (the seed is `distance ==
//! 0`), with [`SymbolIdentity`] as the deterministic tiebreak. The slicer
//! greedily includes every snippet that still fits the remaining budget, skipping
//! (omitting with [`SliceOmitReason::Budget`]) overflow candidates and
//! continuing to later, smaller ones.

use anvil_gctx_types::{OmitReason, SnippetResult};
use anvil_graph_cache::estimate_gctx_tokens;
use anvil_kernel_types::{ByteRange, SymbolIdentity};

/// Why a candidate did not make the slice (internal — mapped to [`OmitReason`] on
/// seal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceOmitReason {
    /// Including it would have exceeded the token budget.
    Budget,
    /// No snippet was available upstream.
    Unlocatable,
    /// The per-session `(file, ByteRange)` byte ceiling was exhausted (CE-6).
    ByteCeiling,
}

/// One graph-selected symbol offered to the slicer.
#[derive(Debug, Clone)]
pub struct SliceCandidate {
    /// The symbol's stable identity.
    pub identity: SymbolIdentity,
    /// Graph distance from the query seed (`0` for the seed itself).
    pub distance: u32,
    /// The extracted snippet, or `None` when unavailable.
    pub snippet: Option<SnippetResult>,
}

/// A snippet selected into the slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedSnippet {
    pub identity: SymbolIdentity,
    pub distance: u32,
    pub snippet: SnippetResult,
}

/// A candidate that did not make the slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmittedSnippet {
    pub identity: SymbolIdentity,
    pub reason: SliceOmitReason,
}

/// The budget-bounded slice.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextSlice {
    pub snippets: Vec<SelectedSnippet>,
    pub omitted: Vec<OmittedSnippet>,
    pub estimated_tokens: u32,
    pub byte_ceiling_hit: bool,
}

impl SliceOmitReason {
    #[must_use]
    pub const fn to_egress_reason(self) -> OmitReason {
        match self {
            Self::Budget => OmitReason::Budget,
            Self::Unlocatable => OmitReason::Unlocatable,
            Self::ByteCeiling => OmitReason::ByteCeiling,
        }
    }
}

/// Per-session snippet byte ledger keyed on `(file, ByteRange)` identity (CE-6).
///
/// Tracks cumulative bytes egressed per span position so overlapping-span calls
/// cannot reassemble a whole file.
#[derive(Debug, Clone, Default)]
pub struct SnippetByteLedger {
    spent: std::collections::BTreeMap<(String, ByteRange), u32>,
}

/// Hard per-span session byte ceiling (CE-6): independent of the per-call token
/// budget.
pub const MAX_SESSION_SNIPPET_BYTES_PER_SPAN: u32 = 16 * 1024;

impl SnippetByteLedger {
    /// Whether `span` in `file` can admit `bytes` more under the session ceiling.
    #[must_use]
    pub fn can_admit(&self, file: &str, span: ByteRange, bytes: u32) -> bool {
        let key = (file.to_string(), span);
        let used = self.spent.get(&key).copied().unwrap_or(0);
        used.saturating_add(bytes) <= MAX_SESSION_SNIPPET_BYTES_PER_SPAN
    }

    /// Record `bytes` egressed for `span` in `file`.
    pub fn record(&mut self, file: &str, span: ByteRange, bytes: u32) {
        let key = (file.to_string(), span);
        let entry = self.spent.entry(key).or_insert(0);
        *entry = entry.saturating_add(bytes);
    }
}

/// Estimate tokens for a snippet's emitted text (redact-before-measure, CE-2).
fn snippet_token_cost(snippet: &SnippetResult) -> u32 {
    let text = snippet.text.as_deref().unwrap_or("");
    estimate_gctx_tokens(text, Some(snippet.language.as_str()))
        .map_or(u32::MAX, |e| u32::try_from(e.tokens).unwrap_or(u32::MAX))
}

/// Bytes that count toward the per-span session ceiling (emitted text only).
fn snippet_byte_cost(snippet: &SnippetResult) -> u32 {
    u32::try_from(snippet.text.as_ref().map_or(0, String::len)).unwrap_or(u32::MAX)
}

/// Pack `candidates` into a [`ContextSlice`] under `token_budget`, honouring the
/// optional per-session byte ledger. The returned `estimated_tokens` never exceeds
/// `token_budget`.
#[must_use]
pub fn slice_under_budget(
    mut candidates: Vec<SliceCandidate>,
    token_budget: u32,
    mut byte_ledger: Option<&mut SnippetByteLedger>,
) -> ContextSlice {
    candidates.sort_by(|a, b| {
        a.distance
            .cmp(&b.distance)
            .then_with(|| a.identity.cmp(&b.identity))
    });

    let mut out = ContextSlice::default();
    for c in candidates {
        let Some(snippet) = c.snippet else {
            out.omitted.push(OmittedSnippet {
                identity: c.identity,
                reason: SliceOmitReason::Unlocatable,
            });
            continue;
        };

        let byte_cost = snippet_byte_cost(&snippet);
        if let Some(ledger) = byte_ledger.as_deref_mut()
            && byte_cost > 0
            && !ledger.can_admit(&snippet.file, snippet.span, byte_cost)
        {
            out.omitted.push(OmittedSnippet {
                identity: c.identity,
                reason: SliceOmitReason::ByteCeiling,
            });
            out.byte_ceiling_hit = true;
            continue;
        }

        let cost = snippet_token_cost(&snippet);
        let fits = out
            .estimated_tokens
            .checked_add(cost)
            .is_some_and(|total| total <= token_budget);
        if fits {
            if let Some(ledger) = byte_ledger.as_deref_mut()
                && byte_cost > 0
            {
                ledger.record(&snippet.file, snippet.span, byte_cost);
            }
            out.estimated_tokens += cost;
            out.snippets.push(SelectedSnippet {
                identity: c.identity,
                distance: c.distance,
                snippet,
            });
        } else {
            out.omitted.push(OmittedSnippet {
                identity: c.identity,
                reason: SliceOmitReason::Budget,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::SymbolKind;

    fn id(name: &str, ordinal: u32) -> SymbolIdentity {
        SymbolIdentity {
            file: "f.ts".to_string(),
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal,
        }
    }

    /// Build a snippet whose token cost is driven by `text` length.
    fn snippet_with_text(text: &str) -> SnippetResult {
        SnippetResult {
            file: "f.ts".into(),
            span: ByteRange { start: 0, end: u32::try_from(text.len()).unwrap_or(0) },
            language: "typescript".into(),
            stale: false,
            text: Some(text.to_string()),
            truncated: false,
            omitted_bytes: 0,
            redacted_secrets: 0,
        }
    }

    fn cand(name: &str, distance: u32, text: Option<&str>) -> SliceCandidate {
        SliceCandidate {
            identity: id(name, 0),
            distance,
            snippet: text.map(snippet_with_text),
        }
    }

    #[test]
    fn property_total_never_exceeds_budget() {
        let texts = ["", "ab", "abcdef", "++++++++++"];
        for budget in [0u32, 1, 5, 20, 100] {
            for i in 0..texts.len() {
                for j in 0..texts.len() {
                    let cands = vec![
                        cand("a", 0, Some(texts[i])),
                        cand("b", 1, Some(texts[j])),
                    ];
                    let s = slice_under_budget(cands, budget, None);
                    assert!(
                        s.estimated_tokens <= budget,
                        "budget {budget} exceeded: {}",
                        s.estimated_tokens
                    );
                }
            }
        }
    }

    #[test]
    fn deterministic_for_same_input() {
        let build = || {
            vec![
                cand("b", 1, Some("beta")),
                cand("a", 0, Some("alpha")),
                cand("c", 1, None),
            ]
        };
        assert_eq!(
            slice_under_budget(build(), 10_000, None),
            slice_under_budget(build(), 10_000, None)
        );
    }

    #[test]
    fn byte_ledger_blocks_overlapping_span_reassembly() {
        let text = "x".repeat(100);
        let snippet = snippet_with_text(&text);
        let mut ledger = SnippetByteLedger::default();
        ledger.record(&snippet.file, snippet.span, MAX_SESSION_SNIPPET_BYTES_PER_SPAN);

        let cands = vec![SliceCandidate {
            identity: id("blocked", 0),
            distance: 0,
            snippet: Some(snippet),
        }];
        let s = slice_under_budget(cands, 10_000, Some(&mut ledger));
        assert!(s.snippets.is_empty());
        assert_eq!(s.omitted[0].reason, SliceOmitReason::ByteCeiling);
        assert!(s.byte_ceiling_hit);
    }

    #[test]
    fn redacted_text_counts_toward_budget_not_raw_secret() {
        let secret = "sk-live-0123456789abcdef0123456789abcdef";
        let redacted = "const k = \"«redacted»\";";
        let raw = format!("const k = \"{secret}\";");
        let redacted_cost = snippet_token_cost(&snippet_with_text(redacted));
        let raw_cost = snippet_token_cost(&snippet_with_text(&raw));
        assert!(
            redacted_cost < raw_cost,
            "budget must use emitted redacted text, not the raw secret: {redacted_cost} vs {raw_cost}"
        );
    }
}