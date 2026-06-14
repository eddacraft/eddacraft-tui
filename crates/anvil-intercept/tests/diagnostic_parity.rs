//! Cross-path diagnostic parity gate (DSV-009 / ADR-061 §8).
//!
//! Proves the **antipattern-family** finding sets are identical across the two
//! save-time delivery surfaces that run the antipattern engine — `watch+daemon`
//! and `watch+fallback` — order-normalised by `(path, rule_id, span_start)`
//! (a total order — the shared `sort_diagnostics` adds full-span + `summary`
//! tiebreakers so ties never fall back to encounter order),
//! with `workspace_assurance` and daemon-only `DoS` coverage notices carved out.
//!
//! Scope note (the parity model, reconciled to the merged DSV-007 design): the
//! daemon `validate_paths` verb and the `anvil check` fallback both run the
//! **antipattern** family, so this is where antipattern parity is meaningful and
//! achievable. The MCP `anvil_validate_write` surface deliberately stays on the
//! `scan_buffer` verb (secret + launch-reasoning / boundary family — see the
//! `anvil-cli` MCP validation module), and its daemon↔embedded parity is proven
//! separately against `default_rule_registry()` (the `scan_buffer` parity tests
//! in `anvil-intercept` + `anvil-cli`). The two surfaces run different families
//! by design, so there is no single four-path antipattern set to compare; this
//! gate locks the antipattern surfaces, the secret gate locks the MCP surfaces.
//!
//! Both surfaces here run the same engine over the same content — the daemon
//! reads guarded **bytes** (`validate_paths` → `run_antipattern_check_bytes`),
//! the fallback reads from **disk** (`anvil check` → `run_antipattern_check`).
//! The corpus is fed to the two paths in **different orders** on purpose: the
//! shared `sort_diagnostics` normalisation is what makes their envelopes
//! byte-identical regardless of encounter order. Map both through the same
//! canonical `antipattern_diagnostic` projection and assert equality.

use std::collections::HashMap;
use std::path::PathBuf;

use anvil_checks::antipattern::run_antipattern_check;
use anvil_checks::antipattern::types::AntipatternCheckConfig;
use anvil_intercept::assurance::AssuranceMachine;
use anvil_intercept::kernel_cache::KernelGraphCache;
use anvil_intercept::validate_paths::{
    ValidateEnv, antipattern_diagnostic, sort_diagnostics, validate_paths,
};
use anvil_intercept::workspace_pool::DosCaps;
use anvil_intercept_proto::protocol::{ChangeDescriptor, ChangeKindWire, ValidatePathsRequest};
use anvil_kernel_types::diagnostics::Diagnostic;

/// `source_module` the daemon stamps on antipattern findings. `DoS` coverage
/// notices use a different module and are carved out of parity.
const ANTIPATTERN_SOURCE_MODULE: &str = "anvil-checks::antipattern";

/// Workspace-relative corpus files in canonical (alphabetical) order. The
/// daemon path feeds these **reversed** and the fallback feeds them **forward**
/// — opposite encounter orders, so `sort_diagnostics` is the only thing that can
/// equalise the two envelopes (the gate would flake/false-pass without it).
const CORPUS: &[&str] = &["alpha.ts", "beta.ts"];

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity-corpus")
}

fn pool() -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("test pool")
}

fn antipattern_only(diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.source.source_module == ANTIPATTERN_SOURCE_MODULE)
        .cloned()
        .collect()
}

/// `watch+daemon`: drive the real `validate_paths` orchestrator over the guarded
/// bytes the daemon would have read, returning its antipattern diagnostics, with
/// the descriptors fed in the given `order` (so callers can prove the envelope is
/// independent of encounter order).
///
/// NOTE (ADR-061 §7.2 — config discovery): both surfaces use
/// `AntipatternCheckConfig::default()` here, so this proves engine + ordering
/// parity given the *same* config. It does NOT yet prove "shared config discovery
/// off `workspace_root`" — update once workspace-level config loading lands.
fn daemon_path_in_order(
    root: &str,
    reads: &HashMap<String, Vec<u8>>,
    order: &[&str],
) -> Vec<Diagnostic> {
    let paths: Vec<ChangeDescriptor> = order
        .iter()
        .map(|p| ChangeDescriptor {
            path: (*p).to_string(),
            change: ChangeKindWire::Modified,
            content_hash: None,
            mtime: None,
        })
        .collect();
    let request = ValidatePathsRequest {
        workspace_root: root.to_string(),
        paths,
    };
    let cache = KernelGraphCache::new();
    let mut assurance = AssuranceMachine::new();
    let response = validate_paths(
        &request,
        &cache,
        &mut assurance,
        |p| {
            reads
                .get(p)
                .cloned()
                .ok_or_else(|| std::io::ErrorKind::NotFound.into())
        },
        // No fed symbols: certifiability is irrelevant to antipattern parity, and
        // `workspace_assurance` is carved out of the comparison by contract.
        |_, _| None,
        &ValidateEnv {
            config: &AntipatternCheckConfig::default(),
            pool: &pool(),
            budget: 64,
            reverse_impact_depth: 1,
            caps: &DosCaps::default(),
        },
    );
    // `validate_paths` already applies the shared sort; filter to the family.
    antipattern_only(&response.diagnostics)
}

/// The daemon feeds the corpus **reversed** (the fallback feeds forward) so the
/// opposite encounter orders make `sort_diagnostics` the only equaliser — the
/// gate then genuinely tests the sort, not a coincidental input ordering.
fn daemon_path(root: &str, reads: &HashMap<String, Vec<u8>>) -> Vec<Diagnostic> {
    let reversed: Vec<&str> = CORPUS.iter().rev().copied().collect();
    daemon_path_in_order(root, reads, &reversed)
}

