//! POLRESET-006 / OPAE-007: pre-write policy evaluation with posture routing.
//!
//! This module is the MCP-side, off-daemon consumer of the policy engine that
//! runs **after** the existing intercept-rules scan on the `anvil_validate_write`
//! path (ADR-098 AD-4: policy runs on existing off-daemon surfaces only — MCP
//! pre-write, `anvil gate`, CI — never on the resident daemon). It:
//!
//! 1. Reads the [`ANVIL_POLICY_ENFORCEMENT`](POLICY_ENFORCEMENT_ENV) **kill
//!    switch** first (re-read per call, trimmed). `off` / `0` skips policy
//!    evaluation entirely — the AD-5 out-of-band recovery path from a broken
//!    pack that bypasses `.anvil.yaml`.
//! 2. Discovers installed packs under `<workspace>/.anvil/policies/`
//!    ([`discover_and_load`]) and evaluates each through the ADR-040 regorus
//!    facade, using the OPAE-006 [`PrewriteInput`] projection and its tight
//!    fail-open [`PrewriteBudget`].
//! 3. Maps each finding onto the engine-free [`PolicyOutcome`] vocabulary
//!    (`violation`/`deny` families → violation-class, `warn` families →
//!    warning-class) and routes it onto a [`ControlDecision`] via the OPAE-007
//!    [`route_policy_outcome`] contract with the resolved posture.
//!
//! ## Fail-open, never a veto on machinery failure (ADR-098 AD-5)
//!
//! Every failure mode — a broken/unparseable pack, a member that will not
//! compile, an evaluation error, a **budget timeout**, or even a panic deep in
//! the engine — degrades that pack to a single **warning-class** outcome
//! carrying the error text. It never becomes a veto and never crashes the tool
//! call. Only a genuine `violation`-family finding under a `fence` / `interrupt`
//! posture can veto a write, and the default posture stays warnings-first
//! (ADR-002).
//!
//! ## Additive, never replacing the scan
//!
//! The caller merges this module's routed decision with the intercept-rules
//! scan decision via [`strictest_decision`] (strictest wins) and appends these
//! diagnostics to the scan diagnostics. Policy evaluation adds findings; it
//! never suppresses a scan finding.

use std::path::Path;

use anvil_intercept_rules::{PolicyOutcome, PolicySeverityClass, route_policy_outcome};
use anvil_kernel_types::diagnostics::ControlDecision;
use anvil_kernel_types::{
    Category, Diagnostic, DiagnosticSource, EnforcementMode, Location, Mode, Severity,
};
use anvil_policy_engine::context::ChangedPath;
use anvil_policy_engine::context::WorkflowPhase;
use anvil_policy_engine::context::assertion::ChangeKind;
use anvil_policy_engine::guidance::{PolicyGuidance, PolicySource};
use anvil_policy_engine::pack::{LoadedPack, discover_and_load};
use anvil_policy_engine::result::{Finding, Severity as FindingSeverity};
use anvil_policy_engine::{Engine, EngineConfig, GraphFacts, PrewriteBudget, PrewriteInput};

use crate::mcp::validation::{PRE_WRITE_MODE, sanitise_id_part};

/// The out-of-band policy kill switch (ADR-098 AD-5). Re-read per call and
/// whitespace-trimmed; `off` / `0` disable policy evaluation entirely,
/// bypassing `.anvil.yaml`. Unset (or any other value, e.g. `1`) leaves policy
/// evaluation enabled. This mirrors the `ANVIL_GCTX_EGRESS` kill-switch shape.
pub(crate) const POLICY_ENFORCEMENT_ENV: &str = "ANVIL_POLICY_ENFORCEMENT";

/// The Rego query the pre-write path evaluates — the same
/// `data.anvil.policies` root the `anvil gate` policy check uses, so packs
/// evaluate identically on both off-daemon surfaces.
const POLICY_QUERY: &str = "data.anvil.policies";

