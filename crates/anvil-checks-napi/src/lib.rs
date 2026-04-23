//! Node-API binding for the authoritative Anvil scanner.
//!
//! TSRET-001 spike scope: prove `anvil_checks::antipattern::scan_artifact` can
//! be called from Node with acceptable startup and per-call overhead, and that
//! the result is byte-for-byte identical to what the Rust CLI produces.
//!
//! Wire shape is intentionally JSON-in / JSON-out for the spike. It keeps
//! the binding small and avoids committing to a typed napi surface before
//! the `VSCode` extension and MCP server consumer needs are pinned (typed
//! bindings can layer on top in TSRET-003/-004).
//!
//! **The binding's JSON is NOT identical to `anvil check --json`.** The CLI
//! wraps multiple files in an aggregate `CheckOutput` and projects warnings
//! through a narrow `JsonWarning` (flat `file`/`line`, ~9 fields). This
//! binding emits a per-artifact `ScanResultOutput` carrying the full
//! `Warning` struct (~17 fields, nested `location`). Both shapes derive
//! from the same `scan_artifact` call, so the warning *content* is parity
//! by construction — the *envelope* is deliberately different because the
//! binding operates one artifact at a time and exposes the richer Warning
//! fields consumers may want. Document any consumer that crosses both
//! surfaces.

use std::panic::{AssertUnwindSafe, catch_unwind};

use anvil_checks::antipattern::{
    Artifact, ArtifactKind, ScanOptions, scan_artifact as scan_artifact_rust,
};
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

/// Scan a single artifact. Both arguments are JSON strings; the return value
/// is the JSON-serialised `ScanResult`.
///
/// Errors map to JS `Error` with `Status::InvalidArg` for bad input or
/// `Status::GenericFailure` for serialisation failures (which should be
/// unreachable but route through a real error rather than a panic).
//
// `String` (rather than `&str`) is the napi-rs idiom for JS string args —
// the macro converts the JS value into an owned String at the FFI boundary,
// and downstream `serde_json::from_str` borrows it for the parse. Silence the
// pedantic `needless_pass_by_value` lint here only.
#[allow(clippy::needless_pass_by_value)]
#[napi]
pub fn scan_artifact_json(artifact_json: String, options_json: Option<String>) -> Result<String> {
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

    // Catch panics so a bug in the scanner (or any of its transitive
    // dependencies) returns a JS error instead of aborting the host Node
    // process. Pairs with the `release-napi` cargo profile (`panic =
    // "unwind"` in workspace Cargo.toml) — `catch_unwind` is a no-op under
    // `panic = "abort"`, so the profile choice is load-bearing here.
    let result = catch_unwind(AssertUnwindSafe(|| {
        scan_artifact_rust(&artifact, options.as_ref())
    }))
    .map_err(|payload| {
        let msg = panic_message(&payload);
        Error::new(Status::GenericFailure, format!("scanner panicked: {msg}"))
    })?;

    let output = ScanResultOutput {
        file: &result.file,
        artifact_type: result.artifact_type.as_str(),
        warnings: &result.warnings,
        patterns_checked: &result.patterns_checked,
    };

    serde_json::to_string(&output)
        .map_err(|e| Error::new(Status::GenericFailure, format!("serialise result: {e}")))
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

/// Returns the binding's semver. Useful for the parity test to assert the
/// loaded `.node` artefact is the one we just built rather than a stale copy.
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
