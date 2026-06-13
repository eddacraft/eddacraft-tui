//! DSV-005 Task 8: the save-time `validate_paths` verdict path.
//!
//! This module assembles the [`ValidatePathsResponse`] the daemon returns for a
//! client's change set: classify each path, certify it against the warm
//! `(SymbolGraph, DependencyGraph)` cache, run the antipattern family over the
//! openat2-guarded bytes, and fold the result into the workspace assurance
//! state. The orchestration that wires those steps to the live daemon (auth,
//! the guarded read, the per-worktree cache, the ipc dispatch arm) lands on top
//! of the pure pieces here.
//!
//! This first slice provides the **boundary mappings** between the
//! graph-cache-local / wire vocabularies — the translations DSV-004 deferred to
//! "the daemon boundary (DSV-005)". They are pure and exhaustively tested so the
//! verdict assembly built on them cannot silently mistranslate a reason (a
//! mistranslation on the verdict path is a B2 false-attestation hazard).

use std::path::PathBuf;

use anvil_checks::antipattern::check::run_antipattern_check_bytes;
use anvil_checks::antipattern::types::{AntipatternCheckConfig, Warning, WarningSeverity};
use anvil_graph_cache::HotReadApi;
use anvil_graph_cache::certify::{Certifiability, CertifyStale, ChangeKind};
use anvil_intercept_proto::protocol::{
    ChangeDescriptor, ChangeKindWire, CheckFamily, Coverage, EvaluatedPath, StaleReason,
    ValidatePathsRequest, ValidatePathsResponse,
};
use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{
    Category, Diagnostic, DiagnosticSource, FileSymbols, Location, Mode, Severity,
};
use sha2::{Digest, Sha256};

use crate::assurance::{AssuranceMachine, ChangeCtx, taxonomy_reason};
use crate::change_class::CanonicalChange;
use crate::kernel_cache::KernelGraphCache;
use crate::rule_cache::WorktreeKey;
use crate::workspace_pool::DosCaps;

/// `source_module` stamped on every save-time antipattern diagnostic.
const ANTIPATTERN_SOURCE_MODULE: &str = "anvil-checks::antipattern";

/// Rule id stamped on the parse-size-cap coverage diagnostic (DSV-006 / Task
/// 11). Stable so a consumer can group / suppress it.
const PARSE_SIZE_CAP_RULE_ID: &str = "intercept-parse-size-cap";

/// `source_module` for daemon-emitted resource-cap diagnostics — distinct from
/// the antipattern family, which never produced this.
const DOS_SOURCE_MODULE: &str = "anvil-intercept::dos";

/// Map a wire [`ChangeKindWire`] to the daemon's internal [`CanonicalChange`]
/// (the taxonomy input). A rename carries its prior path through.
#[must_use]
pub fn canonical_change(wire: &ChangeKindWire) -> CanonicalChange {
    match wire {
        ChangeKindWire::Created => CanonicalChange::Create,
        ChangeKindWire::Modified => CanonicalChange::ContentModify,
        ChangeKindWire::Deleted => CanonicalChange::Delete,
        ChangeKindWire::Renamed { from } => CanonicalChange::Rename {
            from: PathBuf::from(from),
        },
    }
}

/// Map a wire [`ChangeKindWire`] to the graph-cache [`ChangeKind`] certify
/// consumes. The rename's `from` path is irrelevant to certifiability and is
/// dropped (cf. [`ChangeKind`]).
#[must_use]
pub fn certify_change_kind(wire: &ChangeKindWire) -> ChangeKind {
    match wire {
        ChangeKindWire::Created => ChangeKind::Create,
        ChangeKindWire::Modified => ChangeKind::ContentModify,
        ChangeKindWire::Deleted => ChangeKind::Delete,
        ChangeKindWire::Renamed { .. } => ChangeKind::Rename,
    }
}

/// Map a graph-cache-local [`CertifyStale`] to the wire [`StaleReason`] (the
/// translation DSV-004 left to "the daemon boundary").
///
/// Direct counterparts map 1:1. Two graph-cache reasons have no dedicated wire
/// variant, so DSV-005 chooses the closest fail-safe wire reason:
/// - [`CertifyStale::ExportSurfaceChange`] → [`StaleReason::CrossFileResolutionNeeded`]:
///   a public/privileged surface change means importers must be revalidated —
///   exactly "needs cross-file resolution the warm cache cannot supply".
/// - [`CertifyStale::UnreliableGraph`] → [`StaleReason::CrossFileResolutionNeeded`]:
///   a failed `update_file` leaves the warm state untrustworthy, so the verdict
///   conservatively asks for a fresh cross-file resolution rather than claiming
///   a more specific cause it cannot stand behind.
#[must_use]
pub fn wire_stale_reason(reason: CertifyStale) -> StaleReason {
    match reason {
        CertifyStale::ImpactSetOverflow => StaleReason::ImpactSetOverflow,
        CertifyStale::Deleted => StaleReason::Deleted,
        CertifyStale::Renamed => StaleReason::Renamed,
        CertifyStale::CrossFileResolutionNeeded => StaleReason::CrossFileResolutionNeeded,
        CertifyStale::ExportSurfaceChange | CertifyStale::UnreliableGraph => {
            StaleReason::CrossFileResolutionNeeded
        }
    }
}