/// The outcome of a pre-write policy evaluation: the routed enforcement
/// decision plus the diagnostics to surface. `decision` is
/// [`ControlDecision::Allow`] and `diagnostics` is empty when policy is
/// disabled, no packs are installed, or nothing fired.
#[derive(Debug)]
pub(crate) struct PolicyPrewriteOutcome {
    /// PolicyGuidance-derived diagnostics for every finding (and every
    /// fail-open degradation), to be appended to the scan diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// The strictest routed [`ControlDecision`] across every outcome, or
    /// [`ControlDecision::Allow`] when nothing fired.
    pub decision: ControlDecision,
}

impl PolicyPrewriteOutcome {
    /// An inert outcome: no diagnostics, `allow`. Used when policy is disabled,
    /// discovery fails open, or no packs are installed.
    fn inert() -> Self {
        Self {
            diagnostics: Vec::new(),
            decision: ControlDecision::Allow,
        }
    }
}

/// One routed policy finding: its engine-free outcome and the diagnostic that
/// renders it.
struct RoutedRecord {
    outcome: PolicyOutcome,
    diagnostic: Diagnostic,
}

/// Whether pre-write policy evaluation is enabled, reading the
/// [`POLICY_ENFORCEMENT_ENV`] kill switch (AD-5). Unset ⇒ enabled; `off` / `0`
/// (case-insensitive, trimmed) ⇒ disabled; any other value ⇒ enabled.
fn policy_enforcement_enabled() -> bool {
    match std::env::var(POLICY_ENFORCEMENT_ENV) {
        Ok(raw) => !matches!(raw.trim().to_ascii_lowercase().as_str(), "off" | "0"),
        Err(_) => true,
    }
}

/// Evaluate installed policy packs against a single proposed write and route
/// the outcome onto the enforcement vocabulary.
///
/// `changed_path` is the workspace-relative path of the write; `change_kind` is
/// derived from the request's operation; `posture` is the resolved enforcement
/// mode. See the [module docs](self) for the kill switch, fail-open, and
/// routing contracts.
pub(crate) fn evaluate(
    workspace_root: &Path,
    changed_path: &str,
    change_kind: ChangeKind,
    posture: EnforcementMode,
) -> PolicyPrewriteOutcome {
    // Kill switch first (AD-5): a single debug-level log, never per-call spam.
    if !policy_enforcement_enabled() {
        tracing::debug!(
            target: "anvil::policy",
            "{POLICY_ENFORCEMENT_ENV} disables pre-write policy evaluation; skipping",
        );
        return PolicyPrewriteOutcome::inert();
    }

    // Discovery failing open: a containment breach or unreadable policies dir
    // must not block the write (AD-5) — degrade to no policy diagnostics.
    let loaded = match discover_and_load(workspace_root) {
        Ok(loaded) => loaded,
        Err(err) => {
            tracing::debug!(
                target: "anvil::policy",
                error = %err,
                "policy pack discovery failed; failing open with no policy diagnostics",
            );
            return PolicyPrewriteOutcome::inert();
        }
    };
    if loaded.is_empty() {
        return PolicyPrewriteOutcome::inert();
    }

    // One deterministic pre-write input (OPAE-006), projected to the Rego pack
    // shape. The budget is the adapter's tight, documented fail-open default.
    let prewrite = PrewriteInput::from_parts(
        WorkflowPhase::Save,
        [ChangedPath::new(changed_path, change_kind)],
        [],
        GraphFacts::default(),
        PrewriteBudget::default(),
    );
    let policy_input = prewrite.to_policy_input();
    let engine_config = prewrite.engine_config();

    let mut records: Vec<RoutedRecord> = Vec::new();
    for pack in &loaded {
        // Defence in depth (AD-5): the engine facade already guards regorus
        // panics, but wrap the whole per-pack evaluation so nothing — a panic
        // in a builtin, a poisoned lock — can crash the tool call. A caught
        // panic degrades the pack to a warning, exactly like any other failure.
        let pack_id = pack.pack.id.clone();
        let evaluated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            evaluate_pack(pack, &policy_input, &engine_config, changed_path)
        }));
        match evaluated {
            Ok(pack_records) => records.extend(pack_records),
            Err(_) => records.push(degraded_record(
                &pack_id,
                changed_path,
                "policy evaluation panicked",
            )),
        }
    }

    let mut decision = ControlDecision::Allow;
    let mut diagnostics = Vec::with_capacity(records.len());
    for record in records {
        decision = strictest_decision(decision, route_policy_outcome(&record.outcome, posture));
        diagnostics.push(record.diagnostic);
    }

    PolicyPrewriteOutcome {
        diagnostics,
        decision,
    }
}

