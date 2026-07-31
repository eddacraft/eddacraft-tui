//! Read-only `anvil://` MCP resources.
//!
//! Handlers read canonical workspace sources from the server's process-pinned
//! root. Missing or corrupt state is returned in-band; inaccessible roots and
//! snapshot-listing I/O failures are [`ReadError`]s.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{ReadError, contents, descriptor, ensure_known_query_keys, split_uri};

/// `anvil://baseline` — the architecture baseline (`.anvil/architecture.json`).
pub const URI_BASELINE: &str = "anvil://baseline";
/// `anvil://boundaries` — layers and boundary rules derived from the baseline.
pub const URI_BOUNDARIES: &str = "anvil://boundaries";
/// `anvil://patterns` — the built-in anti-pattern catalogue.
pub const URI_PATTERNS: &str = "anvil://patterns";
/// `anvil://suppressions` — active suppressions plus active/expired totals.
pub const URI_SUPPRESSIONS: &str = "anvil://suppressions";
/// `anvil://config` — the discovered anvil config, its source, and parse errors.
pub const URI_CONFIG: &str = "anvil://config";
/// `anvil://constraints` — the aggregated constraint bundle (`anvil export`).
pub const URI_CONSTRAINTS: &str = "anvil://constraints";
/// `anvil://drift` — latest-snapshot drift state and a two-snapshot comparison.
pub const URI_DRIFT: &str = "anvil://drift";

/// The `resources/list` descriptors for the `anvil://` resources.
#[must_use]
pub fn list() -> Vec<Value> {
    vec![
        descriptor(
            URI_BASELINE,
            "Architecture baseline",
            "The workspace architecture baseline (`.anvil/architecture.json`): \
             schema version, entry points, layer definitions, boundary rules, \
             and the baseline violation snapshot. Returns `{ \"error\": \
             \"no-baseline\" }` when no baseline exists, or `{ \"error\": \
             \"baseline-load-failed\" }` when the baseline cannot be loaded.",
        ),
        descriptor(
            URI_BOUNDARIES,
            "Architecture boundaries",
            "Layer definitions and explicit boundary rules derived from the \
             architecture baseline. Returns `{ \"error\": \"no-baseline\" }` \
             when no baseline exists, or `{ \"error\": \"baseline-load-failed\" \
             }` when the baseline cannot be loaded.",
        ),
        descriptor(
            URI_PATTERNS,
            "Anti-pattern catalogue",
            "The built-in anti-pattern catalogue (id, name, category, severity, \
             explanation, suggestion, enablement) and a total count. The \
             authoritative source the `anvil check` anti-pattern scanner uses.",
        ),
        descriptor(
            URI_SUPPRESSIONS,
            "Active suppressions",
            "Active (unexpired) suppressions from `.anvil/suppressions.json`, \
             each with pattern id, file, scope, reason, and optional expiry, \
             plus a summary of total/active/expired counts.",
        ),
        descriptor(
            URI_CONFIG,
            "anvil configuration",
            "The discovered anvil config (`.anvil.{yaml,yml,json,toml}`) parsed \
             to JSON, the resolved source path, an `isDefault` flag when no \
             config file is present, and any parse errors.",
        ),
        descriptor(
            URI_CONSTRAINTS,
            "Constraint bundle",
            "The aggregated constraint bundle — boundaries, layers, \
             anti-patterns, conventions, and active suppressions with metadata \
             — matching `anvil export constraints` except \
             `metadata.workspace_root` is redacted to `.`. The one-call context \
             pack for architecture-aware generation.",
        ),
        descriptor(
            URI_DRIFT,
            "Architecture drift",
            "Drift state from `.anvil/snapshots/`: `no-snapshots`, \
             `single-snapshot` (with the latest metrics), or `ok` with a \
             comparison of the two most recent snapshots (metric deltas and \
             overall trend). Returns `snapshot-load-failed` when a snapshot \
             cannot be loaded.",
        ),
    ]
}

