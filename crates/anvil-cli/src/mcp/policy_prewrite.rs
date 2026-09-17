//! POLRESET-006 / OPAE-007: MCP-side pre-write policy evaluation and posture routing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

use anvil_intercept_rules::{PolicyOutcome, PolicySeverityClass, route_policy_outcome};
use anvil_kernel_types::diagnostics::ControlDecision;
use anvil_kernel_types::{
    Category, Diagnostic, DiagnosticSource, EnforcementMode, Location, Mode, Severity,
};
use anvil_policy::exceptions::{ExceptionStore, Violation, is_suppressed};
use anvil_policy_engine::context::ChangedPath;
use anvil_policy_engine::context::WorkflowPhase;
use anvil_policy_engine::context::assertion::ChangeKind;
use anvil_policy_engine::guidance::{PolicyGuidance, PolicySource};
use anvil_policy_engine::pack::{
    LoadedPack, PackOverlay, discover_and_load, enabled_entries, load_overlay_fail_open,
    overlay_path, resolve_member_id,
};
use anvil_policy_engine::result::{Finding, Severity as FindingSeverity};
use anvil_policy_engine::{Engine, EngineConfig, GraphFacts, PrewriteBudget, PrewriteInput};

use crate::mcp::enforcement;
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

/// Per-member source cap on the pre-write path (1 MiB, matching the gate's
/// per-policy cap): an oversized member degrades fail-open rather than
/// stalling the pass between deadline checks with an unbounded read.
const PREWRITE_MAX_POLICY_BYTES: u64 = 1024 * 1024;

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
    /// Whether installed packs were actually evaluated. `false` for every
    /// inert early-out (kill switch, no changed paths, discovery failed open,
    /// no packs installed) so a caller such as the ADR-149 save-time hook can
    /// withhold the `policy` check family rather than claim a scope that never
    /// ran.
    pub evaluated: bool,
}

impl PolicyPrewriteOutcome {
    /// An inert outcome: no diagnostics, `allow`. Used when policy is disabled,
    /// discovery fails open, or no packs are installed.
    fn inert() -> Self {
        Self {
            diagnostics: Vec::new(),
            decision: ControlDecision::Allow,
            evaluated: false,
        }
    }
}

/// One routed policy finding: its engine-free outcome and the diagnostic that
/// renders it.
struct RoutedRecord {
    outcome: PolicyOutcome,
    diagnostic: Diagnostic,
    /// True when the record is attributed to a specific changed path (a real
    /// finding, evaluated against that path's own policy input). False for
    /// pack-level machinery records — degradation and truncation warnings —
    /// which describe the pass, not a file, and must therefore never be
    /// suppressed by a per-file exception grant.
    path_scoped: bool,
}

/// One changed path in the pass, paired with the deterministic pre-write input
/// that localises evaluation to it.
///
/// ADR-149 coalesces a save into several changed paths. Each path gets its own
/// [`PolicyInput`](anvil_policy_engine::PolicyInput) so every finding,
/// exception lookup, and diagnostic location names the file it actually came
/// from, instead of collapsing the batch onto its first entry. The compiled
/// engine is shared across the batch (OPAE-011), so the extra cost is one
/// bounded eval per path, not one compile.
struct PathInput {
    path: String,
    input: anvil_policy_engine::PolicyInput,
}

/// The outcome of evaluating a single pack under the pass deadline.
enum PackEval {
    /// The pack was evaluated against every changed path; its (possibly empty)
    /// findings.
    Findings(Vec<RoutedRecord>),
    /// The pass deadline was reached mid-pack (before a member compile, or
    /// between two changed paths' evals). Carries the records already produced
    /// for this pack so a partial batch is not silently dropped; the caller
    /// keeps them and collapses this and every remaining pack into one
    /// truncation warning.
    Truncated(Vec<RoutedRecord>),
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
    // Reuse the OPAE-006 adapter's tight default as the TOTAL pass budget — it
    // bounds discovery + compile + eval, not just a single eval.
    let pass_budget = PrewriteBudget::default().max_eval;
    evaluate_many_with_budget(
        workspace_root,
        &[(changed_path, change_kind)],
        posture,
        pass_budget,
    )
}

/// Evaluate installed packs against a change set (ADR-149 save-time hook).
pub(crate) fn evaluate_many(
    workspace_root: &Path,
    changes: &[(&str, ChangeKind)],
    posture: EnforcementMode,
) -> PolicyPrewriteOutcome {
    let pass_budget = PrewriteBudget::default().max_eval;
    evaluate_many_with_budget(workspace_root, changes, posture, pass_budget)
}

/// [`evaluate`] with an explicit total-pass budget, so tests can force deadline
/// exhaustion mid-pass without a real slow eval.
#[cfg(test)]
fn evaluate_with_budget(
    workspace_root: &Path,
    changed_path: &str,
    change_kind: ChangeKind,
    posture: EnforcementMode,
    pass_budget: Duration,
) -> PolicyPrewriteOutcome {
    evaluate_many_with_budget(
        workspace_root,
        &[(changed_path, change_kind)],
        posture,
        pass_budget,
    )
}

