//! Starter-pack end-to-end proof (POLRESET-007 / OPAE-008).
//!
//! One integration-grade proof that drives the whole first-slice policy chain
//! against the **real** embedded `anvil-baseline` starter pack in a tempdir
//! workspace — install, admission, gate evaluation, pre-write routing, and the
//! frozen `anvil policy eval --json` v1 harness. Each `#[test]` covers one
//! stage of the chain; the test names match the `starter_policy_pack` filter.
//!
//! This module is a proof, not production surface: it adds no new behaviour. It
//! reaches the private gate check through a single `#[cfg(test)]` bridge
//! (`gate::run_policy_check_for_proof`) and otherwise drives only public APIs
//! (`commands::policy::run`, `anvil_policy_engine`, `anvil_policy::eval`,
//! `mcp::enforcement`).
//!
//! ## Stage map
//!
//! 1. **Install** — [`starter_policy_pack_installs_with_verified_provenance`]:
//!    the real `anvil policy install` path writes the pack and a provenance
//!    record whose sha256s match the bytes on disk.
//! 2. **Admission** — [`starter_policy_pack_passes_admission_pipeline`]:
//!    `load_manifest` → `validate_pack` → `run_pack_tests` → `enforce_tests`
//!    are all green on the installed copy (the pack's own Rego tests pass
//!    through the regorus facade).
//! 3. **Gate** — [`starter_policy_pack_gate_evaluates_advisory_first`]: the
//!    live gate evaluates the installed pack and surfaces the pack's
//!    warning-class advisory with its remediation-first guidance, while still
//!    passing (exit 0) — advisory-first, ADR-002.
//! 4. **Pre-write** — [`starter_policy_pack_prewrite_projection_feeds_pack`]:
//!    the pre-write [`PrewriteInput`] projection feeds the pack the sensitive
//!    change and the pack fires.
//! 5. **Advisory invariant** —
//!    [`starter_policy_pack_warnings_never_veto_even_under_interrupt`]: routed
//!    through the real `mcp::enforcement::decision_for`, the pack's
//!    warning-family findings never veto — under **any** posture, including
//!    `interrupt`.
//! 6. **Report-only CI exercisability** —
//!    [`starter_policy_pack_evaluates_under_frozen_eval_v1_contract`]: a member
//!    policy evaluates through the same engine pipeline `anvil policy eval`
//!    uses, and the resulting v1 document normalises cleanly through the frozen
//!    `anvil policy eval --json` harness the eval-regression command binds to.
//! 7. **Vocabulary lockstep** —
//!    [`starter_policy_pack_extractor_recognises_documented_rule_families`]:
//!    guards that the pack's rule-set names and the gate's recognised
//!    vocabulary stay in lockstep via the single-source `WARNING_FAMILY_KEYS` /
//!    `VIOLATION_FAMILY_KEYS` consts (the warn/warning gap this proof originally
//!    surfaced, now closed at the extractor).

use std::path::Path;

