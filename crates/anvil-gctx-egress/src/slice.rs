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
    /// The per-session per-file unique-coverage byte ceiling was exhausted (CE-6).
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

/// Per-session snippet byte ledger (CE-6).
///
/// Tracks **file-level** covered byte intervals so overlapping-span calls cannot
/// reassemble a whole file. Only *newly exposed* position bytes are charged
/// against the per-file session ceiling; fully covered spans re-admit free.
/// Keys on position identity (`file` + [`ByteRange`]), never on text content.
#[derive(Debug, Clone, Default)]
pub struct SnippetByteLedger {
    /// Merged, non-overlapping covered intervals per file, ordered by `start`.
    covered: std::collections::BTreeMap<String, Vec<ByteRange>>,
}

/// Hard per-file unique-coverage session byte ceiling (CE-6): independent of the
/// per-call token budget. Overlapping spans share this file-level allowance.
pub const MAX_SESSION_SNIPPET_BYTES_PER_FILE: u32 = 16 * 1024;

/// Historical alias of [`MAX_SESSION_SNIPPET_BYTES_PER_FILE`] (kept for call-site
/// compatibility; the cap is enforced on per-file unique coverage, not per-span).
pub const MAX_SESSION_SNIPPET_BYTES_PER_SPAN: u32 = MAX_SESSION_SNIPPET_BYTES_PER_FILE;

impl SnippetByteLedger {
    /// Total unique bytes already covered for `file`.
    fn covered_total(&self, file: &str) -> u32 {
        self.covered.get(file).map_or(0, |ivs| {
            ivs.iter()
                .map(|iv| iv.len())
                .fold(0u32, u32::saturating_add)
        })
    }

    /// Bytes of `span` not already present in this file's covered intervals.
    fn newly_exposed(&self, file: &str, span: ByteRange) -> u32 {
        if span.is_empty() {
            return 0;
        }
        let Some(intervals) = self.covered.get(file) else {
            return span.len();
        };
        let mut remaining = span.len();
        for iv in intervals {
            if iv.end <= span.start {
                continue;
            }
            if iv.start >= span.end {
                break;
            }
            let overlap_start = span.start.max(iv.start);
            let overlap_end = span.end.min(iv.end);
            if overlap_start < overlap_end {
                remaining = remaining.saturating_sub(overlap_end - overlap_start);
            }
        }
        remaining
    }

    /// Position interval actually charged for an admission of `span` with
    /// `bytes` of emitted text. Truncation keeps a leading slice of the span
    /// (`project_snippet`); never charge more position bytes than were emitted.
    fn charge_span(span: ByteRange, bytes: u32) -> ByteRange {
        if span.is_empty() || bytes == 0 {
            return ByteRange {
                start: span.start,
                end: span.start,
            };
        }
        let cover_len = span.len().min(bytes);
        ByteRange {
            start: span.start,
            end: span.start.saturating_add(cover_len),
        }
    }

    /// Whether admitting `span` in `file` would keep unique coverage under the
    /// per-file session ceiling. `bytes` is the emitted text length and caps how
    /// much of `span` is charged (so truncated large symbols are not refused
    /// solely for their full defining span).
    #[must_use]
    pub fn can_admit(&self, file: &str, span: ByteRange, bytes: u32) -> bool {
        if bytes == 0 {
            return true;
        }
        let charged = Self::charge_span(span, bytes);
        let charge = if charged.is_empty() {
            // Empty span with non-zero text: fall back to text length against
            // the file total (no position coverage to merge).
            bytes
        } else {
            self.newly_exposed(file, charged)
        };
        if charge == 0 {
            return true;
        }
        self.covered_total(file).saturating_add(charge) <= MAX_SESSION_SNIPPET_BYTES_PER_SPAN
    }