fn evaluate_many_with_budget(
    workspace_root: &Path,
    changes: &[(&str, ChangeKind)],
    posture: EnforcementMode,
    pass_budget: Duration,
) -> PolicyPrewriteOutcome {
    let Some((anchor_path, _)) = changes.first() else {
        return PolicyPrewriteOutcome::inert();
    };
    // The anchor localises pack-level machinery records only (degradation and
    // truncation warnings, which describe the pass rather than a file). Real
    // findings are attributed to their own changed path — see [`PathInput`].
    let anchor_path = *anchor_path;
    // Kill switch first (AD-5): a single debug-level log, never per-call spam.
    if !policy_enforcement_enabled() {
        tracing::debug!(
            target: "anvil::policy",
            "{POLICY_ENFORCEMENT_ENV} disables pre-write policy evaluation; skipping",
        );
        return PolicyPrewriteOutcome::inert();
    }

    // One wall-clock deadline over the WHOLE pass (AD-5). Operational timing
    // only — it never enters any PolicyInput content.
    let started = Instant::now();
    let deadline = started + pass_budget;

    // Discovery failing open: a containment breach or unreadable policies dir
    // must not block the write (AD-5) — degrade to no policy diagnostics. The
    // no-packs case (missing dir) is a single stat then this early-out.
    // OPAE-011: warm passes reuse loaded pack/manifest metadata when the
    // policies-dir mtime fingerprint is unchanged (eval-only; no rescan).
    let loaded = match cached_discover_and_load(workspace_root) {
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

    // One deterministic pre-write input (OPAE-006) per changed path, projected
    // to the Rego pack shape. Per-path inputs keep finding attribution, the
    // exception lookup, and the diagnostic location honest for a coalesced
    // save; the budget carries the pass budget through, and each pack's engine
    // is then bounded by min(remaining, that budget).
    let inputs: Vec<PathInput> = changes
        .iter()
        .map(|(path, kind)| {
            let prewrite = PrewriteInput::from_parts(
                WorkflowPhase::Save,
                [ChangedPath::new(*path, *kind)],
                [],
                GraphFacts::default(),
                PrewriteBudget::new(pass_budget),
            );
            PathInput {
                path: (*path).to_string(),
                input: prewrite.to_policy_input(),
            }
        })
        .collect();
    let base_config = PrewriteInput::from_parts(
        WorkflowPhase::Save,
        changes
            .iter()
            .map(|(path, kind)| ChangedPath::new(*path, *kind)),
        [],
        GraphFacts::default(),
        PrewriteBudget::new(pass_budget),
    )
    .engine_config();

    let total = loaded.len();
    let mut records: Vec<RoutedRecord> = Vec::new();
    let mut evaluated = 0usize;
    for pack in loaded.iter() {
        // Deadline gate before each pack's expensive compile/eval work. On
        // exhaustion, collapse this and every remaining pack into one
        // truncation warning (never a veto) and stop.
        if Instant::now() >= deadline {
            records.push(truncation_record(evaluated, total, started, anchor_path));
            break;
        }
        // Defence in depth (AD-5): the engine facade already guards regorus
        // panics, but wrap the whole per-pack evaluation so nothing — a panic
        // in a builtin, a poisoned lock — can crash the tool call. A caught
        // panic degrades the pack to a warning, exactly like any other failure.
        let pack_id = pack.pack.id.clone();
        let pack_eval = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            evaluate_pack(pack, &inputs, &base_config, deadline, anchor_path)
        }));
        match pack_eval {
            Ok(PackEval::Findings(pack_records)) => {
                records.extend(pack_records);
                evaluated += 1;
            }
            Ok(PackEval::Truncated(partial)) => {
                records.extend(partial);
                records.push(truncation_record(evaluated, total, started, anchor_path));
                break;
            }
            Err(_) => {
                records.push(degraded_record(
                    &pack_id,
                    anchor_path,
                    "policy evaluation panicked",
                ));
                evaluated += 1;
            }
        }
    }

    let records = suppress_excepted_records(workspace_root, records);

    let mut decision = ControlDecision::Allow;
    let mut diagnostics = Vec::with_capacity(records.len());
    for record in records {
        decision = strictest_decision(decision, route_policy_outcome(&record.outcome, posture));
        diagnostics.push(record.diagnostic);
    }

    PolicyPrewriteOutcome {
        diagnostics,
        decision,
        evaluated: true,
    }
}

/// Build an [`EngineConfig`] bounded by the time remaining until `deadline`, or
/// `None` when the deadline has already passed (the caller truncates). The
/// per-eval `eval_timeout` is `min(remaining, base eval budget)`.
fn bounded_engine_config(base: &EngineConfig, deadline: Instant) -> Option<EngineConfig> {
    let remaining = deadline.checked_duration_since(Instant::now())?;
    if remaining.is_zero() {
        return None;
    }
    let eval_timeout = Some(match base.eval_timeout {
        Some(budget) => remaining.min(budget),
        None => remaining,
    });
    Some(EngineConfig {
        eval_timeout,
        ..base.clone()
    })
}

/// Process-local compiled engines for the MCP pre-write pass (OPAE-011).
/// Keyed by pack directory; invalidated when the mtime fingerprint of the
/// manifest, overlay, or enabled members changes.
struct PrewriteEngineCache {
    entries: HashMap<PathBuf, CachedPackEngine>,
    compiles: AtomicU64,
}

struct CachedPackEngine {
    fingerprint: PackFingerprint,
    engine: Engine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PackFingerprint {
    stamps: Vec<(String, u128, u64)>,
}

fn prewrite_engine_cache() -> std::sync::MutexGuard<'static, PrewriteEngineCache> {
    static CACHE: OnceLock<Mutex<PrewriteEngineCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            Mutex::new(PrewriteEngineCache {
                entries: HashMap::new(),
                compiles: AtomicU64::new(0),
            })
        })
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
fn reset_prewrite_engine_cache() {
    let mut cache = prewrite_engine_cache();
    cache.entries.clear();
    cache.compiles.store(0, Ordering::Relaxed);
    reset_prewrite_discovery_cache();
}

#[cfg(test)]
fn prewrite_engine_compile_count() -> u64 {
    prewrite_engine_cache().compiles.load(Ordering::Relaxed)
}

/// Process-local loaded pack/manifest cache (OPAE-011 warm path). Keyed by the
/// workspace policies directory; invalidated when any pack.yaml, overlay, or
/// member mtime/len stamp changes.
struct PrewriteDiscoveryCache {
    entry: Option<CachedDiscovery>,
    discoveries: AtomicU64,
}

struct CachedDiscovery {
    policies_dir: PathBuf,
    fingerprint: PackFingerprint,
    packs: Arc<Vec<LoadedPack>>,
}

