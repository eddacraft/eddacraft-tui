//! Deterministic Tier-0 extraction from a pull-request body.

use super::{digest_bytes, valid_path_prefix};
use anvil_kernel_types::{
    ClaimKind, ConformanceClaim, DeclaredScope, EvidenceGrade, IntentSource, IntentSourceKind,
    IntentTier, ScopeAuthority,
};
use std::collections::{BTreeMap, BTreeSet};

const BLOCK_OPEN: &str = "```anvil-claims";
const BLOCK_CLOSE: &str = "```";
const PRODUCER_SCHEMA: &str = "github.pull-request-body.anvil-claims.v1";
const BODY_OVER_LIMIT_DIGEST: &str = "sha256:unavailable-body-over-limit";
const REFERENCE_OVER_LIMIT: &str = "unavailable:reference-over-limit";

/// Version of the resource limits applied by this extractor.
pub const PR_BODY_EXTRACTION_LIMITS_VERSION: u32 = 1;
/// Maximum raw PR-body size admitted for hashing and parsing.
pub const PR_BODY_MAX_BYTES: usize = 256 * 1024;
/// Maximum immutable source-reference size retained in an extraction.
pub const PR_BODY_SOURCE_REFERENCE_MAX_BYTES: usize = 4 * 1024;
/// Maximum non-empty members parsed across all declaration blocks.
pub const PR_BODY_MAX_MEMBERS: usize = 256;
/// Maximum bytes in one declaration member.
pub const PR_BODY_MEMBER_MAX_BYTES: usize = 1024;
/// Maximum distinct explicit path scopes retained.
pub const PR_BODY_MAX_SCOPES: usize = 128;
/// Maximum bytes in one explicit path prefix.
pub const PR_BODY_SCOPE_MAX_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarkdownFence {
    marker: u8,
    length: usize,
}

/// Whether the PR-body declaration was extracted without ambiguity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrBodyExtractionOutcome {
    /// Exactly one valid declaration block was extracted.
    Extracted,
    /// The source was absent, ambiguous, malformed, or contained unknown members.
    NotEvaluated,
}

/// Canonical weak-grade evidence extracted from one pull-request body.
///
/// The raw body is intentionally absent. Only its caller-supplied immutable
/// reference, SHA-256 digest, normalised members, and stable reason codes remain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrBodyClaimExtraction {
    source: IntentSource,
    claims: Vec<ConformanceClaim>,
    declared_scopes: Vec<DeclaredScope>,
    outcome: PrBodyExtractionOutcome,
    reasons: Vec<&'static str>,
}

/// Contract parts released only from an unambiguous, fully valid declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrBodyContractParts {
    pub source: IntentSource,
    pub claims: Vec<ConformanceClaim>,
    pub declared_scopes: Vec<DeclaredScope>,
}

/// Fail-honest extraction evidence that cannot be admitted as contract input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrBodyNonEvaluation {
    source: IntentSource,
    understood_claims: Vec<ConformanceClaim>,
    understood_scopes: Vec<DeclaredScope>,
    reasons: Vec<&'static str>,
}

impl PrBodyClaimExtraction {
    /// Source identity and exact-body digest, or an explicit over-budget sentinel.
    #[must_use]
    pub const fn source(&self) -> &IntentSource {
        &self.source
    }

    /// Canonically ordered members understood for diagnostics.
    #[must_use]
    pub fn claims(&self) -> &[ConformanceClaim] {
        &self.claims
    }

    /// Canonically ordered explicit scopes understood for diagnostics.
    #[must_use]
    pub fn declared_scopes(&self) -> &[DeclaredScope] {
        &self.declared_scopes
    }

    /// Extraction outcome that must be checked before contract admission.
    #[must_use]
    pub const fn outcome(&self) -> PrBodyExtractionOutcome {
        self.outcome
    }

    /// Stable reason codes for a fail-honest extraction.
    #[must_use]
    pub fn reasons(&self) -> &[&'static str] {
        &self.reasons
    }

    /// Release contract parts only when the complete source was extracted.
    pub fn into_contract_parts(self) -> Result<PrBodyContractParts, Box<PrBodyNonEvaluation>> {
        if self.outcome == PrBodyExtractionOutcome::Extracted {
            Ok(PrBodyContractParts {
                source: self.source,
                claims: self.claims,
                declared_scopes: self.declared_scopes,
            })
        } else {
            Err(Box::new(PrBodyNonEvaluation {
                source: self.source,
                understood_claims: self.claims,
                understood_scopes: self.declared_scopes,
                reasons: self.reasons,
            }))
        }
    }
}