/// Read one `anvil://` resource by `uri`, returning the MCP `resources/read`
/// result envelope.
///
/// # Errors
///
/// [`ReadError::BadRequest`] for an unknown URI or an unexpected query
/// parameter (these resources take none); [`ReadError::Internal`] when the
/// server cwd is inaccessible or drift-snapshot listing fails at the IO level.
/// Structural conditions (no baseline, corrupt snapshot) are **not** errors —
/// they ride in the returned payload.
pub fn read(uri: &str) -> Result<Value, ReadError> {
    let (base, query) = split_uri(uri);
    // Report an unknown URI *before* validating the query, so `anvil://nope?x=1`
    // reads as an unknown-resource error rather than an unexpected-parameter one
    // (council CR-NIT).
    let known = matches!(
        base,
        URI_BASELINE
            | URI_BOUNDARIES
            | URI_PATTERNS
            | URI_SUPPRESSIONS
            | URI_CONFIG
            | URI_CONSTRAINTS
            | URI_DRIFT
    );
    if !known {
        return Err(ReadError::BadRequest(format!(
            "unknown resource uri: {base}"
        )));
    }
    // The anvil:// resources are parameterless; reject any stray query so a
    // typo'd `?foo=…` is a loud BadRequest, not a silently-ignored read.
    ensure_known_query_keys(&query, &[])?;
    let payload = match base {
        URI_BASELINE => read_baseline()?,
        URI_BOUNDARIES => read_boundaries()?,
        URI_PATTERNS => read_patterns(),
        URI_SUPPRESSIONS => read_suppressions()?,
        URI_CONFIG => read_config()?,
        URI_CONSTRAINTS => read_constraints()?,
        URI_DRIFT => read_drift()?,
        _ => unreachable!("base membership checked above"),
    };
    Ok(contents(uri, &payload))
}

/// The process-pinned workspace root: the MCP server's own canonicalised cwd
/// (GCTX-002 CE-8 — stdio-only, no client-supplied root), mirroring the MCP
/// tools' `std::env::current_dir()` contract.
fn workspace_root_path() -> Result<PathBuf, ReadError> {
    let cwd = std::env::current_dir()
        .map_err(|err| ReadError::Internal(format!("MCP server cwd is not accessible: {err}")))?;
    Ok(cwd.canonicalize().unwrap_or(cwd))
}

/// Replace the absolute workspace root with `.` in a message bound for the
/// client. The reader-level error strings (config parse errors, baseline/snapshot
/// IO errors) embed absolute paths; redacting them mirrors the MCP tools'
/// `redact_workspace_root` posture so an in-band error never leaks the operator's
/// filesystem layout (council ADV-1/ADV-5/ADV-6).
fn redact_root(root: &Path, message: &str) -> String {
    let root_str = root.to_string_lossy();
    if root_str.is_empty() {
        message.to_string()
    } else {
        message.replace(root_str.as_ref(), ".")
    }
}

fn read_baseline() -> Result<Value, ReadError> {
    let root = workspace_root_path()?;
    Ok(match anvil_architecture::baseline::load_baseline(&root) {
        Ok(Some(baseline)) => {
            serde_json::to_value(&baseline).expect("architecture baseline serialises")
        }
        Ok(None) => json!({
            "error": "no-baseline",
            "message": "No architecture baseline found. Run `anvil init` to create one.",
        }),
        Err(err) => json!({
            "error": "baseline-load-failed",
            "message": redact_root(&root, &err.to_string()),
        }),
    })
}

fn read_boundaries() -> Result<Value, ReadError> {
    let root = workspace_root_path()?;
    Ok(match anvil_architecture::baseline::load_baseline(&root) {
        Ok(Some(baseline)) => json!({
            "layers": serde_json::to_value(&baseline.layers).expect("layers serialise"),
            "boundaries": serde_json::to_value(&baseline.boundaries).expect("boundaries serialise"),
        }),
        Ok(None) => json!({
            "error": "no-baseline",
            "message": "No architecture baseline found. Run `anvil init` to create one.",
        }),
        Err(err) => json!({
            "error": "baseline-load-failed",
            "message": redact_root(&root, &err.to_string()),
        }),
    })
}