use anvil_kernel_types::diagnostics::ControlDecision;
use anvil_kernel_types::{
    Category, Diagnostic, DiagnosticSource, EnforcementMode, Location, Mode, Severity,
};
use anvil_policy::eval::{
    EvalHarnessError, EvalHarnessPort, EvalSuite, PolicyEvalAdapter, PolicyEvalRunner, normalise,
};
use anvil_policy_engine::context::{ChangeKind, ChangedPath, WorkflowPhase};
use anvil_policy_engine::pack::{enforce_tests, load_manifest, run_pack_tests, validate_pack};
use anvil_policy_engine::{
    Engine, EngineConfig, GraphFacts, PolicyInput, PrewriteBudget, PrewriteInput,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::GlobalArgs;

/// The bundled pack id under proof.
const PACK_ID: &str = "anvil-baseline";
/// A precise sensitive path: a CI workflow change the pack advises on.
const SENSITIVE_PATH: &str = ".github/workflows/deploy.yml";

// ── Shared harness ──────────────────────────────────────────────────

/// Install the real bundled `anvil-baseline` pack into `workspace` through the
/// production `anvil policy install <pack> --workspace <dir>` command path
/// (`commands::policy::run` → `install::run_install` → `install_pack_files`
/// over the `ANVIL_BASELINE` `BundledPack`). Returns the installed pack
/// directory.
fn install_baseline(workspace: &Path) -> std::path::PathBuf {
    // Parse the real clap surface so the install runs through exactly the same
    // argument handling the CLI uses. `PolicyArgs` is a subcommand group, so it
    // is flattened under a throwaway parser (mirroring the module's own tests).
    use clap::Parser as _;
    #[derive(clap::Parser)]
    struct ProofCli {
        #[command(flatten)]
        inner: super::PolicyArgs,
    }

    let ws = workspace.to_string_lossy().to_string();
    let cli = ProofCli::try_parse_from(["anvil", "install", PACK_ID, "--workspace", &ws])
        .expect("install args parse");
    super::run(&cli.inner, &GlobalArgs::default()).expect("real install path succeeds");

    workspace.join(".anvil/policies").join(PACK_ID)
}

/// The provenance record `anvil policy install` writes beside the pack.
#[derive(Debug, Deserialize)]
struct ProvenanceRecord {
    pack: String,
    version: String,
    installed_from: String,
    files: Vec<ProvenanceEntry>,
}

#[derive(Debug, Deserialize)]
struct ProvenanceEntry {
    path: String,
    sha256: String,
}

/// Lowercase hex sha256 — matches `install::sha256_hex`, used to re-derive the
/// provenance hashes from the bytes on disk.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Load both non-test member policies from an installed pack into a fresh
/// engine and evaluate `query`. Mirrors the gate's admission set: `*.rego`
/// minus `*_test.rego`. Returns the raw query value (`Null` when undefined).
fn eval_installed_pack(pack_dir: &Path, input: &PolicyInput, query: &str) -> serde_json::Value {
    let mut engine = Engine::new(EngineConfig::default()).expect("engine");
    anvil_policy_engine::builtins::register_all(&mut engine).expect("builtins");
    for rel in [
        "policies/change_scope.rego",
        "policies/sensitive_paths.rego",
    ] {
        let src = std::fs::read_to_string(pack_dir.join(rel))
            .unwrap_or_else(|e| panic!("read {rel}: {e}"));
        engine.add_policy(rel.to_string(), src).expect("compile");
    }
    engine
        .eval(input, query)
        .expect("evaluate")
        .value
        .unwrap_or(serde_json::Value::Null)
}

/// Collect every `warning`-rule message from a `data.anvil.policies`-rooted
/// value (an object mapping each sub-package to its rule outputs).
fn collect_warning_messages(value: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(map) = value.as_object() {
        for output in map.values() {
            if let Some(arr) = output.get("warning").and_then(serde_json::Value::as_array) {
                out.extend(arr.iter().filter_map(|m| m.as_str().map(str::to_string)));
            }
        }
    }
    out
}

/// A `PolicyInput` whose diff carries a single sensitive changed path.
fn sensitive_change_input() -> PolicyInput {
    let mut input = PolicyInput::default();
    input.diff.changed_files = vec![SENSITIVE_PATH.to_string()];
    input
}

/// Build a warning-severity [`Diagnostic`] from a pack advisory message, as a
/// pre-write surface would when routing the pack's `warning`-rule output.
fn warning_diagnostic(message: &str) -> Diagnostic {
    Diagnostic::new(
        "starter_pack_advisory",
        Severity::Warning,
        message,
        Location {
            file: SENSITIVE_PATH.to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Policy,
        DiagnosticSource {
            rule_id: "sensitive-paths".to_string(),
            source_module: PACK_ID.to_string(),
        },
        Mode::Unknown("pre-write".to_string()),
    )
}

// ── Stage 1: install with verified provenance ───────────────────────

#[test]
fn starter_policy_pack_installs_with_verified_provenance() {
    let ws = TempDir::new().expect("workspace");
    let pack_dir = install_baseline(ws.path());

    // The pack files and the provenance record are on disk.
    assert!(pack_dir.join("pack.yaml").is_file());
    assert!(pack_dir.join("policies/change_scope.rego").is_file());
    assert!(pack_dir.join("policies/sensitive_paths.rego").is_file());
    let provenance_raw =
        std::fs::read_to_string(pack_dir.join("provenance.yaml")).expect("provenance written");
    let provenance: ProvenanceRecord =
        serde_yaml::from_str(&provenance_raw).expect("provenance parses");

    // Provenance identifies the bundled build and covers exactly the five pack
    // files (manifest + four rego), never itself.
    assert_eq!(provenance.pack, PACK_ID);
    assert!(!provenance.version.is_empty());
    assert!(
        provenance.installed_from.starts_with("bundled:"),
        "installed_from={}",
        provenance.installed_from
    );
    assert_eq!(provenance.files.len(), 5, "manifest + four rego");
    assert!(
        !provenance.files.iter().any(|f| f.path == "provenance.yaml"),
        "provenance must not hash itself"
    );

    // Every recorded hash matches the bytes actually written — the provenance
    // is verified against disk, not merely present.
    for entry in &provenance.files {
        let bytes = std::fs::read(pack_dir.join(&entry.path))
            .unwrap_or_else(|e| panic!("read {}: {e}", entry.path));
        assert_eq!(
            entry.sha256,
            sha256_hex(&bytes),
            "provenance hash mismatch for {}",
            entry.path
        );
    }
}

// ── Stage 2: admission pipeline ─────────────────────────────────────

#[test]
fn starter_policy_pack_passes_admission_pipeline() {
    let ws = TempDir::new().expect("workspace");
    let pack_dir = install_baseline(ws.path());

    // The full admission pipeline over the installed copy, mirroring
    // `install::assemble_report`: structural validation plus test enforcement.
    let manifest = load_manifest(&pack_dir.join("pack.yaml")).expect("manifest loads");
    let report = validate_pack(&manifest, &pack_dir);
    assert!(
        report.is_valid(),
        "structural validation must pass: {:?}",
        report.issues
    );

    let test_report = run_pack_tests(&manifest, &pack_dir).expect("pack tests run");
    assert!(
        enforce_tests(&test_report).is_empty(),
        "test enforcement must raise no issues"
    );

    // Each member ran its own Rego tests through the regorus facade — no load
    // errors, and every `test_*` rule passed.
    assert_eq!(
        test_report.members.len(),
        2,
        "change-scope + sensitive-paths"
    );
    for member in &test_report.members {
        assert!(
            member.load_error.is_none(),
            "{} load error: {:?}",
            member.policy_id,
            member.load_error
        );
        assert!(
            !member.outcomes.is_empty(),
            "{} ran no tests",
            member.policy_id
        );
        assert!(
            member.outcomes.iter().all(|o| o.passed),
            "{} has failing tests: {:?}",
            member.policy_id,
            member.outcomes
        );
    }
}

// ── Stage 3: live gate evaluation (advisory-first) ──────────────────

#[test]
fn starter_policy_pack_gate_evaluates_advisory_first() {
    let ws = TempDir::new().expect("workspace");
    install_baseline(ws.path());

    // The gate reads the changed set from `git status`, so make the workspace a
    // git repo and stage a sensitive change into the working-tree diff.
    git_init(ws.path());
    let workflow = ws.path().join(SENSITIVE_PATH);
    std::fs::create_dir_all(workflow.parent().unwrap()).unwrap();
    std::fs::write(&workflow, "name: deploy\non: push\njobs: {}\n").unwrap();

    let (passed, message) = crate::commands::gate::run_policy_check_for_proof(ws.path(), &[]);

    // Advisory-first (ADR-002): the gate evaluates the installed pack and still
    // passes (exit 0) with a sensitive path changed — the pack ships no
    // `violation`/`deny` rule, so it can never fail the gate.
    assert!(passed, "gate must pass advisory-first, got: {message}");
    assert!(
        message.contains("policies evaluated"),
        "the installed pack must actually be evaluated (not skipped): {message}"
    );
    assert!(
        !message.contains("Skipping"),
        "an installed pack must not be skipped: {message}"
    );

    // The pack's advisory SURFACES through the gate result as a warning-class
    // finding carrying remediation-first guidance (the review + exception path).
    // The gate recognises the documented `warning` rule family
    // (docs/guides/opa-policy-testing.md).
    assert!(
        message.contains("warning"),
        "a warning-class finding must surface at the gate: {message}"
    );
    assert!(
        message.contains("[warning]"),
        "the finding must render as warning-severity: {message}"
    );
    assert!(
        message.contains("reviewer") && message.contains("exception grant sensitive-paths"),
        "remediation-first guidance (review + exception path) must surface at the gate: {message}"
    );
}

// ── Stage 4: pre-write projection feeds the pack ────────────────────

#[test]
fn starter_policy_pack_prewrite_projection_feeds_pack() {
    let ws = TempDir::new().expect("workspace");
    let pack_dir = install_baseline(ws.path());

    // The pre-write surface builds a `PrewriteInput` and projects it onto the
    // `PolicyInput` the pack consumes (OPAE-006). The projection must carry the
    // sensitive changed path through to `diff.changed_files`.
    let input = PrewriteInput::from_parts(
        WorkflowPhase::Save,
        [ChangedPath::new(SENSITIVE_PATH, ChangeKind::Modified)],
        [],
        GraphFacts::default(),
        PrewriteBudget::default(),
    )
    .to_policy_input();
    assert_eq!(input.diff.changed_files, vec![SENSITIVE_PATH.to_string()]);

    // Evaluated against the installed pack on the pre-write input, the pack
    // fires with remediation-first guidance (the review/exception path).
    let value = eval_installed_pack(&pack_dir, &input, "data.anvil.policies");
    let warnings = collect_warning_messages(&value);
    assert!(
        !warnings.is_empty(),
        "the pre-write projection must feed the pack the sensitive change"
    );
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("reviewer") && m.contains("exception grant")),
        "remediation-first guidance (review + exception path) must be present: {warnings:?}"
    );
}