/// Map an antipattern [`WarningSeverity`] to the canonical diagnostic
/// [`Severity`].
#[must_use]
pub fn diagnostic_severity(severity: WarningSeverity) -> Severity {
    match severity {
        WarningSeverity::Error => Severity::Error,
        WarningSeverity::Warning => Severity::Warning,
        WarningSeverity::Info => Severity::Info,
    }
}

/// Convert one antipattern [`Warning`] into a canonical save-time
/// [`Diagnostic`] (`anvil.diagnostic.v1`, category `Antipattern`, mode
/// `SaveTime`). The warning's suggestion, when non-empty, rides as the
/// remediation hint.
#[must_use]
pub fn antipattern_diagnostic(warning: &Warning) -> Diagnostic {
    let location = Location {
        file: warning.location.file.clone(),
        line: u32::try_from(warning.location.line).ok(),
        column: warning.location.column.and_then(|c| u32::try_from(c).ok()),
        end_line: warning
            .location
            .end_line
            .and_then(|l| u32::try_from(l).ok()),
        end_column: warning
            .location
            .end_column
            .and_then(|c| u32::try_from(c).ok()),
    };
    let diagnostic = Diagnostic::new(
        warning.id.clone(),
        diagnostic_severity(warning.severity),
        warning.message.clone(),
        location,
        Category::Antipattern,
        DiagnosticSource {
            rule_id: warning.id.clone(),
            source_module: ANTIPATTERN_SOURCE_MODULE.to_string(),
        },
        Mode::known(KnownMode::SaveTime),
    );
    if warning.suggestion.trim().is_empty() {
        diagnostic
    } else {
        diagnostic.with_remediation_hint(warning.suggestion.clone())
    }
}

/// Canonical wire order for save-time diagnostics: sort by `(path, rule_id,
/// span_start)` — file, then rule id, then the span as the scanner reports it
/// (`line, column, end_line, end_column`; line is 1-based, column is whatever
/// the antipattern scanner emits — not normalised here), then `summary` as the
/// final tiebreaker.
///
/// This is the **shared sort-before-envelope normalisation** the cross-path
/// diagnostic-parity gate (DSV-009 / ADR-061 §8) depends on: every surface that
/// assembles a diagnostic envelope (the daemon `validate_paths` path here, and
/// the `watch`/`anvil check` fallback) orders findings by the same key, so two
/// paths that discover the same antipattern findings in different encounter
/// orders still emit byte-identical envelopes.
///
/// The key is a **total order** over distinct diagnostics: the span +
/// `summary` tiebreakers mean two findings only compare `Equal` when they are
/// genuine duplicates (same rule, span, and message). Without them, ties would
/// fall back to encounter order — which differs between the bytes path and the
/// disk path and is `rayon`-scheduling-dependent — so the gate could flake or
/// pass falsely. Because the key is total, `sort_unstable_by` is correct and
/// avoids the stable sort's auxiliary allocation. `Option` span fields sort
/// `None` before `Some`, which is deterministic.
pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_unstable_by(|a, b| {
        (
            a.location.file.as_str(),
            a.source.rule_id.as_str(),
            a.location.line,
            a.location.column,
            a.location.end_line,
            a.location.end_column,
            a.summary.as_str(),
        )
            .cmp(&(
                b.location.file.as_str(),
                b.source.rule_id.as_str(),
                b.location.line,
                b.location.column,
                b.location.end_line,
                b.location.end_column,
                b.summary.as_str(),
            ))
    });
}