/// Evaluate a single pack, returning one [`RoutedRecord`] per finding — or a
/// single fail-open warning-class record if the pack cannot be loaded,
/// compiled, or evaluated (AD-5).
fn evaluate_pack(
    pack: &LoadedPack,
    policy_input: &anvil_policy_engine::PolicyInput,
    engine_config: &EngineConfig,
    changed_path: &str,
) -> Vec<RoutedRecord> {
    let pack_id = pack.pack.id.as_str();

    // A broken / unparseable pack fails open to a warning: installation
    // validated it, but drift happens, and a broken pack must never block a
    // write (AD-5).
    let manifest = match &pack.manifest {
        Ok(manifest) => manifest,
        Err(err) => {
            return vec![degraded_record(
                pack_id,
                changed_path,
                &format!("pack manifest failed to load: {err}"),
            )];
        }
    };

    let mut engine = match Engine::new(engine_config.clone()) {
        Ok(engine) => engine,
        Err(err) => {
            return vec![degraded_record(
                pack_id,
                changed_path,
                &format!("policy engine unavailable: {err}"),
            )];
        }
    };
    if let Err(err) = anvil_policy_engine::builtins::register_all(&mut engine) {
        return vec![degraded_record(
            pack_id,
            changed_path,
            &format!("policy engine setup failed: {err}"),
        )];
    }

    // Load the manifest's member policies. Member paths are already lexically
    // contained within the pack directory (validated at manifest load); join
    // and read. Any read / compile failure degrades the whole pack to a
    // warning rather than blocking (AD-5).
    for entry in &manifest.policies {
        let member_path = pack.pack.dir.join(&entry.path);
        let source = match std::fs::read_to_string(&member_path) {
            Ok(source) => source,
            Err(err) => {
                return vec![degraded_record(
                    pack_id,
                    changed_path,
                    &format!(
                        "policy `{}` could not be read ({}): {err}",
                        entry.metadata.id,
                        member_path.display(),
                    ),
                )];
            }
        };
        let name = entry.path.to_string_lossy().into_owned();
        if let Err(err) = engine.add_policy(name, source) {
            return vec![degraded_record(
                pack_id,
                changed_path,
                &format!("policy `{}` failed to compile: {err}", entry.metadata.id),
            )];
        }
    }

    // Evaluation. A regorus error / timeout is caught by the facade and
    // surfaced as an `Err` here (the facade guards panics too); it degrades to
    // a warning, never a veto (AD-5 fail-open budget).
    let value = match engine.eval(policy_input, POLICY_QUERY) {
        Ok(result) => result.value,
        Err(err) => {
            return vec![degraded_record(
                pack_id,
                changed_path,
                &format!("policy evaluation failed: {err}"),
            )];
        }
    };

    value
        .as_ref()
        .map(extract_policy_findings)
        .unwrap_or_default()
        .into_iter()
        .map(|finding| record_from_finding(pack_id, changed_path, &finding))
        .collect()
}

/// A raw finding extracted from a `data.anvil.policies` result before routing.
struct RawFinding {
    /// The Rego sub-package that produced it (the key under `data.anvil.policies`).
    policy_id: String,
    /// Which rule family it came from.
    class: PolicySeverityClass,
    /// The human-readable message.
    message: String,
}