impl PrBodyNonEvaluation {
    /// Source identity retained for the not-evaluated result.
    #[must_use]
    pub const fn source(&self) -> &IntentSource {
        &self.source
    }

    /// Bounded known members retained only for diagnostics.
    #[must_use]
    pub fn understood_claims(&self) -> &[ConformanceClaim] {
        &self.understood_claims
    }

    /// Bounded known scopes retained only for diagnostics.
    #[must_use]
    pub fn understood_scopes(&self) -> &[DeclaredScope] {
        &self.understood_scopes
    }

    /// Stable reasons that prevented admission.
    #[must_use]
    pub fn reasons(&self) -> &[&'static str] {
        &self.reasons
    }
}

/// Extract one explicit `anvil-claims` block from a pull-request body.
///
/// Known members survive malformed or additional members so diagnostics can
/// explain what was understood, but any reason makes the source not evaluated.
#[must_use]
pub fn extract_pr_body_claims(
    immutable_source_reference: &str,
    raw_body: &str,
) -> PrBodyClaimExtraction {
    let mut claims = BTreeMap::<(u8, String), ConformanceClaim>::new();
    let mut scopes = BTreeMap::<String, DeclaredScope>::new();
    let mut reasons = BTreeSet::<&'static str>::new();
    let mut block_count = 0_usize;
    let mut in_block = false;
    let mut saw_member = false;
    let mut member_count = 0_usize;
    let mut outer_fence = None;
    let mut in_html_comment = false;

    let source = extraction_source(immutable_source_reference, raw_body, &mut reasons);

    if raw_body.len() > PR_BODY_MAX_BYTES {
        return finish_extraction(source, claims, scopes, reasons);
    }

    for line in raw_body.lines() {
        if let Some(fence) = outer_fence {
            if closes_markdown_fence(line, fence) {
                outer_fence = None;
            }
            continue;
        }

        if in_html_comment {
            if line.contains("-->") {
                in_html_comment = false;
            }
            continue;
        }

        if !in_block {
            if let Some(comment_start) = line.find("<!--") {
                in_html_comment = !line[comment_start + 4..].contains("-->");
                continue;
            }
            if line == BLOCK_OPEN {
                block_count += 1;
                in_block = true;
                continue;
            }
            if let Some(fence) = parse_markdown_fence(line) {
                outer_fence = Some(fence);
            }
            continue;
        }

        if line == BLOCK_OPEN {
            block_count += 1;
            continue;
        }
        if line == BLOCK_CLOSE {
            in_block = false;
            continue;
        }
        if line.is_empty() {
            continue;
        }

        saw_member = true;
        member_count += 1;
        if member_count > PR_BODY_MAX_MEMBERS {
            reasons.insert("claim.pr-body.budget.members");
            continue;
        }
        if line.len() > PR_BODY_MEMBER_MAX_BYTES {
            reasons.insert("claim.pr-body.budget.member-bytes");
            continue;
        }
        parse_member(line, &mut claims, &mut scopes, &mut reasons);
    }

    match block_count {
        0 => {
            reasons.insert("claim.pr-body.block-missing");
        }
        1 => {}
        _ => {
            reasons.insert("claim.pr-body.multiple-blocks");
        }
    }
    if in_block {
        reasons.insert("claim.pr-body.block-unclosed");
    }
    if block_count > 0 && !saw_member {
        reasons.insert("claim.pr-body.block-empty");
    }

    finish_extraction(source, claims, scopes, reasons)
}

fn extraction_source(
    immutable_source_reference: &str,
    raw_body: &str,
    reasons: &mut BTreeSet<&'static str>,
) -> IntentSource {
    let reference = if immutable_source_reference.is_empty() {
        reasons.insert("claim.pr-body.reference-missing");
        immutable_source_reference.to_owned()
    } else if immutable_source_reference.len() > PR_BODY_SOURCE_REFERENCE_MAX_BYTES {
        reasons.insert("claim.pr-body.budget.reference-bytes");
        REFERENCE_OVER_LIMIT.to_owned()
    } else {
        immutable_source_reference.to_owned()
    };
    let digest = if raw_body.len() > PR_BODY_MAX_BYTES {
        reasons.insert("claim.pr-body.budget.body-bytes");
        BODY_OVER_LIMIT_DIGEST.to_owned()
    } else {
        digest_bytes(raw_body.as_bytes())
    };
    IntentSource {
        tier: IntentTier::Tier0,
        kind: IntentSourceKind::PullRequest,
        reference,
        digest,
        evidence_grade: EvidenceGrade::Weak,
        producer_schema: PRODUCER_SCHEMA.to_owned(),
        producer_record_id: None,
    }
}