    /// Record that `span` in `file` was egressed (merge the emitted prefix into
    /// covered intervals). `bytes` caps the charged coverage to the emitted
    /// text length so truncation does not mark unexposed tail bytes as spent.
    pub fn record(&mut self, file: &str, span: ByteRange, bytes: u32) {
        let charged = Self::charge_span(span, bytes);
        if charged.is_empty() {
            return;
        }
        let intervals = self.covered.entry(file.to_string()).or_default();
        // Insert `charged` and merge overlapping / adjacent intervals.
        intervals.push(charged);
        intervals.sort_by_key(|iv| iv.start);
        let mut merged: Vec<ByteRange> = Vec::with_capacity(intervals.len());
        for iv in intervals.drain(..) {
            if let Some(last) = merged.last_mut() {
                // Adjacent (end == next.start) merges so a tiling of small
                // spans still exhausts the unique-coverage ceiling.
                if iv.start <= last.end {
                    last.end = last.end.max(iv.end);
                    continue;
                }
            }
            merged.push(iv);
        }
        *intervals = merged;
    }
}

/// Estimate tokens for a snippet's emitted text (redact-before-measure, CE-2).
fn snippet_token_cost(snippet: &SnippetResult) -> u32 {
    let text = snippet.text.as_deref().unwrap_or("");
    estimate_gctx_tokens(text, Some(snippet.language.as_str()))
        .map_or(u32::MAX, |e| u32::try_from(e.tokens).unwrap_or(u32::MAX))
}