fn prewrite_discovery_cache() -> std::sync::MutexGuard<'static, PrewriteDiscoveryCache> {
    static CACHE: OnceLock<Mutex<PrewriteDiscoveryCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            Mutex::new(PrewriteDiscoveryCache {
                entry: None,
                discoveries: AtomicU64::new(0),
            })
        })
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
fn reset_prewrite_discovery_cache() {
    let mut cache = prewrite_discovery_cache();
    cache.entry = None;
    cache.discoveries.store(0, Ordering::Relaxed);
}

#[cfg(test)]
fn prewrite_discovery_count() -> u64 {
    prewrite_discovery_cache()
        .discoveries
        .load(Ordering::Relaxed)
}

/// Stamp every immediate pack.yaml, overlay, and member under the policies
/// directory. Missing dir => empty fingerprint (cold empty install).
fn discovery_fingerprint(policies_dir: &Path) -> PackFingerprint {
    let mut stamps = Vec::new();
    let Ok(entries) = std::fs::read_dir(policies_dir) else {
        return PackFingerprint { stamps };
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let manifest = path.join("pack.yaml");
            if let Some((mtime, len)) = file_stamp(&manifest) {
                stamps.push((manifest.display().to_string(), mtime, len));
            }
            let members = path.join("policies");
            if let Ok(members_it) = std::fs::read_dir(&members) {
                for member in members_it.flatten() {
                    let mp = member.path();
                    if mp.is_file()
                        && let Some((mtime, len)) = file_stamp(&mp)
                    {
                        stamps.push((mp.display().to_string(), mtime, len));
                    }
                }
            }
        } else if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with(".overlay.yaml")
                && let Some((mtime, len)) = file_stamp(&path)
            {
                stamps.push((path.display().to_string(), mtime, len));
            }
        }
    }
    stamps.sort_by(|a, b| a.0.cmp(&b.0));
    PackFingerprint { stamps }
}

fn cached_discover_and_load(
    workspace_root: &Path,
) -> Result<Arc<Vec<LoadedPack>>, anvil_policy_engine::pack::DiscoveryError> {
    let policies_dir = workspace_root.join(".anvil/policies");
    let fingerprint = discovery_fingerprint(&policies_dir);
    {
        let cache = prewrite_discovery_cache();
        if let Some(entry) = cache.entry.as_ref()
            && entry.policies_dir == policies_dir
            && entry.fingerprint == fingerprint
        {
            return Ok(Arc::clone(&entry.packs));
        }
    }
    let loaded = discover_and_load(workspace_root)?;
    let packs = Arc::new(loaded);
    let mut cache = prewrite_discovery_cache();
    cache.discoveries.fetch_add(1, Ordering::Relaxed);
    cache.entry = Some(CachedDiscovery {
        policies_dir,
        fingerprint,
        packs: Arc::clone(&packs),
    });
    Ok(packs)
}

fn file_stamp(path: &Path) -> Option<(u128, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some((mtime.as_nanos(), meta.len()))
}

fn stamp_or_zero(path: &Path) -> (u128, u64) {
    file_stamp(path).unwrap_or((0, 0))
}

fn pack_fingerprint(
    pack: &LoadedPack,
    manifest: &anvil_policy_engine::pack::PackManifest,
    overlay: &PackOverlay,
) -> Option<PackFingerprint> {
    let mut stamps = Vec::new();
    let (mtime, len) = file_stamp(&pack.pack.manifest_path)?;
    stamps.push((pack.pack.manifest_path.display().to_string(), mtime, len));
    if let Some(policies_dir) = pack.pack.dir.parent()
        && let Ok(path) = overlay_path(policies_dir, &pack.pack.id)
    {
        let (mtime, len) = stamp_or_zero(&path);
        stamps.push((path.display().to_string(), mtime, len));
    }
    for entry in enabled_entries(manifest, overlay) {
        let member_path = pack.pack.dir.join(&entry.path);
        let (mtime, len) = file_stamp(&member_path)?;
        stamps.push((member_path.display().to_string(), mtime, len));
    }
    stamps.sort_by(|a, b| a.0.cmp(&b.0));
    Some(PackFingerprint { stamps })
}

fn remember_compiled_engine(pack_dir: PathBuf, fingerprint: PackFingerprint, engine: Engine) {
    let mut cache = prewrite_engine_cache();
    cache.entries.insert(
        pack_dir,
        CachedPackEngine {
            fingerprint,
            engine,
        },
    );
}

/// Warm-cache hit: take the engine out under a short lock, eval unlocked, put
/// back. Returns `None` on miss/stale/poison so the caller compiles cold.
fn try_eval_cached_pack(
    pack: &LoadedPack,
    fingerprint: &PackFingerprint,
    eval_timeout: Option<Duration>,
    inputs: &[PathInput],
    manifest: &anvil_policy_engine::pack::PackManifest,
    anchor_path: &str,
    deadline: Instant,
) -> Option<PackEval> {
    let pack_id = pack.pack.id.as_str();
    let cached = {
        let mut cache = prewrite_engine_cache();
        match cache.entries.remove(&pack.pack.dir) {
            Some(entry) if entry.fingerprint == *fingerprint && !entry.engine.is_poisoned() => {
                Some(entry)
            }
            Some(entry) if entry.engine.is_poisoned() => None,
            Some(_stale) => None,
            None => None,
        }
    }?;
    let mut entry = cached;
    if Instant::now() >= deadline {
        remember_compiled_engine(pack.pack.dir.clone(), entry.fingerprint, entry.engine);
        return Some(PackEval::Truncated(Vec::new()));
    }
    let _ = entry.engine.set_eval_timeout(eval_timeout);
    let outcome = eval_compiled_engine(
        &mut entry.engine,
        inputs,
        manifest,
        pack_id,
        anchor_path,
        deadline,
    );
    if !entry.engine.is_poisoned() {
        remember_compiled_engine(pack.pack.dir.clone(), entry.fingerprint, entry.engine);
    }
    Some(outcome)
}