fn read_patterns() -> Value {
    let patterns = anvil_checks::antipattern::patterns::all_patterns();
    json!({
        "count": patterns.len(),
        "patterns": serde_json::to_value(&patterns).expect("anti-pattern catalogue serialises"),
    })
}

fn read_suppressions() -> Result<Value, ReadError> {
    let root = workspace_root_path()?;
    let report = crate::services::suppressions::load_suppressions_report(&root);
    Ok(json!({
        "suppressions": serde_json::to_value(&report.active).expect("suppressions serialise"),
        "summary": {
            "total": report.total,
            "active": report.active.len(),
            "expired": report.expired,
        },
    }))
}

fn read_config() -> Result<Value, ReadError> {
    // Conscious egress decision (council ADV-7): this returns the full parsed
    // `.anvil.*` config, matching the archived TS `anvil://config` contract and
    // the `anvil config` display surface. `.anvil.*` is architecture/check
    // configuration, not a secret store — there is no credential field in the
    // config schema. If credential-bearing config is ever introduced, this
    // surface must be filtered before egress.
    let root = workspace_root_path()?;
    match anvil_config::discover(&root, ".anvil") {
        Ok(Some(discovered)) => {
            let source = discovered
                .path
                .strip_prefix(&root)
                .unwrap_or(&discovered.path)
                .to_string_lossy()
                .replace('\\', "/");
            Ok(match anvil_config::parse_file(&discovered.path) {
                Ok(config) => json!({
                    "config": config,
                    "source": source,
                    "isDefault": false,
                    "errors": [],
                }),
                Err(err) => json!({
                    "config": Value::Null,
                    "source": source,
                    "isDefault": false,
                    "errors": [redact_root(&root, &err.to_string())],
                }),
            })
        }
        Ok(None) => Ok(json!({
            "config": Value::Null,
            "source": Value::Null,
            "isDefault": true,
            "errors": [],
        })),
        Err(err) => Err(ReadError::Internal(format!(
            "config discovery failed: {err}"
        ))),
    }
}

fn read_constraints() -> Result<Value, ReadError> {
    let root = workspace_root_path()?;
    let data = crate::commands::export::collect_constraints(&root);
    let mut value = serde_json::to_value(&data).expect("constraint bundle serialises");
    // `collect_constraints` records the absolute workspace root in `metadata`;
    // redact it to the process-relative `.` before egress (council ADV-2),
    // matching the MCP tools' `redact_workspace_root` posture.
    if let Some(metadata) = value.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.insert("workspace_root".to_string(), json!("."));
    }
    Ok(value)
}

fn read_drift() -> Result<Value, ReadError> {
    use crate::commands::drift;

    let root = workspace_root_path()?;
    let files = drift::list_snapshot_files(&root)
        .map_err(|err| ReadError::Internal(format!("listing drift snapshots failed: {err}")))?;

    if files.is_empty() {
        return Ok(json!({
            "status": "no-snapshots",
            "snapshotCount": 0,
            "message": "No drift snapshots found. Run `anvil drift snapshot` to create one.",
        }));
    }

    // `list_snapshot_files` returns newest-first; the comparison is always of the
    // two most recent. `snapshotCount` reports the true number on disk even when a
    // pathological snapshot directory caps how many are scanned (CIB-084).
    let total = drift::count_snapshot_files(&root)
        .map_err(|err| ReadError::Internal(format!("counting drift snapshots failed: {err}")))?;
    let latest = match drift::load_snapshot_file(&files[0]) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            return Ok(json!({
                "status": "snapshot-load-failed",
                "message": redact_root(&root, &err.to_string()),
            }));
        }
    };

    if files.len() == 1 {
        return Ok(json!({
            "status": "single-snapshot",
            "snapshotCount": 1,
            "latest": snapshot_summary(&latest),
            "message": "Only one snapshot exists. Run `anvil drift snapshot` again to compare.",
        }));
    }

    let previous = match drift::load_snapshot_file(&files[1]) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            // The latest snapshot loaded fine; surface it even though the
            // comparison can't be built (council CR-MINOR).
            return Ok(json!({
                "status": "snapshot-load-failed",
                "message": redact_root(&root, &err.to_string()),
                "latest": snapshot_summary(&latest),
            }));
        }
    };
    let comparison = drift::compare_snapshots(&previous, &latest);

    Ok(json!({
        "status": "ok",
        "snapshotCount": total,
        "latest": snapshot_summary(&latest),
        "comparison": serde_json::to_value(&comparison).expect("drift comparison serialises"),
    }))
}