/// Hex-encoded SHA-256 of the daemon-read bytes — the authoritative content
/// hash echoed in `evaluated[]`. The client's `content_hash` hint is **never**
/// used for this (the daemon re-reads under the openat2 guard).
#[must_use]
pub fn content_hash(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Whether a wire change has on-disk bytes the daemon should read + hash.
/// `Deleted` and `Renamed` carry no readable content (cf. [`EvaluatedPath`]).
fn change_has_bytes(change: &ChangeKindWire) -> bool {
    matches!(change, ChangeKindWire::Created | ChangeKindWire::Modified)
}

/// The coverage diagnostic emitted when a file exceeds the parse-size `DoS` cap
/// (Task 11). It is a *coverage notice*, not a finding: the antipattern family
/// never ran on this path and the verdict cannot certify it. Severity is
/// `Warning` per "warnings over blocks" — the save still proceeds; the user is
/// told that coverage was reduced for this one path.
fn oversized_diagnostic(path: &str, size: usize, cap: usize) -> Diagnostic {
    Diagnostic::new(
        PARSE_SIZE_CAP_RULE_ID,
        Severity::Warning,
        format!(
            "file skipped: {size} bytes exceeds the {cap}-byte save-time parse-size cap; \
             antipattern coverage is reduced for this path until it is split or excluded"
        ),
        Location {
            file: path.to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Other,
        DiagnosticSource {
            rule_id: PARSE_SIZE_CAP_RULE_ID.to_string(),
            source_module: DOS_SOURCE_MODULE.to_string(),
        },
        Mode::known(KnownMode::SaveTime),
    )
}

/// The non-per-path environment for a [`validate_paths`] run: the antipattern
/// config, the interactive rayon pool the scan runs on (DSV-006/Task 10), and
/// the reverse-impact certify budget. Grouped so the entry point stays readable.
pub struct ValidateEnv<'a> {
    /// Antipattern check configuration (patterns, extensions, threshold).
    pub config: &'a AntipatternCheckConfig,
    /// The pool the antipattern scan executes on.
    pub pool: &'a rayon::ThreadPool,
    /// Reverse-impact closure budget passed to `certify`.
    pub budget: usize,
    /// The reverse-impact hop depth passed to `certify` — the GV2-026 runtime
    /// lever (ADR-063 §3). Resolved once per run by the config layer
    /// (`save_time`) and clamped into `1..=MAX_REVERSE_IMPACT_DEPTH`; the hot
    /// path never re-resolves it.
    pub reverse_impact_depth: u32,
    /// Per-workspace `DoS` caps (DSV-006 / Task 11). The parse-size cap is
    /// applied per path here; the walk-depth cap governs the background scan
    /// executor (cf. [`walk_capped`](crate::workspace_pool::walk_capped)).
    pub caps: &'a DosCaps,
}

/// The per-path outcome the orchestration folds into the response.
struct PathOutcome {
    evaluated: EvaluatedPath,
    /// `(path, bytes)` for the antipattern scan; `None` when unreadable or
    /// skipped by the parse-size cap.
    scanned: Option<(String, Vec<u8>)>,
    /// `true` when the change is graph-certifiable self-contained.
    graph_certified: bool,
    /// The staleness cause when not graph-certified.
    stale_reason: Option<StaleReason>,
    /// A daemon-emitted diagnostic for this path (currently only the
    /// parse-size-cap coverage notice); folded into the response alongside the
    /// antipattern findings.
    diagnostic: Option<Diagnostic>,
}

/// Run the save-time verdict for one change set against the warm cache.
///
/// For each (coalesced) path: classify it (taxonomy), read the openat2-guarded
/// bytes and compute the **daemon's** content hash, certify a `ContentModify`
/// against the warm `(SymbolGraph, DependencyGraph)` cache, and collect bytes
/// for the antipattern scan. The verdict is then assembled:
/// - `coverage = Certified` iff **every** path is graph-certifiable self-contained
///   **and** the antipattern family found nothing blocking; otherwise `Partial`.
///   `check_families` is always `[antipattern]` (the family that ran).
/// - `workspace_assurance` is driven by **graph** certifiability only (an
///   antipattern finding is a diagnostic, not a workspace-staleness cause): an
///   uncertifiable change marks the workspace stale via the [`AssuranceMachine`].
/// - `evaluated[]` echoes the daemon-computed hash, never the client hint.
///
/// Dependencies are injected so the orchestration is testable without a live
/// daemon: `read_guarded` is the Task 3 openat2 read; `fed_symbols` is the
/// injected `SymbolParser` feed (DSV-005 — wired from `anvil-cli` via
/// `ForegroundOpts::with_symbol_parser`), called with the **same** guarded bytes
/// just read. Until a parser is wired (or for an unparseable file) it yields
/// `None`, so that path cannot be certified and is conservatively
/// `Partial(CrossFileResolutionNeeded)`, which is safe.
pub fn validate_paths<R, F>(
    request: &ValidatePathsRequest,
    cache: &KernelGraphCache,
    assurance: &mut AssuranceMachine,
    read_guarded: R,
    fed_symbols: F,
    env: &ValidateEnv<'_>,
) -> ValidatePathsResponse
where
    R: Fn(&str) -> std::io::Result<Vec<u8>>,
    // `fed_symbols` is handed the **exact** guarded bytes the daemon read and
    // hashed for this path, so the parsed symbols provably describe the bytes
    // the verdict attests — never a second read that could race the edit
    // (a B2 false-attestation hazard).
    F: Fn(&str, &[u8]) -> Option<FileSymbols>,
{
    let key = WorktreeKey::from_canonical(PathBuf::from(&request.workspace_root));

    // Coalesce: keep the last descriptor per path. The daemon re-reads current
    // bytes, so same-path descriptors resolve to one daemon hash (identical-hash
    // collapse); a later descriptor for the same path wins (distinct-hash → latest).
    let mut order: Vec<String> = Vec::new();
    let mut last: std::collections::HashMap<String, &ChangeDescriptor> =
        std::collections::HashMap::new();
    for desc in &request.paths {
        if last.insert(desc.path.clone(), desc).is_none() {
            order.push(desc.path.clone());
        }
    }

    let mut outcomes: Vec<PathOutcome> = order
        .iter()
        .map(|path| {
            let desc = last[path];
            per_path_outcome(desc, &key, cache, &read_guarded, &fed_symbols, env)
        })
        .collect();

    // Antipattern scan over the guarded bytes (B7: bytes + injected pool).
    let scanned: Vec<(&str, &[u8])> = outcomes
        .iter()
        .filter_map(|o| o.scanned.as_ref().map(|(p, b)| (p.as_str(), b.as_slice())))
        .collect();
    let antipattern = run_antipattern_check_bytes(
        &scanned,
        env.config,
        Some(request.workspace_root.as_str()),
        env.pool,
    );
    // Collect daemon-emitted per-path diagnostics (parse-size-cap notices) and
    // the antipattern findings into one vec; `sort_diagnostics` below imposes
    // the canonical wire order (the two kinds interleave by path — neither is
    // "first"). `take` moves the diagnostic out rather than cloning its `String`
    // fields — the later `outcomes.into_iter()` only reads `evaluated`, so the
    // emptied `diagnostic` is never observed again.
    let mut diagnostics: Vec<Diagnostic> = outcomes
        .iter_mut()
        .filter_map(|o| o.diagnostic.take())
        .collect();
    diagnostics.extend(
        antipattern
            .warnings
            .warnings
            .iter()
            .map(antipattern_diagnostic),
    );
    // Shared sort-before-envelope normalisation (DSV-009 / ADR-061 §8): emit the
    // wire in canonical `(path, rule_id, span_start)` order — a total order with
    // full-span + `summary` tiebreakers (see `sort_diagnostics`) — so the
    // cross-path parity gate holds regardless of per-path encounter order.
    sort_diagnostics(&mut diagnostics);

    let graph_certified = outcomes.iter().all(|o| o.graph_certified);
    let first_stale = outcomes.iter().find_map(|o| o.stale_reason);

    // Workspace assurance is driven by GRAPH certifiability only.
    assurance.record_verdict(graph_certified, first_stale);

    let coverage = if graph_certified && antipattern.passed {
        Coverage::Certified
    } else {
        Coverage::Partial
    };

    ValidatePathsResponse {
        diagnostics,
        evaluated: outcomes.into_iter().map(|o| o.evaluated).collect(),
        workspace_assurance: assurance.snapshot(),
        coverage,
        check_families: vec![CheckFamily::Antipattern],
    }
}

/// Compute one path's verdict contribution (read + hash + classify + certify).
fn per_path_outcome<R, F>(
    desc: &ChangeDescriptor,
    key: &WorktreeKey,
    cache: &KernelGraphCache,
    read_guarded: &R,
    fed_symbols: &F,
    env: &ValidateEnv<'_>,
) -> PathOutcome
where
    R: Fn(&str) -> std::io::Result<Vec<u8>>,
    F: Fn(&str, &[u8]) -> Option<FileSymbols>,
{
    let budget = env.budget;
    let reverse_impact_depth = env.reverse_impact_depth;
    let caps = env.caps;
    let canonical = canonical_change(&desc.change);
    let ctx = ChangeCtx::for_path(&desc.path, false);
    let taxonomy = taxonomy_reason(&canonical, &ctx);

    // Read guarded bytes + daemon hash for content-bearing changes, enforcing
    // the parse-size DoS cap (Task 11) before any parse/scan/hash work. Two
    // layers: the *read* is already bounded to
    // `path_safety::MAX_GUARDED_READ_BYTES` (a file beyond the hard memory
    // ceiling is refused at the read), and this softer `max_parse_bytes` cap
    // skips the *super-linear* parse + antipattern scan for a file that read OK
    // but is still too big to scan — detected on its read length and skipped
    // (never parsed, scanned, or hashed) with a coverage diagnostic.
    let mut oversized: Option<usize> = None;
    let (content_h, scanned) = if change_has_bytes(&desc.change) {
        match read_guarded(&desc.path) {
            Ok(bytes) if bytes.len() > caps.max_parse_bytes => {
                oversized = Some(bytes.len());
                (None, None)
            }
            Ok(bytes) => (Some(content_hash(&bytes)), Some((desc.path.clone(), bytes))),
            // A read failure (gone, symlink rejected by the openat2 guard, or
            // over the guarded-read ceiling) is a staleness signal, not
            // certifiable.
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    let evaluated = EvaluatedPath {
        path: desc.path.clone(),
        content_hash: content_h,
    };

    // Oversized: skipped before parse/scan. It cannot be certified (the warm
    // cache never saw its symbols) and carries a coverage diagnostic.
    if let Some(size) = oversized {
        return PathOutcome {
            evaluated,
            scanned: None,
            graph_certified: false,
            stale_reason: Some(StaleReason::CrossFileResolutionNeeded),
            diagnostic: Some(oversized_diagnostic(&desc.path, size, caps.max_parse_bytes)),
        };
    }

    // Certifiability. taxonomy `Some` (delete/rename/create/gitignore/config/...)
    // is non-certifiable up front. Only a plain ContentModify reaches certify.
    if let Some(reason) = taxonomy {
        return PathOutcome {
            evaluated,
            scanned,
            graph_certified: false,
            stale_reason: Some(reason),
            diagnostic: None,
        };
    }

    // ContentModify with no readable bytes ⇒ cannot certify.
    let Some((_, bytes)) = scanned.as_ref() else {
        return PathOutcome {
            evaluated,
            scanned,
            graph_certified: false,
            stale_reason: Some(StaleReason::CrossFileResolutionNeeded),
            diagnostic: None,
        };
    };

    // Certify against the warm cache, but only if the feed parsed THIS path's
    // guarded bytes into symbols (the daemon never parses — the symbols come
    // from the injected kernel-backed parser over the exact bytes just read).
    // Until a parser is wired: Partial.
    let Some(symbols) = fed_symbols(&desc.path, bytes) else {
        return PathOutcome {
            evaluated,
            scanned,
            graph_certified: false,
            stale_reason: Some(StaleReason::CrossFileResolutionNeeded),
            diagnostic: None,
        };
    };

    let outcome = cache.apply_delta(key, ChangeKind::ContentModify, symbols);
    // GV2-027 (A→A′): certify through the resident GV2 hot-read index
    // (`HotReadApi`) rather than the raw graph pair, so the warm hot-index is
    // the live backing behind the unchanged `validate_paths` wire. Verdict is
    // identical to the interim direct-`certify` backing by construction (proven
    // over arbitrary delta sequences by `backing_parity`).
    let verdict = cache
        .with_graphs(key, |sym, dep| {
            HotReadApi::new(sym, dep).certify(
                &ChangeKind::ContentModify,
                &outcome.delta,
                budget,
                reverse_impact_depth,
            )
        })
        .unwrap_or(Certifiability::Partial {
            reason: CertifyStale::CrossFileResolutionNeeded,
        });

    match verdict {
        Certifiability::Certified { .. } => PathOutcome {
            evaluated,
            scanned,
            graph_certified: true,
            stale_reason: None,
            diagnostic: None,
        },
        Certifiability::Partial { reason } => PathOutcome {
            evaluated,
            scanned,
            graph_certified: false,
            stale_reason: Some(wire_stale_reason(reason)),
            diagnostic: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_checks::antipattern::types::{Confidence, Location as WarnLocation, WarningCategory};

    #[test]
    fn canonical_change_maps_every_wire_kind() {
        assert_eq!(
            canonical_change(&ChangeKindWire::Created),
            CanonicalChange::Create
        );
        assert_eq!(
            canonical_change(&ChangeKindWire::Modified),
            CanonicalChange::ContentModify
        );
        assert_eq!(
            canonical_change(&ChangeKindWire::Deleted),
            CanonicalChange::Delete
        );
        assert_eq!(
            canonical_change(&ChangeKindWire::Renamed {
                from: "old/path.ts".to_string()
            }),
            CanonicalChange::Rename {
                from: PathBuf::from("old/path.ts")
            }
        );
    }

    #[test]
    fn certify_change_kind_drops_rename_from() {
        assert_eq!(
            certify_change_kind(&ChangeKindWire::Created),
            ChangeKind::Create
        );
        assert_eq!(
            certify_change_kind(&ChangeKindWire::Modified),
            ChangeKind::ContentModify
        );
        assert_eq!(
            certify_change_kind(&ChangeKindWire::Deleted),
            ChangeKind::Delete
        );
        assert_eq!(
            certify_change_kind(&ChangeKindWire::Renamed {
                from: "old.ts".to_string()
            }),
            ChangeKind::Rename
        );
    }

    #[test]
    fn wire_stale_reason_maps_direct_counterparts() {
        assert_eq!(
            wire_stale_reason(CertifyStale::ImpactSetOverflow),
            StaleReason::ImpactSetOverflow
        );
        assert_eq!(
            wire_stale_reason(CertifyStale::Deleted),
            StaleReason::Deleted
        );
        assert_eq!(
            wire_stale_reason(CertifyStale::Renamed),
            StaleReason::Renamed
        );
        assert_eq!(
            wire_stale_reason(CertifyStale::CrossFileResolutionNeeded),
            StaleReason::CrossFileResolutionNeeded
        );
    }

    #[test]
    fn wire_stale_reason_maps_unwired_reasons_to_cross_file_resolution() {
        // No dedicated wire variant — both conservatively ask for cross-file
        // resolution rather than over-claiming a specific cause.
        assert_eq!(
            wire_stale_reason(CertifyStale::ExportSurfaceChange),
            StaleReason::CrossFileResolutionNeeded
        );
        assert_eq!(
            wire_stale_reason(CertifyStale::UnreliableGraph),
            StaleReason::CrossFileResolutionNeeded
        );
    }

    #[test]
    fn diagnostic_severity_maps_all_levels() {
        assert_eq!(diagnostic_severity(WarningSeverity::Error), Severity::Error);
        assert_eq!(
            diagnostic_severity(WarningSeverity::Warning),
            Severity::Warning
        );
        assert_eq!(diagnostic_severity(WarningSeverity::Info), Severity::Info);
    }

    fn sample_warning(suggestion: &str) -> Warning {
        Warning {
            id: "AP-001".to_string(),
            fingerprint: None,
            category: WarningCategory::AntiPattern,
            severity: WarningSeverity::Warning,
            confidence: Confidence::High,
            title: "Avoid any".to_string(),
            message: "`any` defeats the type checker".to_string(),
            explanation: String::new(),
            suggestion: suggestion.to_string(),
            nudge: None,
            location: WarnLocation {
                file: "src/app.ts".to_string(),
                line: 12,
                column: Some(4),
                end_line: None,
                end_column: None,
            },
            pattern: None,
            suppressed: None,
            family: None,
            definition_ref: None,
            spectrum_position: None,
        }
    }

    #[test]
    fn antipattern_diagnostic_carries_canonical_fields() {
        let d = antipattern_diagnostic(&sample_warning("use a precise type"));
        assert_eq!(d.id, "AP-001");
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.summary, "`any` defeats the type checker");
        assert_eq!(d.location.file, "src/app.ts");
        assert_eq!(d.location.line, Some(12));
        assert_eq!(d.location.column, Some(4));
        assert_eq!(d.category, Category::Antipattern);
        assert_eq!(d.source.rule_id, "AP-001");
        assert_eq!(d.source.source_module, ANTIPATTERN_SOURCE_MODULE);
        assert_eq!(d.mode, Mode::known(KnownMode::SaveTime));
        assert_eq!(d.remediation_hint.as_deref(), Some("use a precise type"));
    }

    #[test]
    fn antipattern_diagnostic_omits_empty_suggestion_hint() {
        let d = antipattern_diagnostic(&sample_warning("   "));
        assert_eq!(
            d.remediation_hint, None,
            "a blank suggestion must not become a placeholder hint"
        );
    }

    // ---- orchestration ----

    use anvil_kernel_types::{ImportEdge, SymbolKind, SymbolNode, TrustLevel, Visibility};
    use std::collections::HashMap;

    fn pool() -> rayon::ThreadPool {
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("test pool")
    }

    fn wt() -> WorktreeKey {
        WorktreeKey::from_canonical(PathBuf::from("/wt"))
    }

    /// A `FileSymbols` for `file` with public functions `names` and import
    /// specifiers `imports`, ids from `base`.
    fn file_symbols(file: &str, names: &[&str], imports: &[&str], base: u64) -> FileSymbols {
        FileSymbols {
            file: file.to_string(),
            symbols: names
                .iter()
                .enumerate()
                .map(|(i, n)| SymbolNode {
                    id: base + i as u64,
                    kind: SymbolKind::Function,
                    name: (*n).to_string(),
                    visibility: Visibility::Public,
                    file: file.to_string(),
                    trust_level: TrustLevel::Unknown,
                })
                .collect(),
            imports: imports
                .iter()
                .map(|src| ImportEdge {
                    from_file: file.to_string(),
                    to_source: (*src).to_string(),
                    line: 0,
                })
                .collect(),
            reexports: Vec::new(),
        }
    }

    fn desc(path: &str, change: ChangeKindWire, hint: Option<&str>) -> ChangeDescriptor {
        ChangeDescriptor {
            path: path.to_string(),
            change,
            content_hash: hint.map(str::to_string),
            mtime: None,
        }
    }

    fn request(paths: Vec<ChangeDescriptor>) -> ValidatePathsRequest {
        ValidatePathsRequest {
            workspace_root: "/wt".to_string(),
            paths,
        }
    }

    #[test]
    fn evaluated_echoes_daemon_computed_hash_not_client_hint() {
        // Client sends a bogus hint; the daemon re-reads and echoes ITS hash.
        let bytes = b"const value = 1;".to_vec();
        let expected = content_hash(&bytes);
        let reads: HashMap<String, Vec<u8>> =
            HashMap::from([("src/a.ts".to_string(), bytes.clone())]);

        let cache = KernelGraphCache::new();
        let mut assurance = AssuranceMachine::new();
        let resp = validate_paths(
            &request(vec![desc(
                "src/a.ts",
                ChangeKindWire::Modified,
                Some("deadbeef-bogus-client-hash"),
            )]),
            &cache,
            &mut assurance,
            |p| {
                reads
                    .get(p)
                    .cloned()
                    .ok_or(std::io::ErrorKind::NotFound.into())
            },
            |_, _| None,
            &ValidateEnv {
                config: &AntipatternCheckConfig::default(),
                pool: &pool(),
                budget: 64,
                reverse_impact_depth: 1,
                caps: &DosCaps::default(),
            },
        );

        assert_eq!(resp.evaluated.len(), 1);
        assert_eq!(resp.evaluated[0].path, "src/a.ts");
        assert_eq!(
            resp.evaluated[0].content_hash.as_deref(),
            Some(expected.as_str()),
            "evaluated echoes the daemon-computed hash, never the client hint"
        );
        assert_ne!(
            resp.evaluated[0].content_hash.as_deref(),
            Some("deadbeef-bogus-client-hash")
        );
    }

    #[test]
    fn client_supplied_hash_not_trusted_for_verdict() {
        // Even with no fed symbols (so certify can't run), the verdict is driven
        // by the daemon's reading, not the client hint — result is Partial and
        // the evaluated hash is the daemon's.
        let bytes = b"const value = 1;".to_vec();
        let reads: HashMap<String, Vec<u8>> =
            HashMap::from([("src/a.ts".to_string(), bytes.clone())]);
        let cache = KernelGraphCache::new();
        let mut assurance = AssuranceMachine::new();
        let resp = validate_paths(
            &request(vec![desc(
                "src/a.ts",
                ChangeKindWire::Modified,
                Some("client-says-clean"),
            )]),
            &cache,
            &mut assurance,
            |p| {
                reads
                    .get(p)
                    .cloned()
                    .ok_or(std::io::ErrorKind::NotFound.into())
            },
            |_, _| None, // feed has not delivered symbols → cannot certify
            &ValidateEnv {
                config: &AntipatternCheckConfig::default(),
                pool: &pool(),
                budget: 64,
                reverse_impact_depth: 1,
                caps: &DosCaps::default(),
            },
        );
        assert_eq!(resp.coverage, Coverage::Partial);
        assert_eq!(
            resp.evaluated[0].content_hash.as_deref(),
            Some(content_hash(&bytes).as_str())
        );
    }

    #[test]
    fn coalesce_collapses_identical_path_to_one_evaluated() {
        let bytes = b"const value = 1;".to_vec();
        let reads: HashMap<String, Vec<u8>> = HashMap::from([("src/a.ts".to_string(), bytes)]);
        let cache = KernelGraphCache::new();
        let mut assurance = AssuranceMachine::new();
        let resp = validate_paths(
            &request(vec![
                desc("src/a.ts", ChangeKindWire::Modified, None),
                desc("src/a.ts", ChangeKindWire::Modified, None),
            ]),
            &cache,
            &mut assurance,
            |p| {
                reads
                    .get(p)
                    .cloned()
                    .ok_or(std::io::ErrorKind::NotFound.into())
            },
            |_, _| None,
            &ValidateEnv {
                config: &AntipatternCheckConfig::default(),
                pool: &pool(),
                budget: 64,
                reverse_impact_depth: 1,
                caps: &DosCaps::default(),
            },
        );
        assert_eq!(
            resp.evaluated.len(),
            1,
            "two descriptors for the same path collapse to one evaluated entry"
        );
    }

    #[test]
    fn validate_paths_certified_clean_for_self_contained_edit() {
        // Pre-warm the cache so a body-only re-edit (same public surface)
        // certifies self-contained. Clean bytes ⇒ antipattern passes ⇒ Certified.
        let cache = KernelGraphCache::new();
        cache.apply_delta(
            &wt(),
            ChangeKind::Create,
            file_symbols("src/a.ts", &["foo"], &[], 0),
        );

        let clean = b"export function foo() { return 1; }".to_vec();
        let reads: HashMap<String, Vec<u8>> = HashMap::from([("src/a.ts".to_string(), clean)]);
        // Feed delivers the same public surface (foo) — a body-only change.
        let fed = |p: &str, _: &[u8]| {
            (p == "src/a.ts").then(|| file_symbols("src/a.ts", &["foo"], &[], 0))
        };

        let mut assurance = AssuranceMachine::new();
        let resp = validate_paths(
            &request(vec![desc("src/a.ts", ChangeKindWire::Modified, None)]),
            &cache,
            &mut assurance,
            |p| {
                reads
                    .get(p)
                    .cloned()
                    .ok_or(std::io::ErrorKind::NotFound.into())
            },
            fed,
            &ValidateEnv {
                config: &AntipatternCheckConfig::default(),
                pool: &pool(),
                budget: 64,
                reverse_impact_depth: 1,
                caps: &DosCaps::default(),
            },
        );
        assert_eq!(
            resp.coverage,
            Coverage::Certified,
            "a self-contained body-only edit with a clean antipattern scan is certified"
        );
        assert_eq!(resp.check_families, vec![CheckFamily::Antipattern]);
    }

    #[test]
    fn validate_paths_partial_stale_on_overflow() {
        // b.ts imports a.ts; warm both. A public-surface change to a.ts pulls in
        // b.ts; with budget 0 the impact closure overflows ⇒ Partial, and the
        // workspace is marked stale with ImpactSetOverflow.
        let cache = KernelGraphCache::new();
        cache.apply_delta(
            &wt(),
            ChangeKind::Create,
            file_symbols("src/b.ts", &["b_fn"], &["./a"], 0),
        );
        cache.apply_delta(
            &wt(),
            ChangeKind::Create,
            file_symbols("src/a.ts", &["foo"], &[], 10),
        );

        let bytes = b"export function bar() {}".to_vec();
        let reads: HashMap<String, Vec<u8>> = HashMap::from([("src/a.ts".to_string(), bytes)]);
        // Surface change: foo -> bar.
        let fed = |p: &str, _: &[u8]| {
            (p == "src/a.ts").then(|| file_symbols("src/a.ts", &["bar"], &[], 20))
        };

        let mut assurance = AssuranceMachine::new();
        let resp = validate_paths(
            &request(vec![desc("src/a.ts", ChangeKindWire::Modified, None)]),
            &cache,
            &mut assurance,
            |p| {
                reads
                    .get(p)
                    .cloned()
                    .ok_or(std::io::ErrorKind::NotFound.into())
            },
            fed,
            &ValidateEnv {
                config: &AntipatternCheckConfig::default(),
                pool: &pool(),
                budget: 0, // budget 0 ⇒ any importer overflows
                reverse_impact_depth: 1,
                caps: &DosCaps::default(),
            },
        );
        assert_eq!(resp.coverage, Coverage::Partial);
        assert_eq!(
            resp.workspace_assurance.reason,
            Some(StaleReason::ImpactSetOverflow),
            "an overflow marks the workspace stale with ImpactSetOverflow"
        );
    }

    #[test]
    fn oversized_file_skipped_with_diagnostic() {
        // A file past the parse-size cap is never parsed/scanned/hashed: it
        // yields a coverage diagnostic, no daemon hash, and a Partial verdict
        // (it cannot be certified), and the workspace goes stale.
        let big = vec![b'x'; 64]; // 64 bytes
        let reads: HashMap<String, Vec<u8>> =
            HashMap::from([("src/huge.ts".to_string(), big.clone())]);
        let cache = KernelGraphCache::new();
        let mut assurance = AssuranceMachine::new();
        // Cap below the file size so it is treated as oversized; a parser that
        // would otherwise certify proves the skip happens *before* certify.
        let caps = DosCaps {
            max_parse_bytes: 16,
            ..DosCaps::default()
        };
        let fed = |p: &str, _: &[u8]| {
            (p == "src/huge.ts").then(|| file_symbols("src/huge.ts", &["foo"], &[], 0))
        };
        let resp = validate_paths(
            &request(vec![desc("src/huge.ts", ChangeKindWire::Modified, None)]),
            &cache,
            &mut assurance,
            |p| {
                reads
                    .get(p)
                    .cloned()
                    .ok_or(std::io::ErrorKind::NotFound.into())
            },
            fed,
            &ValidateEnv {
                config: &AntipatternCheckConfig::default(),
                pool: &pool(),
                budget: 64,
                reverse_impact_depth: 1,
                caps: &caps,
            },
        );

        assert_eq!(
            resp.coverage,
            Coverage::Partial,
            "oversized ⇒ not certified"
        );
        assert_eq!(
            resp.evaluated[0].content_hash, None,
            "oversized file is not hashed"
        );
        assert_eq!(
            resp.diagnostics.len(),
            1,
            "exactly the parse-size-cap coverage diagnostic"
        );
        let d = &resp.diagnostics[0];
        assert_eq!(d.id, PARSE_SIZE_CAP_RULE_ID);
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.category, Category::Other);
        assert_eq!(d.source.source_module, DOS_SOURCE_MODULE);
        assert_eq!(d.location.file, "src/huge.ts");
        assert!(
            d.summary.contains("64") && d.summary.contains("16"),
            "diagnostic names the size and the cap: {}",
            d.summary
        );
        assert_eq!(
            resp.workspace_assurance.reason,
            Some(StaleReason::CrossFileResolutionNeeded),
            "an uncertifiable oversized change marks the workspace stale"
        );
    }

    #[test]
    fn file_at_cap_is_scanned_not_skipped() {
        // Boundary: a file exactly at the cap is NOT oversized (strict `>`),
        // so it is read + hashed normally.
        let bytes = b"const value = 1;".to_vec(); // 16 bytes
        let cap = bytes.len();
        let reads: HashMap<String, Vec<u8>> =
            HashMap::from([("src/a.ts".to_string(), bytes.clone())]);
        let cache = KernelGraphCache::new();
        let mut assurance = AssuranceMachine::new();
        let caps = DosCaps {
            max_parse_bytes: cap,
            ..DosCaps::default()
        };
        let resp = validate_paths(
            &request(vec![desc("src/a.ts", ChangeKindWire::Modified, None)]),
            &cache,
            &mut assurance,
            |p| {
                reads
                    .get(p)
                    .cloned()
                    .ok_or(std::io::ErrorKind::NotFound.into())
            },
            |_, _| None,
            &ValidateEnv {
                config: &AntipatternCheckConfig::default(),
                pool: &pool(),
                budget: 64,
                reverse_impact_depth: 1,
                caps: &caps,
            },
        );
        assert_eq!(
            resp.evaluated[0].content_hash.as_deref(),
            Some(content_hash(&bytes).as_str()),
            "a file exactly at the cap is read + hashed, not skipped"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "no parse-size diagnostic for an at-cap file"
        );
    }

    #[test]
    fn deleted_path_has_no_daemon_hash_and_is_partial() {
        let cache = KernelGraphCache::new();
        let mut assurance = AssuranceMachine::new();
        let resp = validate_paths(
            &request(vec![desc("src/gone.ts", ChangeKindWire::Deleted, None)]),
            &cache,
            &mut assurance,
            |_| Err(std::io::ErrorKind::NotFound.into()),
            |_, _| None,
            &ValidateEnv {
                config: &AntipatternCheckConfig::default(),
                pool: &pool(),
                budget: 64,
                reverse_impact_depth: 1,
                caps: &DosCaps::default(),
            },
        );
        assert_eq!(resp.evaluated[0].content_hash, None);
        assert_eq!(resp.coverage, Coverage::Partial);
        assert_eq!(resp.workspace_assurance.reason, Some(StaleReason::Deleted));
    }
}