/// Cold-path engine construction + builtin registration for a pre-write pack.
fn new_prewrite_engine(
    config: EngineConfig,
    pack_id: &str,
    anchor_path: &str,
) -> Result<Engine, PackEval> {
    let mut engine = match Engine::new(config) {
        Ok(engine) => engine,
        Err(err) => {
            return Err(PackEval::Findings(vec![degraded_record(
                pack_id,
                anchor_path,
                &format!("policy engine unavailable: {err}"),
            )]));
        }
    };
    if let Err(err) = anvil_policy_engine::builtins::register_all(&mut engine) {
        return Err(PackEval::Findings(vec![degraded_record(
            pack_id,
            anchor_path,
            &format!("policy engine setup failed: {err}"),
        )]));
    }
    prewrite_engine_cache()
        .compiles
        .fetch_add(1, Ordering::Relaxed);
    Ok(engine)
}

/// Evaluate a single pack under the pass `deadline`. Returns
/// [`PackEval::Findings`] with one [`RoutedRecord`] per finding (or a single
/// fail-open warning-class record if the pack cannot be loaded, compiled, or
/// evaluated), or [`PackEval::Truncated`] if the deadline is reached before this
/// pack's compile or eval completes (AD-5).
fn evaluate_pack(
    pack: &LoadedPack,
    inputs: &[PathInput],
    base_config: &EngineConfig,
    deadline: Instant,
    anchor_path: &str,
) -> PackEval {
    let pack_id = pack.pack.id.as_str();

    // A broken / unparseable pack fails open to a warning: installation
    // validated it, but drift happens, and a broken pack must never block a
    // write (AD-5).
    let manifest = match &pack.manifest {
        Ok(manifest) => manifest,
        Err(err) => {
            return PackEval::Findings(vec![degraded_record(
                pack_id,
                anchor_path,
                &format!("pack manifest failed to load: {err}"),
            )]);
        }
    };

    // Bound the engine's eval by the time remaining in the pass; a passed
    // deadline truncates.
    let Some(config) = bounded_engine_config(base_config, deadline) else {
        return PackEval::Truncated(Vec::new());
    };

    let overlay = pack
        .pack
        .dir
        .parent()
        .map(|policies_dir| load_overlay_fail_open(policies_dir, pack_id))
        .unwrap_or_default();
    let fingerprint = pack_fingerprint(pack, manifest, &overlay);

    if let Some(fp) = fingerprint.as_ref()
        && let Some(outcome) = try_eval_cached_pack(
            pack,
            fp,
            config.eval_timeout,
            inputs,
            manifest,
            anchor_path,
            deadline,
        )
    {
        return outcome;
    }

    let mut engine = match new_prewrite_engine(config, pack_id, anchor_path) {
        Ok(engine) => engine,
        Err(early) => return early,
    };

    if let Err(early) = compile_enabled_members(
        pack,
        manifest,
        pack_id,
        &mut engine,
        &overlay,
        deadline,
        anchor_path,
    ) {
        return early;
    }

    // Deadline gate before the eval itself.
    if Instant::now() >= deadline {
        return PackEval::Truncated(Vec::new());
    }

    let outcome = eval_compiled_engine(
        &mut engine,
        inputs,
        manifest,
        pack_id,
        anchor_path,
        deadline,
    );
    if !engine.is_poisoned()
        && let Some(fingerprint) = fingerprint
    {
        remember_compiled_engine(pack.pack.dir.clone(), fingerprint, engine);
    }
    outcome
}

fn eval_compiled_engine(
    engine: &mut Engine,
    inputs: &[PathInput],
    manifest: &anvil_policy_engine::pack::PackManifest,
    pack_id: &str,
    anchor_path: &str,
    deadline: Instant,
) -> PackEval {
    // One eval per changed path against the already-compiled engine, so each
    // finding carries the path it fired on rather than the batch's first entry.
    let mut records = Vec::new();
    for input in inputs {
        // Deadline gate between paths: a long batch must not outrun the pass
        // budget. Records produced so far travel with the truncation.
        if Instant::now() >= deadline {
            return PackEval::Truncated(records);
        }
        // Evaluation. A regorus error / timeout (the engine's own eval_timeout
        // is min(remaining, budget)) is caught by the facade and surfaced as an
        // `Err` here (the facade guards panics too); it degrades to a warning,
        // never a veto (AD-5 fail-open budget). The degradation is pack-level,
        // so it anchors at the pass anchor and stops the pack's batch.
        let value = match engine.eval(&input.input, POLICY_QUERY) {
            Ok(result) => result.value,
            Err(err) => {
                records.push(degraded_record(
                    pack_id,
                    anchor_path,
                    &format!("policy evaluation failed: {err}"),
                ));
                return PackEval::Findings(records);
            }
        };
        match findings_from_eval(value.as_ref(), manifest, pack_id, &input.path) {
            PackEval::Findings(path_records) => records.extend(path_records),
            PackEval::Truncated(partial) => {
                records.extend(partial);
                return PackEval::Truncated(records);
            }
        }
    }
    PackEval::Findings(records)
}