fn finish_extraction(
    source: IntentSource,
    claims: BTreeMap<(u8, String), ConformanceClaim>,
    scopes: BTreeMap<String, DeclaredScope>,
    reasons: BTreeSet<&'static str>,
) -> PrBodyClaimExtraction {
    let outcome = if reasons.is_empty() {
        PrBodyExtractionOutcome::Extracted
    } else {
        PrBodyExtractionOutcome::NotEvaluated
    };

    PrBodyClaimExtraction {
        source,
        claims: claims.into_values().collect(),
        declared_scopes: scopes.into_values().collect(),
        outcome,
        reasons: reasons.into_iter().collect(),
    }
}

fn parse_member(
    line: &str,
    claims: &mut BTreeMap<(u8, String), ConformanceClaim>,
    scopes: &mut BTreeMap<String, DeclaredScope>,
    reasons: &mut BTreeSet<&'static str>,
) {
    if let Some(value) = line.strip_prefix("claim: ") {
        let kind = match value {
            "documentation-only" | "test-only" => ClaimKind::FileClass,
            "no-behaviour-change" | "refactor-only" => ClaimKind::GraphSemantic,
            _ => {
                reasons.insert("claim.pr-body.claim-unknown");
                return;
            }
        };
        let value = value.to_owned();
        claims
            .entry((claim_kind_order(kind), value.clone()))
            .or_insert(ConformanceClaim {
                kind,
                value,
                source_index: 0,
            });
        return;
    }

    if let Some(prefix) = line.strip_prefix("scope: path:") {
        if prefix.len() > PR_BODY_SCOPE_MAX_BYTES {
            reasons.insert("claim.pr-body.budget.scope-bytes");
            return;
        }
        if !valid_path_prefix(prefix) {
            reasons.insert("claim.pr-body.scope-invalid");
            return;
        }
        let prefix = prefix.to_owned();
        let label = format!("path:{prefix}");
        if !scopes.contains_key(&label) && scopes.len() >= PR_BODY_MAX_SCOPES {
            reasons.insert("claim.pr-body.budget.scopes");
            return;
        }
        claims
            .entry((claim_kind_order(ClaimKind::PathPrefix), prefix.clone()))
            .or_insert(ConformanceClaim {
                kind: ClaimKind::PathPrefix,
                value: prefix.clone(),
                source_index: 0,
            });
        scopes.entry(label.clone()).or_insert(DeclaredScope {
            label,
            authority: ScopeAuthority::ExplicitPath { prefix },
            source_index: 0,
        });
        return;
    }

    reasons.insert("claim.pr-body.member-malformed");
}

fn parse_markdown_fence(line: &str) -> Option<MarkdownFence> {
    let leading_spaces = line.bytes().take_while(|byte| *byte == b' ').count();
    if leading_spaces > 3 {
        return None;
    }
    let content = &line.as_bytes()[leading_spaces..];
    let marker = *content.first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = content.iter().take_while(|byte| **byte == marker).count();
    if length < 3 {
        return None;
    }
    if marker == b'`' && content[length..].contains(&b'`') {
        return None;
    }
    Some(MarkdownFence { marker, length })
}

fn closes_markdown_fence(line: &str, fence: MarkdownFence) -> bool {
    let leading_spaces = line.bytes().take_while(|byte| *byte == b' ').count();
    if leading_spaces > 3 {
        return false;
    }
    let content = &line.as_bytes()[leading_spaces..];
    let length = content
        .iter()
        .take_while(|byte| **byte == fence.marker)
        .count();
    length >= fence.length && content[length..].iter().all(u8::is_ascii_whitespace)
}

const fn claim_kind_order(kind: ClaimKind) -> u8 {
    match kind {
        ClaimKind::FileClass => 0,
        ClaimKind::PathPrefix => 1,
        ClaimKind::GraphSemantic => 2,
    }
}