// ── Stage 5: advisory invariant — warnings never veto ───────────────

#[test]
fn starter_policy_pack_warnings_never_veto_even_under_interrupt() {
    let ws = TempDir::new().expect("workspace");
    let pack_dir = install_baseline(ws.path());

    // The pack's advisories, as warning-severity diagnostics a pre-write
    // surface would route.
    let value = eval_installed_pack(&pack_dir, &sensitive_change_input(), "data.anvil.policies");
    let diagnostics: Vec<Diagnostic> = collect_warning_messages(&value)
        .iter()
        .map(|m| warning_diagnostic(m))
        .collect();
    assert!(
        !diagnostics.is_empty(),
        "the pack must produce at least one advisory to route"
    );

    // Routed through the real production decision rule
    // (`mcp::enforcement::decision_for`): a warning-family finding caps at
    // `Warn` under every non-`off` posture and NEVER vetoes — not even under
    // the strictest `interrupt`. The pack is advisory/warning-family by design.
    for mode in [
        EnforcementMode::Warn,
        EnforcementMode::Fence,
        EnforcementMode::Interrupt,
    ] {
        let decision = crate::mcp::enforcement::decision_for(&diagnostics, mode);
        assert!(
            !decision.is_veto(),
            "warning-family finding must never veto (posture {mode:?} -> {decision:?})"
        );
        assert_eq!(
            decision,
            ControlDecision::Warn,
            "non-error findings warn under any non-off posture ({mode:?})"
        );
    }
    // `off` suppresses even the warning decision (finding still surfaced).
    assert_eq!(
        crate::mcp::enforcement::decision_for(&diagnostics, EnforcementMode::Off),
        ControlDecision::Allow
    );
}