/// Compile overlay-enabled members into `engine`. Disabled members are skipped
/// so the overlay is honoured on the pre-write path. Read/compile failures
/// degrade fail-open (AD-5).
fn compile_enabled_members(
    pack: &LoadedPack,
    manifest: &anvil_policy_engine::pack::PackManifest,
    pack_id: &str,
    engine: &mut Engine,
    overlay: &PackOverlay,
    deadline: Instant,
    changed_path: &str,
) -> Result<(), PackEval> {
    for entry in enabled_entries(manifest, overlay) {
        if Instant::now() >= deadline {
            return Err(PackEval::Truncated(Vec::new()));
        }
        let member_path = pack.pack.dir.join(&entry.path);
        if let Ok(meta) = std::fs::metadata(&member_path)
            && meta.len() > PREWRITE_MAX_POLICY_BYTES
        {
            return Err(PackEval::Findings(vec![degraded_record(
                pack_id,
                changed_path,
                &format!(
                    "policy `{}` exceeds the {} KiB pre-write size cap ({} bytes); \
                         failing open — evaluate it via `anvil gate` instead",
                    entry.metadata.id,
                    PREWRITE_MAX_POLICY_BYTES / 1024,
                    meta.len()
                ),
            )]));
        }
        let source = match std::fs::read_to_string(&member_path) {
            Ok(source) => source,
            Err(err) => {
                return Err(PackEval::Findings(vec![degraded_record(
                    pack_id,
                    changed_path,
                    &format!(
                        "policy `{}` could not be read ({}): {err}",
                        entry.metadata.id,
                        member_path.display(),
                    ),
                )]));
            }
        };
        let name = entry.path.to_string_lossy().into_owned();
        if let Err(err) = engine.add_policy(name, source) {
            return Err(PackEval::Findings(vec![degraded_record(
                pack_id,
                changed_path,
                &format!("policy `{}` failed to compile: {err}", entry.metadata.id),
            )]));
        }
    }
    Ok(())
}

