//! N-API bindings exposing anvil checks to Node/TypeScript tooling.

use std::panic::{AssertUnwindSafe, catch_unwind};

use anvil_checks::antipattern::{
    Artifact, ArtifactKind, CompiledRegistry, LoadRegistryOptions, ScanOptions,
    compiled_to_antipattern, get_pattern as get_pattern_rust, load_compiled_registry,
    scan_artifact as scan_artifact_rust, types::AntiPattern,
};
use anvil_rayon_init::init_global as init_rayon_pool;
use napi::{Error, Result, Status};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ArtifactInput {
    /// One of: "source", "pr-description", "commit-message", "agent-output".
    kind: String,
    reference: String,
    content: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ScanOptionsInput {
    patterns: Option<Vec<String>>,
    #[serde(rename = "includeOptIn", alias = "include_opt_in")]
    include_opt_in: bool,
}

/// Per-artifact wire shape for the binding. Field names use camelCase to
/// match JS conventions; this is intentionally distinct from the CLI's
/// `CheckOutput` aggregate (see crate-level doc comment).
#[derive(Debug, Serialize)]
struct ScanResultOutput<'a> {
    file: &'a str,
    #[serde(rename = "artifactType")]
    artifact_type: &'static str,
    warnings: &'a [anvil_checks::antipattern::Warning],
    #[serde(rename = "patternsChecked")]
    patterns_checked: &'a [String],
}

/// Load the compiled pattern registry, turning a genuine load failure into a
/// loud `GenericFailure` instead of a silent empty catalogue.
///
/// `anvil_checks::antipattern::load_registry_patterns` (and the `get_*`
/// helpers that layer on it) return `Vec::new()` silently when no registry
/// resolves. That empty-catalogue default is tolerable for a one-shot CLI,
/// but it is the *wrong* default for a napi binding embedded in a long-lived
/// editor host: a silent-empty catalogue means diagnostics quietly stop
/// working, with no signal to the JS caller.
///
/// This helper inverts that default. Note that the compile-time-embedded
/// catalogue (resolution step 4) normally covers a *missing* on-disk
/// registry, so the common path still succeeds; `load_compiled_registry`
/// only yields `registry: None` when a resolved registry fails to load —
/// an I/O read error, or a parse / schema-validation failure — or the
/// embedded fallback itself is broken. In that case JS callers receive a
/// `GenericFailure` with the loader's warning strings so
/// the editor / CLI surface can tell the user what to do (run `anvil
/// doctor`, rebuild the registry, point `registry_path` somewhere valid).
fn load_registry_or_err(opts: &LoadRegistryOptions) -> Result<CompiledRegistry> {
    let result = load_compiled_registry(opts);
    if let Some(reg) = result.registry {
        return Ok(reg);
    }
    let detail = if result.warnings.is_empty() {
        "no diagnostics reported".to_string()
    } else {
        result.warnings.join("; ")
    };
    Err(Error::new(
        Status::GenericFailure,
        format!(
            "anvil scanner registry unavailable: {detail}. Run `anvil doctor` \
             or verify `patterns/compiled/registry.json` exists on disk."
        ),
    ))
}

/// Default catalogue (enabled, non-opt-in) drawn from a loaded registry.
fn default_patterns_from(reg: &CompiledRegistry) -> Vec<AntiPattern> {
    reg.patterns
        .iter()
        .filter_map(compiled_to_antipattern)
        .filter(|p| p.enabled && !p.opt_in)
        .collect()
}

/// Scan a single artifact. Both arguments are JSON strings; the return value
/// is the JSON-serialised `ScanResultOutput` wrapper used by this binding
/// (per-artifact, camelCase, full `Warning` fields) — *not* the Rust
/// `ScanResult` type or the CLI's `--json` shape.
///
/// Errors map to JS `Error` with `Status::InvalidArg` for bad input,
/// `Status::GenericFailure` for registry-load failure (the C1 fix — see
/// `load_registry_or_err`), serialisation failures, or caught scanner
/// panics.
//
// `String` (rather than `&str`) is the napi-rs idiom for JS string args —
// the macro converts the JS value into an owned String at the FFI boundary,
// and downstream `serde_json::from_str` borrows it for the parse. Silence the
// pedantic `needless_pass_by_value` lint here only.
#[allow(clippy::needless_pass_by_value)]
#[napi]
pub fn scan_artifact_json(artifact_json: String, options_json: Option<String>) -> Result<String> {
    // V050F-007: cap rayon's global pool at half available cores so the
    // editor host that loaded this binding does not get its UI thread
    // starved by a `scan_artifact_rust` `par_iter`. Idempotent across
    // calls and across crates (kernel + checks share the same `Once`),
    // so the cost is one atomic load after the first call.
    init_rayon_pool();

    let input: ArtifactInput = serde_json::from_str(&artifact_json)
        .map_err(|e| Error::new(Status::InvalidArg, format!("artifact JSON: {e}")))?;

    let kind = ArtifactKind::from_wire(&input.kind).ok_or_else(|| {
        Error::new(
            Status::InvalidArg,
            format!("unknown artifact kind: {}", input.kind),
        )
    })?;

    let options = match options_json {
        Some(raw) => {
            let parsed: ScanOptionsInput = serde_json::from_str(&raw)
                .map_err(|e| Error::new(Status::InvalidArg, format!("options JSON: {e}")))?;
            Some(ScanOptions {
                patterns: parsed.patterns,
                include_opt_in: parsed.include_opt_in,
            })
        }
        None => None,
    };

    let artifact = Artifact {
        kind,
        reference: input.reference,
        content: input.content,
    };

    // Catch panics so a bug in the scanner — including the registry
    // loader called below — returns a JS error instead of aborting the
    // host Node process. Pairs with the `release-napi` cargo profile
    // (`panic = "unwind"` in workspace Cargo.toml) — `catch_unwind` is a
    // no-op under `panic = "abort"`, so the profile choice is
    // load-bearing here. CLAWP-008 (#1650): the registry load was
    // previously outside this guard, so a panic in `load_compiled_registry`
    // would abort Node instead of becoming a JS error.
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<_> {
        // Fail loudly if the registry isn't loadable. Without this, a
        // missing registry produces an empty catalogue and every scan
        // returns zero warnings — which looks like a passing scan and
        // silently disables enforcement. See council review C1
        // (2026-04-24).
        let _registry = load_registry_or_err(&LoadRegistryOptions::default())?;
        Ok(scan_artifact_rust(&artifact, options.as_ref()))
    }))
    .map_err(|payload| panic_to_error("scanner", &payload))??;

    let output = ScanResultOutput {
        file: &result.file,
        artifact_type: result.artifact_type.as_str(),
        warnings: &result.warnings,
        patterns_checked: &result.patterns_checked,
    };

    serde_json::to_string(&output)
        .map_err(|e| Error::new(Status::GenericFailure, format!("serialise result: {e}")))
}