// ── Stage 6: report-only eval-harness exercisability ────────────────

/// A [`PolicyEvalRunner`] that returns a fixed, already-produced v1 document —
/// used to drive the real [`PolicyEvalAdapter`] without spawning a subprocess
/// (the CLI binary sits behind an auth gate that `anvil policy eval` inherits;
/// the harness contract this binds to is the JSON shape, not the transport).
struct FixtureRunner(String);
impl PolicyEvalRunner for FixtureRunner {
    fn eval_json(&self, _suite: &EvalSuite) -> Result<String, EvalHarnessError> {
        Ok(self.0.clone())
    }
}

#[test]
fn starter_policy_pack_evaluates_under_frozen_eval_v1_contract() {
    let ws = TempDir::new().expect("workspace");
    let pack_dir = install_baseline(ws.path());

    // Evaluate one member policy through the SAME engine pipeline
    // `anvil policy eval` uses (Engine::new -> register_all -> add_policy ->
    // eval), pointed at the pack's `warning` rule with a sensitive-path input.
    let policy_path = pack_dir.join("policies/sensitive_paths.rego");
    let policy_display = policy_path.display().to_string();
    let query = "data.anvil.policies.sensitive_paths.warning";
    let mut engine = Engine::new(EngineConfig::default()).expect("engine");
    anvil_policy_engine::builtins::register_all(&mut engine).expect("builtins");
    engine
        .add_policy(
            policy_display.clone(),
            std::fs::read_to_string(&policy_path).expect("read policy"),
        )
        .expect("compile member policy");
    let result = engine
        .eval(&sensitive_change_input(), query)
        .expect("evaluate member policy");

    // The member policy fires: the raw value carries the remediation advisory.
    let raw = result.value.clone().unwrap_or(serde_json::Value::Null);
    let advisories: Vec<&str> = raw
        .as_array()
        .map(|a| a.iter().filter_map(serde_json::Value::as_str).collect())
        .unwrap_or_default();
    assert!(
        advisories.iter().any(|m| m.contains("reviewer")),
        "the member policy must fire under a sensitive-path input: {raw}"
    );

    // Assemble the frozen `anvil policy eval --json` v1 document. The pack's
    // `warning` rule yields a string set (not a `Finding` object array), so —
    // exactly as `commands::policy::eval::run` does for a non-findings query —
    // `findings` is empty and `exit_code` is 0. The gate-critical envelope
    // (schema_version/policy/query/findings/exit_code) is pinned by
    // `eval::eval_output_schema_stability_snapshot` and `policy-eval-output-v1`.
    let v1_document = serde_json::json!({
        "schema_version": "1.0.0",
        "policy": policy_display,
        "query": query,
        "findings": [],
        "exit_code": 0,
    })
    .to_string();

    // Normalise through the REAL frozen harness the eval-regression command
    // binds to — first the pure normaliser, then the full adapter port.
    let summary = normalise("starter-pack-sensitive-paths", &v1_document)
        .expect("v1 document normalises through the frozen harness");
    assert_eq!(summary.schema_version, "1.0.0");
    assert_eq!(summary.policy, policy_display);
    assert_eq!(summary.query, query);
    assert!(
        summary.passed(),
        "an advisory-only run is a passing verdict"
    );

    let adapter = PolicyEvalAdapter::new(FixtureRunner(v1_document));
    let suite = EvalSuite {
        name: "starter-pack-sensitive-paths".to_string(),
        policy: policy_path,
        input: None,
        query: query.to_string(),
    };
    let port_summary = adapter
        .run_suite(&suite)
        .expect("the harness port exercises the pack under the frozen contract");
    assert_eq!(port_summary.schema_version, "1.0.0");
    assert!(port_summary.passed());

    // NOTE: this proves the eval-regression harness CAN exercise a starter-pack
    // policy under the frozen v1 contract. Wiring an actual report-only CI step
    // is out of scope here — that is POLRESET-008 / EVALCI-006.
}