/// Bytes that count toward the session ceiling gate (emitted text only; a
/// zero-cost snippet skips the ledger). Coverage itself is position-based.
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
            span: ByteRange {
                start: 0,
                end: u32::try_from(text.len()).unwrap_or(0),
            },
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
                    let cands = vec![cand("a", 0, Some(texts[i])), cand("b", 1, Some(texts[j]))];
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

    fn snippet_at(file: &str, start: u32, end: u32, text: &str) -> SnippetResult {
        SnippetResult {
            file: file.into(),
            span: ByteRange { start, end },
            language: "typescript".into(),
            stale: false,
            text: Some(text.to_string()),
            truncated: false,
            omitted_bytes: 0,
            redacted_secrets: 0,
        }
    }

    #[test]
    fn byte_ledger_blocks_new_span_after_file_ceiling() {
        // Exhaust the per-file unique-coverage ceiling with a full 16 KiB span.
        let mut ledger = SnippetByteLedger::default();
        let full = MAX_SESSION_SNIPPET_BYTES_PER_SPAN;
        ledger.record(
            "f.ts",
            ByteRange {
                start: 0,
                end: full,
            },
            full,
        );

        // A disjoint span past the ceiling must be refused (new exposure).
        let text = "x".repeat(100);
        let snippet = snippet_at("f.ts", full, full + 100, &text);
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
    fn byte_ledger_blocks_overlapping_span_reassembly() {
        // Distinct but overlapping ranges must share the per-file coverage budget
        // so sliding-window requests cannot reassemble a whole file (CE-6).
        let mut ledger = SnippetByteLedger::default();
        let half = MAX_SESSION_SNIPPET_BYTES_PER_SPAN / 2;
        let full = MAX_SESSION_SNIPPET_BYTES_PER_SPAN;

        // First call: admit [0, full).
        let text_a = "a".repeat(full as usize);
        let snip_a = snippet_at("big.ts", 0, full, &text_a);
        let cands_a = vec![SliceCandidate {
            identity: id("a", 0),
            distance: 0,
            snippet: Some(snip_a),
        }];
        let s_a = slice_under_budget(cands_a, 100_000, Some(&mut ledger));
        assert_eq!(s_a.snippets.len(), 1, "first full span must admit");
        assert!(!s_a.byte_ceiling_hit);

        // Second call: overlapping [half, half+full) exposes half new bytes
        // which would push unique coverage past the per-file ceiling.
        let text_b = "b".repeat(full as usize);
        let snip_b = snippet_at("big.ts", half, half + full, &text_b);
        let cands_b = vec![SliceCandidate {
            identity: id("b", 0),
            distance: 0,
            snippet: Some(snip_b),
        }];
        let s_b = slice_under_budget(cands_b, 100_000, Some(&mut ledger));
        assert!(
            s_b.snippets.is_empty(),
            "overlapping span must not admit after file ceiling"
        );
        assert_eq!(s_b.omitted[0].reason, SliceOmitReason::ByteCeiling);
        assert!(s_b.byte_ceiling_hit);
    }

    #[test]
    fn byte_ledger_allows_non_overlapping_spans_under_ceiling() {
        let mut ledger = SnippetByteLedger::default();
        let half = MAX_SESSION_SNIPPET_BYTES_PER_SPAN / 2;
        let text = "x".repeat(half as usize);

        let snip_a = snippet_at("f.ts", 0, half, &text);
        let snip_b = snippet_at("f.ts", half, half * 2, &text);
        let cands = vec![
            SliceCandidate {
                identity: id("a", 0),
                distance: 0,
                snippet: Some(snip_a),
            },
            SliceCandidate {
                identity: id("b", 0),
                distance: 1,
                snippet: Some(snip_b),
            },
        ];
        let s = slice_under_budget(cands, 100_000, Some(&mut ledger));
        assert_eq!(s.snippets.len(), 2);
        assert!(!s.byte_ceiling_hit);

        // Third non-overlapping span exceeds the per-file ceiling.
        let snip_c = snippet_at("f.ts", half * 2, half * 3, &text);
        let cands_c = vec![SliceCandidate {
            identity: id("c", 0),
            distance: 0,
            snippet: Some(snip_c),
        }];
        let s_c = slice_under_budget(cands_c, 100_000, Some(&mut ledger));
        assert!(s_c.snippets.is_empty());
        assert_eq!(s_c.omitted[0].reason, SliceOmitReason::ByteCeiling);
    }

    #[test]
    fn byte_ledger_admits_truncated_prefix_of_oversized_span() {
        // A symbol span larger than the ceiling must still admit when only a
        // truncated prefix is emitted (matches project_snippet truncation).
        let mut ledger = SnippetByteLedger::default();
        let oversized_end = MAX_SESSION_SNIPPET_BYTES_PER_FILE * 2;
        let emitted = MAX_SESSION_SNIPPET_BYTES_PER_FILE;
        let text = "x".repeat(emitted as usize);
        let snip = snippet_at("big.ts", 0, oversized_end, &text);
        // snippet_at sets text length = emitted, span end = oversized.
        assert_eq!(snip.span.end, oversized_end);
        assert_eq!(snip.text.as_ref().map(String::len), Some(emitted as usize));

        let cands = vec![SliceCandidate {
            identity: id("big", 0),
            distance: 0,
            snippet: Some(snip),
        }];
        let s = slice_under_budget(cands, 200_000, Some(&mut ledger));
        assert_eq!(s.snippets.len(), 1, "truncated oversized span must admit");
        assert!(!s.byte_ceiling_hit);

        // A further non-overlapping prefix past the emitted region is refused.
        let more = "y".repeat(100);
        let snip2 = snippet_at("big.ts", emitted, emitted + 100, &more);
        let cands2 = vec![SliceCandidate {
            identity: id("more", 0),
            distance: 0,
            snippet: Some(snip2),
        }];
        let s2 = slice_under_budget(cands2, 200_000, Some(&mut ledger));
        assert!(s2.snippets.is_empty());
        assert_eq!(s2.omitted[0].reason, SliceOmitReason::ByteCeiling);
    }

    #[test]
    fn byte_ledger_readmits_fully_covered_span() {
        // Re-requesting an already-covered span exposes no new file bytes.
        let mut ledger = SnippetByteLedger::default();
        let text = "hello";
        let snip = snippet_at("f.ts", 0, 5, text);
        ledger.record("f.ts", snip.span, 5);

        let cands = vec![SliceCandidate {
            identity: id("again", 0),
            distance: 0,
            snippet: Some(snip),
        }];
        let s = slice_under_budget(cands, 10_000, Some(&mut ledger));
        assert_eq!(s.snippets.len(), 1);
        assert!(!s.byte_ceiling_hit);
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