/// Extract findings from a `data.anvil.policies`-rooted value, mirroring the
/// `anvil gate` policy check's family mapping: `violation`/`violations`/`deny`/
/// `denies` are violation-class and `warn`/`warnings` are warning-class. An
/// object finding may override its class via a `severity` field (`error` ⇒
/// violation, `warn`/`warning`/`info` ⇒ warning); an unrecognised override
/// keeps the family default (fail-closed on a typo). Helper rules and other
/// keys are ignored.
fn extract_policy_findings(value: &serde_json::Value) -> Vec<RawFinding> {
    const KEYS: [(&str, PolicySeverityClass); 6] = [
        ("violation", PolicySeverityClass::Violation),
        ("violations", PolicySeverityClass::Violation),
        ("deny", PolicySeverityClass::Violation),
        ("denies", PolicySeverityClass::Violation),
        ("warn", PolicySeverityClass::Warning),
        ("warnings", PolicySeverityClass::Warning),
    ];

    let mut out = Vec::new();
    let Some(map) = value.as_object() else {
        return out;
    };
    for (policy_id, output) in map {
        let Some(obj) = output.as_object() else {
            continue;
        };
        for (key, default_class) in KEYS {
            let Some(arr) = obj.get(key).and_then(serde_json::Value::as_array) else {
                continue;
            };
            for item in arr {
                out.push(raw_finding_from_item(item, policy_id, default_class));
            }
        }
    }
    out
}