fn findings_from_eval(
    value: Option<&serde_json::Value>,
    manifest: &anvil_policy_engine::pack::PackManifest,
    pack_id: &str,
    changed_path: &str,
) -> PackEval {
    PackEval::Findings(
        value
            .map(extract_policy_findings)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(ordinal, mut finding)| {
                finding.policy_id = resolve_member_id(manifest, &finding.policy_id);
                record_from_finding(pack_id, changed_path, &finding, ordinal)
            })
            .collect(),
    )
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
/// `anvil gate` policy check's family mapping via the shared, crate single-source
/// vocabulary ([`crate::policy_vocab`]): [`VIOLATION_FAMILY_KEYS`] rule sets are
/// violation-class and [`WARNING_FAMILY_KEYS`] rule sets (including the
/// documented `warning`) are warning-class. Consuming the same consts as
/// `commands::gate::extract_policy_findings` keeps the pre-write and gate
/// surfaces from drifting on which rule families they recognise. An object
/// finding may override its class via a `severity` field (`error` ⇒ violation,
/// `warn`/`warning`/`info` ⇒ warning); an unrecognised override keeps the
/// family default (fail-closed on a typo). Helper rules and other keys are
/// ignored.
fn extract_policy_findings(value: &serde_json::Value) -> Vec<RawFinding> {
    use crate::policy_vocab::{VIOLATION_FAMILY_KEYS, WARNING_FAMILY_KEYS};

    let mut out = Vec::new();
    let Some(map) = value.as_object() else {
        return out;
    };
    let families = [
        (VIOLATION_FAMILY_KEYS, PolicySeverityClass::Violation),
        (WARNING_FAMILY_KEYS, PolicySeverityClass::Warning),
    ];
    for (policy_id, output) in map {
        let Some(obj) = output.as_object() else {
            continue;
        };
        for (keys, default_class) in families {
            for key in keys {
                let Some(arr) = obj.get(*key).and_then(serde_json::Value::as_array) else {
                    continue;
                };
                for item in arr {
                    out.push(raw_finding_from_item(item, policy_id, default_class));
                }
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
fn record_from_finding(
    pack_id: &str,
    changed_path: &str,
    finding: &RawFinding,
    ordinal: usize,
) -> RoutedRecord {
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
    let diagnostic = guidance_to_diagnostic(&guidance, finding.class, changed_path, ordinal);
    RoutedRecord {
        outcome: PolicyOutcome {
            rule_id: finding.policy_id.clone(),
            class: finding.class,
        },
        diagnostic,
        // A real finding: evaluated against `changed_path`'s own policy input,
        // so its diagnostic location is the file the exception lookup must use.
        path_scoped: true,
    }
}

/// Render a [`PolicyGuidance`] into a canonical [`Diagnostic`].
fn guidance_to_diagnostic(
    guidance: &PolicyGuidance,
    class: PolicySeverityClass,
    changed_path: &str,
    ordinal: usize,
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
        // Pack label + per-pack ordinal keep ids unique when one rule emits
        // several findings for the same path, or two packs share a rule name.
        format!(
            "diag_policy_prewrite_{}_{}_{}_{ordinal}",
            sanitise_id_part(changed_path),
            sanitise_id_part(match &guidance.source {
                PolicySource::Pack(p) => p,
                PolicySource::Assertion(id) => id,
                PolicySource::Scanner(n) => n,
            }),
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
        // Pack-level machinery, not a per-file finding: its location is the
        // pass anchor, so a per-file exception grant must not suppress it.
        path_scoped: false,
    }
}

/// Build the single fail-open truncation record for the packs left unevaluated
/// when the pass deadline is reached (AD-5). `evaluated` of `total` packs ran;
/// the remainder are collapsed into this one warning-class outcome that warns
/// but never vetoes under any posture. The packs already evaluated keep their
/// findings.
fn truncation_record(
    evaluated: usize,
    total: usize,
    started: Instant,
    changed_path: &str,
) -> RoutedRecord {
    let remaining = total.saturating_sub(evaluated);
    let elapsed_ms = started.elapsed().as_millis();
    let rule_id = "policy-truncated";
    let diagnostic = Diagnostic::new(
        format!(
            "diag_policy_prewrite_truncated_{}",
            sanitise_id_part(changed_path)
        ),
        // Warning, never error: exhausting the pass budget must not escalate to
        // a veto under a strict posture.
        Severity::Warning,
        format!(
            "Policy evaluation truncated after {elapsed_ms}ms: {evaluated} of {total} pack(s) \
             evaluated; {remaining} remaining pack(s) skipped — failing open (the write is not \
             blocked). Findings from the evaluated pack(s) still stand."
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
            rule_id: rule_id.to_string(),
            source_module: "anvil-cli::mcp::policy_prewrite".to_string(),
        },
        Mode::Unknown(PRE_WRITE_MODE.to_string()),
    )
    .with_remediation_hint(
        "The pre-write policy pass hit its wall-clock budget before evaluating every pack. \
         Compiled engines are cached, but a cold or invalidated pass still compiles under the \
         deadline. Reduce the installed pack count, or run `anvil gate` for a full, unbudgeted \
         policy evaluation.",
    );
    RoutedRecord {
        // Warning-class: routes to `warn` under every enforcing posture and
        // never vetoes (OPAE-007).
        outcome: PolicyOutcome::warning(rule_id),
        diagnostic,
        // Pass-level machinery, not a per-file finding: never suppressible by
        // a per-file exception grant.
        path_scoped: false,
    }
}

/// Drop findings covered by an active tracked exception grant. Load failures
/// fail open (keep the findings) so a broken store cannot hide a veto.
fn suppress_excepted_records(
    workspace_root: &Path,
    records: Vec<RoutedRecord>,
) -> Vec<RoutedRecord> {
    let Ok(store) = ExceptionStore::load(workspace_root) else {
        return records;
    };
    let exceptions: Vec<_> = store.active_exceptions().into_iter().cloned().collect();
    if exceptions.is_empty() {
        return records;
    }
    records
        .into_iter()
        .filter(|record| {
            // Pack-level degradation and truncation warnings describe the pass,
            // not a file; a per-file grant must never hide them.
            if !record.path_scoped {
                return true;
            }
            let violation = Violation {
                policy_id: record.outcome.rule_id.clone(),
                // The record's own path — for a coalesced multi-path save this
                // is the file the finding fired on, so one file's grant cannot
                // suppress (or fail to suppress) another file's finding.
                file: record.diagnostic.location.file.clone(),
                message: String::new(),
                severity: String::new(),
                category: None,
                fingerprint: None,
            };
            !is_suppressed(&violation, &exceptions)
        })
        .collect()
}

/// Append pack diagnostics to `diagnostics` and return the strictest of the
/// intercept-rules scan decision (from current diagnostics) and the routed
/// policy decision. Additive — never suppresses a scan finding; a broken pack
/// or eval failure warns rather than blocks (ADR-098 AD-5).
///
/// Shared by `anvil_validate_write` and `anvil_apply_patch` so path-scoped
/// packs cannot be skipped on the lean added-line path.
pub(crate) fn merge_scan_with_policy(
    workspace_root: &Path,
    changed_path: &str,
    change_kind: ChangeKind,
    diagnostics: &mut Vec<Diagnostic>,
    posture: EnforcementMode,
) -> ControlDecision {
    let scan_decision = enforcement::decision_for(diagnostics, posture);
    let policy = evaluate(workspace_root, changed_path, change_kind, posture);
    diagnostics.extend(policy.diagnostics);
    strictest_decision(scan_decision, policy.decision)
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

    /// Write an installed-style pack into an existing workspace:
    /// `<ws>/.anvil/policies/<id>/pack.yaml` plus one member `.rego`.
    fn write_pack(ws: &std::path::Path, pack_id: &str, member_rego: &str) {
        let pack_dir = ws.join(".anvil/policies").join(pack_id);
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
    }

    /// Write a fresh workspace holding a single installed-style pack.
    fn install_pack(pack_id: &str, member_rego: &str) -> TempDir {
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), pack_id, member_rego);
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

    // The documented canonical warning rule set is `warning` (singular), not
    // `warn` — see `docs/guides/opa-policy-testing.md`. The starter pack emits
    // its advisories under `warning`; this fixture exercises that the pre-write
    // extractor recognises it (it was dropped before the vocabulary fix).
    const WARNING_REGO: &str = r#"package anvil.policies.always_warning
import rego.v1

warning contains msg if {
    some f in input.diff.changed_files
    msg := sprintf("change to %s is advised against by the test policy", [f])
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
    fn policy_prewrite_routing_warning_family_surfaces_warning_class_diagnostic() {
        // Regression guard for the warn/warning vocabulary fix: a pack member
        // emitting `warning`-rule findings (the documented canonical rule set,
        // which the starter pack uses) must surface a warning-class policy
        // diagnostic through the pre-write extractor — and, being warning-class,
        // never veto even under the strictest posture. Before the fix the
        // extractor keyed only on `warn`/`warnings`, so this was dropped.
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = install_pack("warning-pack", WARNING_REGO);
            let outcome = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(outcome.decision, ControlDecision::Warn);
            assert!(!outcome.decision.is_veto(), "a warning must never veto");
            assert!(
                outcome
                    .diagnostics
                    .iter()
                    .any(|d| d.severity == Severity::Warning && d.category == Category::Policy),
                "the documented `warning` rule set must surface a warning-class \
                 policy diagnostic: {:?}",
                outcome.diagnostics,
            );
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
    fn policy_prewrite_routing_deadline_exhaustion_truncates_fail_open() {
        // AD-5: a zero pass budget with two packs installed exhausts the
        // deadline before any pack's compile/eval, so the whole pass truncates
        // into one warning-class outcome — no veto, no error, even under an
        // interrupt posture. (First-N-evaluated-or-zero: zero here.)
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = TempDir::new().expect("workspace");
            write_pack(ws.path(), "pack-a", VIOLATION_REGO);
            write_pack(ws.path(), "pack-b", VIOLATION_REGO);

            let outcome = evaluate_with_budget(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
                Duration::ZERO,
            );

            assert_eq!(
                outcome.decision,
                ControlDecision::Warn,
                "truncation must never veto, even a violation pack under interrupt",
            );
            assert!(!outcome.decision.is_veto());
            // Exactly the truncation warning, and nothing error-severity.
            assert_eq!(outcome.diagnostics.len(), 1);
            assert_eq!(outcome.diagnostics[0].severity, Severity::Warning);
            assert!(
                !outcome
                    .diagnostics
                    .iter()
                    .any(|d| d.severity == Severity::Error),
                "truncation is fail-open — no error diagnostics: {:?}",
                outcome.diagnostics,
            );
            let summary = &outcome.diagnostics[0].summary;
            assert!(summary.contains("truncated"), "{summary}");
            assert!(
                summary.contains("0 of 2 pack(s)"),
                "truncation names the counts: {summary}",
            );
        });
    }

    #[test]
    fn policy_prewrite_routing_within_budget_evaluates_all_packs() {
        // Sanity: with a generous pass budget both packs evaluate (the deadline
        // gate is inert), proving truncation is budget-driven, not structural.
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = TempDir::new().expect("workspace");
            write_pack(ws.path(), "pack-a", VIOLATION_REGO);
            write_pack(ws.path(), "pack-b", WARN_REGO);

            let outcome = evaluate_with_budget(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
                Duration::from_secs(30),
            );
            // The violation pack vetoes under interrupt; the warn pack adds a
            // warning. No truncation diagnostic is present.
            assert_eq!(outcome.decision, ControlDecision::Interrupt);
            assert!(
                !outcome
                    .diagnostics
                    .iter()
                    .any(|d| d.summary.contains("truncated")),
                "no truncation under a generous budget: {:?}",
                outcome.diagnostics,
            );
        });
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

    fn write_member(
        pack_dir: &std::path::Path,
        member_id: &str,
        file_stem: &str,
        package: &str,
        family: &str,
    ) {
        let rel = format!("policies/{file_stem}.rego");
        let source = format!(
            "package anvil.policies.{package}\nimport rego.v1\n\n{family} contains msg if {{\n    some f in input.diff.changed_files\n    msg := sprintf(\"{family} %s\", [f])\n}}\n"
        );
        std::fs::write(pack_dir.join(&rel), source).expect("member");
        let manifest_path = pack_dir.join("pack.yaml");
        let header = if manifest_path.is_file() {
            std::fs::read_to_string(&manifest_path).expect("read manifest")
        } else {
            format!(
                "id: {}\nname: t\nversion: 1.0.0\ndescription: t\nowner: o\npolicies:\n",
                pack_dir.file_name().unwrap().to_string_lossy()
            )
        };
        let manifest = format!(
            "{header}  - path: {rel}\n    metadata:\n      id: {member_id}\n      title: t\n      severity: high\n      owner: o\n      rationale: r\n      scope: diff.changed_files\n      tags: [t]\n"
        );
        std::fs::write(manifest_path, manifest).expect("manifest");
    }

    #[test]
    fn control_examples_overlay_skips_disabled_violation_member() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = TempDir::new().expect("ws");
            let pack_dir = ws.path().join(".anvil/policies/demo");
            std::fs::create_dir_all(pack_dir.join("policies")).expect("dirs");
            write_member(
                &pack_dir,
                "crypto-human-signoff",
                "crypto_human_signoff",
                "crypto_human_signoff",
                "violation",
            );
            write_member(
                &pack_dir,
                "personal-data-paths",
                "personal_data_paths",
                "personal_data_paths",
                "warning",
            );
            let mut overlay = anvil_policy_engine::pack::PackOverlay::default();
            overlay.disable("crypto-human-signoff");
            anvil_policy_engine::pack::save_overlay(
                &ws.path().join(".anvil/policies"),
                "demo",
                &overlay,
            )
            .expect("overlay");
            let outcome = evaluate(
                ws.path(),
                "crypto/src/aes.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert!(
                !outcome.decision.is_veto(),
                "disabled violation member must not veto: {:?}",
                outcome.decision
            );
        });
    }

    #[test]
    fn control_examples_exception_grant_lifts_crypto_veto() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = TempDir::new().expect("ws");
            let pack_dir = ws.path().join(".anvil/policies/demo");
            std::fs::create_dir_all(pack_dir.join("policies")).expect("dirs");
            write_member(
                &pack_dir,
                "crypto-human-signoff",
                "crypto_human_signoff",
                "crypto_human_signoff",
                "violation",
            );
            let blocked = evaluate(
                ws.path(),
                "crypto/src/aes.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert!(
                blocked.decision.is_veto(),
                "crypto violation must veto before a grant"
            );

            let mut store = anvil_policy::exceptions::ExceptionStore::empty();
            store
                .add(anvil_policy::exceptions::PolicyException {
                    schema_version: String::new(),
                    id: String::new(),
                    policy_id: "crypto-human-signoff".into(),
                    file_pattern: String::new(),
                    finding_hash: None,
                    reason: "human reviewed".into(),
                    owner: Some("reviewer".into()),
                    created_by: Some("reviewer@example.test".into()),
                    created_at: chrono::Utc::now(),
                    expires_at: None,
                    revoked: None,
                })
                .expect("add");
            let _ = store.save(ws.path()).expect("save store");

            let allowed = evaluate(
                ws.path(),
                "crypto/src/aes.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert!(
                !allowed.decision.is_veto(),
                "grant must lift the crypto veto: {:?}",
                allowed.decision
            );
        });
    }

    /// ADR-149 batch attribution: a coalesced save carries several changed
    /// paths, and every finding must name the file it fired on — not the
    /// batch's first entry.
    #[test]
    fn policy_prewrite_batch_attributes_findings_to_their_own_path() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let outcome = evaluate_many(
                ws.path(),
                &[
                    ("src/first.rs", ChangeKind::Modified),
                    ("src/second.rs", ChangeKind::Modified),
                    ("src/third.rs", ChangeKind::Modified),
                ],
                EnforcementMode::Warn,
            );
            let files: Vec<&str> = outcome
                .diagnostics
                .iter()
                .map(|d| d.location.file.as_str())
                .collect();
            for expected in ["src/first.rs", "src/second.rs", "src/third.rs"] {
                assert!(
                    files.contains(&expected),
                    "every changed path must carry its own finding, got {files:?}"
                );
            }
            assert!(
                outcome.evaluated,
                "installed packs ran, so the pass must report evaluated"
            );
            // Each finding's message names its own file, not the anchor.
            for diagnostic in &outcome.diagnostics {
                assert!(
                    diagnostic.summary.contains(&diagnostic.location.file),
                    "finding message must describe its own path: {diagnostic:?}"
                );
            }
        });
    }

    /// A grant scoped to one file of a coalesced save must not suppress
    /// another file's finding for the same policy.
    #[test]
    fn policy_prewrite_batch_exception_suppresses_only_the_granted_path() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = TempDir::new().expect("ws");
            let pack_dir = ws.path().join(".anvil/policies/demo");
            std::fs::create_dir_all(pack_dir.join("policies")).expect("dirs");
            write_member(
                &pack_dir,
                "crypto-human-signoff",
                "crypto_human_signoff",
                "crypto_human_signoff",
                "violation",
            );

            let mut store = anvil_policy::exceptions::ExceptionStore::empty();
            store
                .add(anvil_policy::exceptions::PolicyException {
                    schema_version: String::new(),
                    id: String::new(),
                    policy_id: "crypto-human-signoff".into(),
                    file_pattern: "crypto/src/aes.rs".into(),
                    finding_hash: None,
                    reason: "human reviewed".into(),
                    owner: Some("reviewer".into()),
                    created_by: Some("reviewer@example.test".into()),
                    created_at: chrono::Utc::now(),
                    expires_at: None,
                    revoked: None,
                })
                .expect("add");
            let _ = store.save(ws.path()).expect("save store");

            let outcome = evaluate_many(
                ws.path(),
                &[
                    ("crypto/src/aes.rs", ChangeKind::Modified),
                    ("crypto/src/rsa.rs", ChangeKind::Modified),
                ],
                EnforcementMode::Interrupt,
            );
            let files: Vec<&str> = outcome
                .diagnostics
                .iter()
                .map(|d| d.location.file.as_str())
                .collect();
            assert!(
                !files.contains(&"crypto/src/aes.rs"),
                "the granted path's finding must be suppressed: {files:?}"
            );
            assert!(
                files.contains(&"crypto/src/rsa.rs"),
                "an ungranted path's finding must survive the batch: {files:?}"
            );
            assert!(
                outcome.decision.is_veto(),
                "the ungranted path still vetoes under interrupt: {:?}",
                outcome.decision
            );
        });
    }

    /// An inert pass (kill switch, no packs, empty change set) must report
    /// `evaluated: false` so the ADR-149 save-time hook withholds the `policy`
    /// check family instead of claiming a scope that never ran.
    #[test]
    fn policy_prewrite_inert_pass_reports_not_evaluated() {
        temp_env::with_var(POLICY_ENFORCEMENT_ENV, Some("off"), || {
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let outcome = evaluate_many(
                ws.path(),
                &[("src/app.rs", ChangeKind::Modified)],
                EnforcementMode::Interrupt,
            );
            assert!(!outcome.evaluated, "kill switch must report not evaluated");
        });
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            let ws = TempDir::new().expect("workspace");
            let outcome = evaluate_many(
                ws.path(),
                &[("src/app.rs", ChangeKind::Modified)],
                EnforcementMode::Interrupt,
            );
            assert!(!outcome.evaluated, "no packs must report not evaluated");

            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let empty = evaluate_many(ws.path(), &[], EnforcementMode::Interrupt);
            assert!(
                !empty.evaluated,
                "an empty change set must report not evaluated"
            );

            let ran = evaluate_many(
                ws.path(),
                &[("src/app.rs", ChangeKind::Modified)],
                EnforcementMode::Interrupt,
            );
            assert!(ran.evaluated, "an installed pack pass reports evaluated");
        });
    }

    fn bump_mtime(path: &std::path::Path) {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open for mtime");
        let later = std::time::SystemTime::now() + Duration::from_secs(2);
        file.set_modified(later).expect("set mtime");
    }

    #[test]
    fn policy_prewrite_cache_warm_pass_skips_recompile() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            reset_prewrite_engine_cache();
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let first = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(first.decision, ControlDecision::Interrupt);
            assert_eq!(prewrite_engine_compile_count(), 1, "cold pass compiles");
            assert_eq!(prewrite_discovery_count(), 1, "cold pass discovers once");

            let second = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(second.decision, ControlDecision::Interrupt);
            assert_eq!(
                prewrite_engine_compile_count(),
                1,
                "warm pass must be eval-only"
            );
            assert_eq!(
                prewrite_discovery_count(),
                1,
                "warm pass must reuse cached pack/manifest metadata"
            );
        });
    }

    #[test]
    fn policy_prewrite_cache_member_mtime_invalidates() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            reset_prewrite_engine_cache();
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let _ = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(prewrite_engine_compile_count(), 1);

            let member = ws
                .path()
                .join(".anvil/policies/deny-pack/policies/policy.rego");
            bump_mtime(&member);
            let _ = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert_eq!(
                prewrite_engine_compile_count(),
                2,
                "edited member must recompile"
            );
        });
    }

    #[test]
    fn policy_prewrite_cache_overlay_change_invalidates() {
        temp_env::with_var_unset(POLICY_ENFORCEMENT_ENV, || {
            reset_prewrite_engine_cache();
            let ws = install_pack("deny-pack", VIOLATION_REGO);
            let blocked = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert!(blocked.decision.is_veto());
            assert_eq!(prewrite_engine_compile_count(), 1);

            let mut overlay = PackOverlay::default();
            overlay.disabled.push("deny-pack-policy".into());
            anvil_policy_engine::pack::save_overlay(
                &ws.path().join(".anvil/policies"),
                "deny-pack",
                &overlay,
            )
            .expect("overlay");
            bump_mtime(&ws.path().join(".anvil/policies/deny-pack.overlay.yaml"));

            let allowed = evaluate(
                ws.path(),
                "src/app.rs",
                ChangeKind::Modified,
                EnforcementMode::Interrupt,
            );
            assert!(
                !allowed.decision.is_veto(),
                "disabled member must not veto: {:?}",
                allowed.decision
            );
            assert_eq!(
                prewrite_engine_compile_count(),
                2,
                "overlay change must recompile"
            );
        });
    }
}