// ── Stage 7: documented-vocabulary lockstep guard ──────────────────

#[test]
fn starter_policy_pack_extractor_recognises_documented_rule_families() {
    // The pack emits its advisories under the documented `warning` rule set
    // (docs/guides/opa-policy-testing.md: "Both `violation` and `warning` rule
    // sets are recognised by the gate"). BOTH Rego rule-set extractors — the
    // gate (`commands::gate::extract_policy_findings`) and the pre-write path
    // (`mcp::policy_prewrite::extract_policy_findings`) — key off these same
    // crate single-source consts, so the pack's rule vocabulary and each
    // surface's recognised vocabulary cannot silently drift (the warn/warning
    // gap this proof originally surfaced). The pre-write surface's own
    // behavioural regression lives in
    // `policy_prewrite_routing_warning_family_surfaces_warning_class_diagnostic`.
    use crate::policy_vocab::{VIOLATION_FAMILY_KEYS, WARNING_FAMILY_KEYS};

    // The documented canonical rule names are in the single source of truth.
    assert!(
        WARNING_FAMILY_KEYS.contains(&"warning"),
        "the documented `warning` rule set must be recognised"
    );
    assert!(
        VIOLATION_FAMILY_KEYS.contains(&"violation"),
        "the documented `violation` rule set must be recognised"
    );

    // Lockstep: the bundled pack's actual rule-set names must all be members of
    // the recognised families. If the pack adds a rule set the extractor does
    // not key on, this trips — the two surfaces are kept in lockstep by the
    // shared consts, and this asserts the pack conforms to them.
    let ws = TempDir::new().expect("workspace");
    let pack_dir = install_baseline(ws.path());
    let value = eval_installed_pack(&pack_dir, &sensitive_change_input(), "data.anvil.policies");
    let recognised: std::collections::HashSet<&str> = WARNING_FAMILY_KEYS
        .iter()
        .chain(VIOLATION_FAMILY_KEYS.iter())
        .copied()
        .collect();
    if let Some(map) = value.as_object() {
        for (pkg, output) in map {
            if let Some(obj) = output.as_object() {
                for rule in obj.keys() {
                    // Rule sets that emit findings are arrays; helper rules
                    // (scalars/objects like `soft_limit`) are not findings.
                    let is_finding_set = obj.get(rule).is_some_and(serde_json::Value::is_array);
                    if is_finding_set {
                        assert!(
                            recognised.contains(rule.as_str()),
                            "pack `{pkg}` emits an unrecognised finding rule set `{rule}` \
                             (not in WARNING_FAMILY_KEYS/VIOLATION_FAMILY_KEYS)"
                        );
                    }
                }
            }
        }
    }
}

// ── git helper ──────────────────────────────────────────────────────

/// Initialise a bare-minimum git repo in `dir` so the gate's `git status`
/// changed-file discovery has something to read. No commit or identity is
/// needed: untracked files show under `git status --porcelain -u`.
fn git_init(dir: &Path) {
    let ok = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir)
        .status()
        .is_ok_and(|s| s.success());
    assert!(
        ok,
        "git init failed (git must be available for the gate proof)"
    );
}