/// A compact, identity-light view of one drift snapshot for the `latest` field.
fn snapshot_summary(snapshot: &crate::commands::drift::DriftSnapshot) -> Value {
    json!({
        "name": snapshot.name,
        "created_at": snapshot.created_at,
        "metrics": serde_json::to_value(&snapshot.metrics).expect("snapshot metrics serialise"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIME_JSON: &str = "application/json";

    #[test]
    fn list_advertises_the_seven_anvil_resources() {
        let uris: Vec<String> = list()
            .iter()
            .filter_map(|r| r.get("uri").and_then(Value::as_str).map(str::to_string))
            .collect();
        assert_eq!(
            uris,
            vec![
                URI_BASELINE,
                URI_BOUNDARIES,
                URI_PATTERNS,
                URI_SUPPRESSIONS,
                URI_CONFIG,
                URI_CONSTRAINTS,
                URI_DRIFT,
            ]
        );
        for resource in list() {
            assert_eq!(resource["mimeType"], MIME_JSON);
            assert_eq!(resource["annotations"]["readOnlyHint"], json!(true));
        }
    }

    #[test]
    fn list_describes_in_band_error_shapes_and_redaction() {
        let resources = list();
        let description = |uri: &str| {
            resources
                .iter()
                .find(|resource| resource["uri"] == uri)
                .and_then(|resource| resource["description"].as_str())
                .expect("resource description exists")
        };

        assert!(description(URI_BASELINE).contains("baseline-load-failed"));
        assert!(description(URI_BOUNDARIES).contains("baseline-load-failed"));
        assert!(
            description(URI_CONSTRAINTS).contains("metadata.workspace_root")
                && description(URI_CONSTRAINTS).contains("redacted to `.`")
        );
        assert!(description(URI_DRIFT).contains("snapshot-load-failed"));
    }

    #[test]
    fn read_rejects_unknown_anvil_uri_as_bad_request() {
        let err = read("anvil://nope").expect_err("unknown uri is rejected");
        assert!(matches!(err, ReadError::BadRequest(_)), "{err:?}");
        assert!(
            err.reason().contains("unknown resource uri"),
            "{}",
            err.reason()
        );
    }

    #[test]
    fn read_rejects_unexpected_query_param() {
        // The anvil:// resources take no parameters; a stray query is a loud
        // BadRequest rather than a silently-ignored read.
        let err = read("anvil://patterns?foo=bar").expect_err("query is rejected");
        assert!(matches!(err, ReadError::BadRequest(_)), "{err:?}");
    }

    #[test]
    fn patterns_resource_returns_catalogue_with_matching_count() {
        // The anti-pattern catalogue is embedded (cwd-independent).
        let payload = read_patterns();
        let count = payload["count"].as_u64().expect("count is a number");
        let len = payload["patterns"]
            .as_array()
            .expect("patterns is an array")
            .len();
        assert!(count > 0, "catalogue should not be empty");
        assert_eq!(
            count,
            u64::try_from(len).expect("len fits u64"),
            "count must match the array length"
        );
    }

    #[test]
    fn read_patterns_wraps_payload_in_resources_read_envelope() {
        let result = read(URI_PATTERNS).expect("patterns read succeeds");
        let content = &result["contents"][0];
        assert_eq!(content["uri"], URI_PATTERNS);
        assert_eq!(content["mimeType"], MIME_JSON);
        // The payload rides as a JSON string in `text`.
        let text = content["text"].as_str().expect("text is a string");
        let parsed: Value = serde_json::from_str(text).expect("text is JSON");
        assert!(parsed["count"].as_u64().is_some());
    }
}