/// `watch+fallback`: the daemon-absent path runs `anvil check` over the changed
/// files on disk (`run_antipattern_check`). Fed in **forward** order (the daemon
/// feeds reverse), mapped through the canonical projection, then normalised with
/// the same shared sort.
fn fallback_path(root: &str) -> Vec<Diagnostic> {
    // Forward corpus order here; `daemon_path` feeds reverse. Opposite encounter
    // orders make the shared sort the only thing that can equalise the envelopes.
    let files: Vec<String> = CORPUS
        .iter()
        .map(|p| corpus_dir().join(p).to_string_lossy().into_owned())
        .collect();
    let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();

    let result = run_antipattern_check(&file_refs, &AntipatternCheckConfig::default(), Some(root));
    let mut diagnostics: Vec<Diagnostic> = result
        .warnings
        .warnings
        .iter()
        .map(antipattern_diagnostic)
        .collect();
    sort_diagnostics(&mut diagnostics);
    antipattern_only(&diagnostics)
}

#[test]
fn antipattern_findings_match_across_daemon_and_fallback_paths() {
    let dir = corpus_dir();
    let root = dir.to_string_lossy().into_owned();

    // The daemon reads guarded bytes keyed by the workspace-relative path; the
    // fallback reads the same files from disk. Same content, same engine.
    let reads: HashMap<String, Vec<u8>> = CORPUS
        .iter()
        .map(|p| {
            let bytes = std::fs::read(dir.join(p)).expect("read corpus fixture");
            ((*p).to_string(), bytes)
        })
        .collect();

    let daemon = daemon_path(&root, &reads);
    let fallback = fallback_path(&root);

    // Non-trivial gate: the corpus must actually trip the antipattern engine, or
    // a regression to "no findings" would make parity vacuously true.
    assert!(
        daemon.len() >= 3,
        "parity corpus must produce >= 3 antipattern findings (got {}) — \
         a vacuous empty-set parity is not a gate",
        daemon.len()
    );

    // The load-bearing assertion. `Diagnostic` derives `PartialEq`, so this
    // compares every field (id, severity, summary, location, category, source,
    // remediation_hint, mode, schema_version) in canonical order — proving the
    // bytes path and the disk path emit byte-identical antipattern envelopes
    // despite being fed in opposite orders.
    assert_eq!(
        daemon, fallback,
        "watch+daemon and watch+fallback must return identical antipattern \
         finding sets, order-normalised by (path, rule_id, span_start)"
    );
}

#[test]
fn daemon_envelope_is_independent_of_input_order() {
    // The production envelope must be identical no matter what order the client
    // submits paths in — that is what makes cross-path parity possible. Drive the
    // orchestrator with the corpus forward and reversed and assert byte-identical
    // antipattern envelopes. (This is non-tautological: it compares two *separate*
    // orchestrator runs with different inputs, not a re-sort of one output.)
    let dir = corpus_dir();
    let root = dir.to_string_lossy().into_owned();
    let reads: HashMap<String, Vec<u8>> = CORPUS
        .iter()
        .map(|p| {
            (
                (*p).to_string(),
                std::fs::read(dir.join(p)).expect("read corpus fixture"),
            )
        })
        .collect();

    let forward: Vec<&str> = CORPUS.to_vec();
    let reversed: Vec<&str> = CORPUS.iter().rev().copied().collect();

    let from_forward = daemon_path_in_order(&root, &reads, &forward);
    let from_reversed = daemon_path_in_order(&root, &reads, &reversed);

    assert!(!from_forward.is_empty(), "corpus must produce findings");
    assert_eq!(
        from_forward, from_reversed,
        "the validate_paths envelope must be independent of client path order \
         (the shared sort-before-envelope normalisation must be wired)"
    );
}

/// DSV-010b: extend the DSV-009 cross-path parity gate to the **Windows delivery
/// path**. The Windows daemon reads guarded bytes through the ADR-068
/// `WorkspaceAnchor` (the `NtCreateFile`/`OBJ_DONT_REPARSE` read guard); those
/// bytes must produce the byte-identical antipattern envelope as the disk
/// fallback. This proves the Windows guard reads the *exact* on-disk bytes (no
/// reparse-follow, no truncation, no normalisation drift) the verdict attests —
/// the same parity claim the in-memory `daemon_path` proves for the engine, now
/// carried through the real Windows read primitive.
#[cfg(windows)]
#[test]
fn antipattern_findings_match_across_windows_anchor_and_fallback_paths() {
    use anvil_intercept::workspace_anchor::WorkspaceAnchor;

    let dir = corpus_dir();
    let root = dir.to_string_lossy().into_owned();

    // Read the corpus through the held Windows anchor (the production read
    // primitive), keyed by the workspace-relative path the daemon dispatch uses.
    let anchor = WorkspaceAnchor::open(&dir).expect("open corpus anchor");
    let reads: HashMap<String, Vec<u8>> = CORPUS
        .iter()
        .map(|&p| {
            (
                p.to_string(),
                anchor.read_rel(p).expect("anchor read corpus fixture"),
            )
        })
        .collect();

    let daemon = daemon_path(&root, &reads);
    let fallback = fallback_path(&root);

    assert!(
        daemon.len() >= 3,
        "parity corpus must produce >= 3 antipattern findings (got {}) — \
         a vacuous empty-set parity is not a gate",
        daemon.len()
    );
    assert_eq!(
        daemon, fallback,
        "the Windows anchor-read envelope must be byte-identical to the disk \
         fallback — the ADR-068 guard must read the exact on-disk bytes"
    );
}