/// Route a caught panic into a generic JS error + a stderr log entry.
///
/// The raw panic payload is logged to stderr (visible in the daemon /
/// host log) so a developer can debug what actually happened. The JS
/// error returned to the caller carries only a fixed, non-informative
/// message — raw panic payloads can include absolute file paths,
/// partial file content, or internal invariant strings that a consumer
/// surfacing the error to a non-local log or remote agent should not
/// leak. Council review X3 (2026-04-24).
fn panic_to_error(kind: &str, payload: &Box<dyn std::any::Any + Send>) -> Error {
    let detail = if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    };
    // Prefix every line separately so multi-line payloads (backtraces,
    // panic formatters that include file:line context) still carry the
    // `[anvil-checks-napi] {kind} panicked:` tag on each line. Single
    // eprintln! with a `\n` inside would produce continuation lines
    // without the prefix and confuse log aggregators.
    if detail.is_empty() {
        eprintln!("[anvil-checks-napi] {kind} panicked:");
    } else {
        for line in detail.lines() {
            eprintln!("[anvil-checks-napi] {kind} panicked: {line}");
        }
    }
    Error::new(
        Status::GenericFailure,
        format!("{kind} internal error; see host log for details"),
    )
}

/// Returns the binding's semver. Useful for the parity test to assert the
/// loaded `.node` artefact is the one we just built rather than a stale copy.
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Return the default (enabled, non-opt-in) pattern catalogue as a JSON
/// array, computed from a freshly-loaded registry.
///
/// The wire shape is `AntiPattern`'s serde output: camelCase keys where
/// the struct renames them (`fileExtensions`, `allFileTypes`, `optIn`)
/// and `snake_case` otherwise (`definition_ref`, `spectrum_position`).
/// That inconsistency is inherited from the core types; aligning it is
/// tracked separately — changing it here would break the TS consumer
/// parity the wrapper depends on.
///
/// **Registry-unavailable is a hard error**, not a silent-empty success.
/// See `load_registry_or_err` and the crate-level doc comment.
///
/// Wrapped in `catch_unwind` so a panic inside the registry parser or
/// mapping code becomes a JS error rather than a host-Node abort.
#[napi]
pub fn get_default_patterns_json() -> Result<String> {
    catch_unwind(AssertUnwindSafe(|| {
        let reg = load_registry_or_err(&LoadRegistryOptions::default())?;
        let patterns = default_patterns_from(&reg);
        serde_json::to_string(&patterns)
            .map_err(|e| Error::new(Status::GenericFailure, format!("serialise patterns: {e}")))
    }))
    .map_err(|payload| panic_to_error("pattern registry", &payload))?
}

/// Look up a single pattern by id. Returns `null` on the JS side (via
/// `Option<String>`) when the id is unknown, rather than throwing — a
/// miss is a normal negative result, not an error.
///
/// **Registry-unavailable is still a hard error** (distinct from
/// unknown-id): the caller asking for a specific pattern while the
/// registry is unloadable is a bug worth surfacing, not a silent miss.
//
// `String` over `&str` matches the napi-rs idiom for JS-string args — see
// the same allow on `scan_artifact_json`.
#[allow(clippy::needless_pass_by_value)]
#[napi]
pub fn get_pattern_json(id: String) -> Result<Option<String>> {
    catch_unwind(AssertUnwindSafe(|| {
        // Hard-error if the registry isn't loadable, matching the other
        // entry points. Without this, an unloadable registry would return
        // `null` for every id, which is indistinguishable from "id not
        // found in a healthy registry" — a silent downgrade of the
        // consumer's error handling.
        load_registry_or_err(&LoadRegistryOptions::default())?;

        let Some(pattern) = get_pattern_rust(&id) else {
            return Ok(None);
        };
        serde_json::to_string(&pattern)
            .map(Some)
            .map_err(|e| Error::new(Status::GenericFailure, format!("serialise pattern: {e}")))
    }))
    .map_err(|payload| panic_to_error("pattern registry", &payload))?
}

// The registry-missing error path is exercised by a JS integration test
// in `__tests__/registry-missing.test.mjs` — a `cdylib` with napi
// bindings cannot run `cargo test` against itself because the resulting
// test binary does not have the Node runtime's symbol table available
// (it needs `napi_reference_unref`, `napi_delete_reference`, etc., which
// are provided by the host Node process at load time). The JS-side test
// runs the compiled `.node` artefact inside Node where the symbols
// resolve; `node --test` spawns a fresh child process per test file, so
// the failure-path test can set `ANVIL_REGISTRY_PATH` to a bogus value
// without affecting the parity tests that run in sibling files.