/// Build one [`RawFinding`] from a rule-set item: a bare string message, or an
/// object carrying `message`/`msg` and an optional `severity` class override.
fn raw_finding_from_item(
    item: &serde_json::Value,
    policy_id: &str,
    default_class: PolicySeverityClass,
) -> RawFinding {
    match item {
        serde_json::Value::String(message) => RawFinding {
            policy_id: policy_id.to_string(),
            class: default_class,
            message: message.clone(),
        },
        serde_json::Value::Object(obj) => {
            let message = obj
                .get("message")
                .or_else(|| obj.get("msg"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("policy violation")
                .to_string();
            let class = obj
                .get("severity")
                .and_then(serde_json::Value::as_str)
                .map_or(default_class, |sev| resolve_class(sev, default_class));
            RawFinding {
                policy_id: policy_id.to_string(),
                class,
                message,
            }
        }
        other => RawFinding {
            policy_id: policy_id.to_string(),
            class: default_class,
            message: other.to_string(),
        },
    }
}

/// Resolve a policy-supplied severity override onto a class, fail-closed to the
/// family default on an unrecognised value (so a typo cannot silently demote a
/// violation into the non-blocking bucket).
fn resolve_class(severity: &str, default_class: PolicySeverityClass) -> PolicySeverityClass {
    match severity.to_ascii_lowercase().as_str() {
        "error" | "err" => PolicySeverityClass::Violation,
        "warning" | "warn" | "info" => PolicySeverityClass::Warning,
        _ => default_class,
    }
}

/// Build a routed record from a real finding: a [`PolicyGuidance`]-derived
/// diagnostic plus the engine-free [`PolicyOutcome`] for routing.
fn record_from_finding(pack_id: &str, changed_path: &str, finding: &RawFinding) -> RoutedRecord {
    let finding_severity = match finding.class {
        PolicySeverityClass::Violation => FindingSeverity::Error,
        PolicySeverityClass::Warning => FindingSeverity::Warning,
    };
    // The pre-write path has no dependency edge, baseline, or fingerprint; the
    // guidance carries the message, rule id, pack source, and exception hint.
    let engine_finding = Finding {
        severity: finding_severity,
        message: finding.message.clone(),
        from: None,
        to: None,
        fingerprint: None,
        is_new_edge: false,
        baselined: false,
    };
    let guidance = PolicyGuidance::from_pack_finding(
        &engine_finding,
        finding.policy_id.clone(),
        pack_id.to_string(),
        "Review the flagged change against the policy, then adjust it or request an exception.",
    );
    let diagnostic = guidance_to_diagnostic(&guidance, finding.class, changed_path);
    RoutedRecord {
        outcome: PolicyOutcome {
            rule_id: finding.policy_id.clone(),
            class: finding.class,
        },
        diagnostic,
    }
}

/// Render a [`PolicyGuidance`] into a canonical [`Diagnostic`].
fn guidance_to_diagnostic(
    guidance: &PolicyGuidance,
    class: PolicySeverityClass,
    changed_path: &str,
) -> Diagnostic {
    let severity = match class {
        PolicySeverityClass::Violation => Severity::Error,
        PolicySeverityClass::Warning => Severity::Warning,
    };
    // Anchor at the offending path when the guidance localised one, else the
    // proposed write's path.
    let file = guidance
        .context
        .first()
        .map_or_else(|| changed_path.to_string(), |ctx| ctx.path.clone());
    let source_module = match &guidance.source {
        PolicySource::Pack(pack) => format!("anvil-policy-engine::pack::{pack}"),
        PolicySource::Assertion(id) => format!("anvil-policy-engine::assertion::{id}"),
        PolicySource::Scanner(name) => format!("anvil-policy-engine::scanner::{name}"),
    };
    Diagnostic::new(
        format!(
            "diag_policy_prewrite_{}_{}",
            sanitise_id_part(changed_path),
            sanitise_id_part(&guidance.rule_id),
        ),
        severity,
        guidance.message.clone(),
        Location {
            file,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Policy,
        DiagnosticSource {
            rule_id: guidance.rule_id.clone(),
            source_module,
        },
        Mode::Unknown(PRE_WRITE_MODE.to_string()),
    )
    .with_remediation_hint(format!(
        "{} {}",
        guidance.remediation, guidance.exception_guidance
    ))
}

/// Build the fail-open degradation record for a pack that could not be
/// evaluated (AD-5). Always warning-class, so it warns but never vetoes under
/// any posture — including a budget timeout, an engine error, or a panic.
fn degraded_record(pack_id: &str, changed_path: &str, message: &str) -> RoutedRecord {
    let rule_id = format!("policy-degraded:{pack_id}");
    let diagnostic = Diagnostic::new(
        format!(
            "diag_policy_prewrite_degraded_{}",
            sanitise_id_part(pack_id)
        ),
        // Warning, never error: a machinery failure must not escalate to a veto
        // under a strict posture.
        Severity::Warning,
        format!(
            "Policy pack `{pack_id}` could not be evaluated; failing open (the write is not \
             blocked): {message}"
        ),
        Location {
            file: changed_path.to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Policy,
        DiagnosticSource {
            rule_id: rule_id.clone(),
            source_module: "anvil-cli::mcp::policy_prewrite".to_string(),
        },
        Mode::Unknown(PRE_WRITE_MODE.to_string()),
    )
    .with_remediation_hint(
        "This is a policy-engine degradation, not a policy finding; the write is not blocked. \
         Check the pack for drift or corruption, or set ANVIL_POLICY_ENFORCEMENT=off to bypass \
         policy evaluation entirely while you recover.",
    );
    RoutedRecord {
        // Warning-class: routes to `warn` under every enforcing posture and
        // never vetoes (OPAE-007).
        outcome: PolicyOutcome::warning(rule_id),
        diagnostic,
    }
}

/// Strictest-wins merge of two [`ControlDecision`]s (the caller merges the
/// scan decision with the routed policy decision). Ordering mirrors the
/// enforcement escalation ladder: `allow < warn (= unknown) < block < fence <
/// interrupt`. `Unknown` degrades to the `warn` rank (never a veto), matching
/// the kernel-types forward-compat default (ADR-098 AD-3).
pub(crate) fn strictest_decision(a: ControlDecision, b: ControlDecision) -> ControlDecision {
    if decision_rank(a) >= decision_rank(b) {
        a
    } else {
        b
    }
}

/// Escalation rank for [`strictest_decision`].
fn decision_rank(decision: ControlDecision) -> u8 {
    match decision {
        ControlDecision::Allow => 0,
        // `Unknown` is treated as `warn` (never a veto) per ADR-098 AD-3.
        ControlDecision::Warn | ControlDecision::Unknown => 1,
        ControlDecision::Block => 2,
        ControlDecision::Fence => 3,
        ControlDecision::Interrupt => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Write an installed-style pack: `<ws>/.anvil/policies/<id>/pack.yaml`
    /// plus one member `.rego` under `policies/`. Returns the workspace dir.
    fn install_pack(pack_id: &str, member_rego: &str) -> TempDir {
        let ws = TempDir::new().expect("workspace");
        let pack_dir = ws.path().join(".anvil/policies").join(pack_id);
        std::fs::create_dir_all(pack_dir.join("policies")).expect("pack dirs");
        std::fs::write(pack_dir.join("policies/policy.rego"), member_rego).expect("member rego");
        let manifest = format!(
            "id: {pack_id}\n\
             name: Test pack {pack_id}\n\
             version: 1.0.0\n\
             description: Pre-write routing test pack.\n\
             owner: platform-security\n\
             policies:\n\
             \x20 - path: policies/policy.rego\n\
             \x20   metadata:\n\
             \x20     id: {pack_id}-policy\n\
             \x20     title: Test policy\n\
             \x20     severity: high\n\
             \x20     owner: platform-security\n\
             \x20     rationale: Exercises the pre-write routing path.\n\
             \x20     scope: '**'\n\
             \x20     tags: [test]\n"
        );
        std::fs::write(pack_dir.join("pack.yaml"), manifest).expect("manifest");
        ws
    }

    const VIOLATION_REGO: &str = r#"package anvil.policies.always_deny
import rego.v1

violation contains msg if {
    some f in input.diff.changed_files
    msg := sprintf("change to %s is denied by the test policy", [f])
}
"#;

    const WARN_REGO: &str = r#"package anvil.policies.always_warn
import rego.v1

warn contains msg if {
    some f in input.diff.changed_files
    msg := sprintf("change to %s is discouraged by the test policy", [f])
}
"#;

    #[test]
    fn policy_prewrite_routing_violation_pack_interrupt_posture_vetoes() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(outcome.decision, ControlDecision::Interrupt);
            assert!(
                outcome.decision.is_veto(),
                "violation + interrupt must veto"
            );
            assert!(
                outcome
                    .diagnostics
                    .iter()
                    .any(|d| d.severity == Severity::Error && d.category == Category::Policy),
                "a violation renders an error-severity policy diagnostic: {:?}",
                outcome.diagnostics,
            );
        });
    }

    #[test]
    fn policy_prewrite_routing_violation_pack_default_warn_posture_does_not_veto() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Warn,
            );
            // Warnings-first default (ADR-002): a violation only warns until the
            // operator opts into a stricter posture.
            assert_eq!(outcome.decision, ControlDecision::Warn);
            assert!(!outcome.decision.is_veto(), "warn posture must not veto");
            assert!(
                !outcome.diagnostics.is_empty(),
                "the finding is still surfaced under warn",
            );
        });
    }

    #[test]
    fn policy_prewrite_routing_warn_family_never_vetoes_even_under_interrupt() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = install_pack("warn-pack", WARN_REGO);
            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(outcome.decision, ControlDecision::Warn);
            assert!(!outcome.decision.is_veto());
        });
    }

    #[test]
    fn policy_prewrite_routing_kill_switch_off_yields_no_diagnostics() {
        // AD-5: the kill switch bypasses discovery entirely, even with a
        // violation pack installed and an interrupt posture.
        temp_env::with_var(POLICY_ENFORCEMENT_ENV, Some("off"), || {
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(outcome.decision, ControlDecision::Allow);
            assert!(
                outcome.diagnostics.is_empty(),
                "kill switch off must produce no policy diagnostics: {:?}",
                outcome.diagnostics,
            );
        });
        // `0` disables too.
        temp_env::with_var(POLICY_ENFORCEMENT_ENV, Some("0"), || {
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(outcome.decision, ControlDecision::Allow);
            assert!(outcome.diagnostics.is_empty());
        });
    }

    #[test]
    fn policy_prewrite_routing_broken_pack_warns_not_vetoes() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            // A pack whose manifest will not parse: installation validated it,
            // but drift happens. It must fail open to a warning, never a veto,
            // even under an interrupt posture.
            let ws = TempDir::new().expect("workspace");
            let pack_dir = ws.path().join(".anvil/policies/busted");
            std::fs::create_dir_all(&pack_dir).expect("pack dir");
            std::fs::write(pack_dir.join("pack.yaml"), "{ not valid pack yaml")
                .expect("broken manifest");

            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(
                outcome.decision,
                ControlDecision::Warn,
                "a broken pack must warn, not veto",
            );
            assert!(!outcome.decision.is_veto());
            assert_eq!(outcome.diagnostics.len(), 1);
            assert_eq!(outcome.diagnostics[0].severity, Severity::Warning);
            assert!(
                outcome.diagnostics[0].summary.contains("failing open"),
                "the degradation is labelled: {}",
                outcome.diagnostics[0].summary,
            );
        });
    }

    #[test]
    fn policy_prewrite_routing_uncompilable_member_warns_not_vetoes() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            // A valid manifest whose member `.rego` will not compile degrades to
            // a warning (fail-open), never a veto.
            let ws = install_pack(
                "bad-rego",
                "package anvil.policies.oops\nthis is not rego\n",
            );
            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(outcome.decision, ControlDecision::Warn);
            assert!(!outcome.decision.is_veto());
            assert_eq!(outcome.diagnostics.len(), 1);
            assert_eq!(outcome.diagnostics[0].severity, Severity::Warning);
        });
    }

    #[test]
    fn policy_prewrite_routing_no_packs_is_inert() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = TempDir::new().expect("workspace");
            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(outcome.decision, ControlDecision::Allow);
            assert!(outcome.diagnostics.is_empty());
        });
    }

    #[test]
    fn policy_prewrite_routing_budget_exhaustion_degrades_to_warning() {
        // AD-5 fail-open budget: an eval timeout is mapped through the same
        // degradation path as any other engine error. Simulate the timeout by
        // injecting the error mapping directly (no real slow eval), then route
        // it — it must warn, never veto, even under interrupt.
        let record = degraded_record(
            "slow-pack",
            "src/app.rs",
            "policy evaluation failed: regorus error: evaluation exceeded the time limit",
        );
        assert_eq!(record.outcome.class, PolicySeverityClass::Warning);
        assert_eq!(
            route_policy_outcome(&record.outcome, EnforcementMode::Interrupt),
            ControlDecision::Warn,
        );
        assert!(
            !route_policy_outcome(&record.outcome, EnforcementMode::Interrupt).is_veto(),
            "a budget-exhausted pack must never veto",
        );
        assert_eq!(record.diagnostic.severity, Severity::Warning);
    }

    #[test]
    fn policy_prewrite_routing_strictest_decision_merges_scan_and_policy() {
        // The caller merges the scan decision with the routed policy decision;
        // strictest wins, and `Unknown` never out-ranks a real veto.
        assert_eq!(
            strictest_decision(ControlDecision::Warn, ControlDecision::Interrupt),
            ControlDecision::Interrupt,
        );
        assert_eq!(
            strictest_decision(ControlDecision::Fence, ControlDecision::Warn),
            ControlDecision::Fence,
        );
        assert_eq!(
            strictest_decision(ControlDecision::Allow, ControlDecision::Warn),
            ControlDecision::Warn,
        );
        assert_eq!(
            strictest_decision(ControlDecision::Unknown, ControlDecision::Allow),
            ControlDecision::Unknown,
        );
        assert_eq!(
            strictest_decision(ControlDecision::Interrupt, ControlDecision::Unknown),
            ControlDecision::Interrupt,
        );
    }

    #[test]
    fn policy_prewrite_routing_kill_switch_reads_are_trimmed_and_case_insensitive() {
        for value in ["off", " OFF ", "Off", "0", " 0 "] {
            temp_env::with_var(POLICY_ENFORCEMENT_ENV, Some(value), || {
                assert!(
                    !policy_enforcement_enabled(),
                    "value {value:?} must disable"
                );
            });
        }
        for value in ["1", "on", "enabled", "", "  "] {
            temp_env::with_var(POLICY_ENFORCEMENT_ENV, Some(value), || {
                assert!(policy_enforcement_enabled(), "value {value:?} must enable");
            });
        }
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            assert!(policy_enforcement_enabled(), "unset must enable");
        });
    }

    // Keeps the `PathBuf` import used across helpers explicit and stable.
    #[test]
    fn policy_prewrite_routing_install_pack_layout_is_discoverable() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let manifest: PathBuf = ws.path().join(".anvil/policies/deny-pack/pack.yaml");
            assert!(manifest.is_file(), "fixture writes an installed-style pack");
        });
    }
}
