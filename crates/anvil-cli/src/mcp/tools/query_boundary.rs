//! `anvil_query_boundary` — architecture boundary query tool.
//!
//! RMCPF-011 MCP-driver-local composition: reads the live architecture
//! definition (project config `architecture` section or
//! `.anvil/architecture.yaml`) and answers whether `sourceFile` is
//! allowed to import `targetFile` under layered rules. No daemon
//! round-trip — this is local read-only state that the daemon does not
//! own. The TypeScript-era `.anvil/architecture.json` snapshot is not
//! consulted.
//!
//! - `no-architecture`: no definition — the query did **not** evaluate;
//!   `allowed` is false so agents cannot treat silence as a green light.
//! - `architecture-load-failed`: a definition was present but could not
//!   be parsed — also unevaluated, not fail-open.
//! - `same-layer`: source and target resolve to the same layer —
//!   always allowed.
//! - `unassigned-layer`: at least one side has no matching layer —
//!   allowed by default, with the un-assigned side identified.
//! - `boundary-ok` / `boundary-violation`: cross-layer import checked
//!   against the merged default + explicit boundary rules.
//!
//! Workspace containment, redaction, and relative-path validation
//! reuse `shared::validate_workspace_root` so the tool matches the
//! RMCPF-010 contract.

use std::path::Path;

use serde_json::{Value, json};

use anvil_architecture::{
    Boundary, BoundarySeverity, Layers, assign_layers, create_default_boundaries,
};

use crate::mcp::tools::shared::{
    WorkspacePathKind, normalise_workspace_relative_path, redact_workspace_root,
    validate_workspace_root,
};

pub const TOOL_NAME: &str = "anvil_query_boundary";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Check if a file can import from another file given architecture boundary rules. Use before writing import statements to prevent violations.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "sourceFile": {
                    "type": "string",
                    "description": "File that wants to import (relative to workspaceRoot)"
                },
                "targetFile": {
                    "type": "string",
                    "description": "File being imported from (relative to workspaceRoot)"
                },
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                }
            },
            "required": ["sourceFile", "targetFile", "workspaceRoot"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    let payload = match query_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn query_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = crate::mcp::tools::shared::mcp_server_root()?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;
    let source_file = arguments
        .get("sourceFile")
        .and_then(Value::as_str)
        .ok_or_else(|| "sourceFile is required".to_string())?;
    let target_file = arguments
        .get("targetFile")
        .and_then(Value::as_str)
        .ok_or_else(|| "targetFile is required".to_string())?;

    // Lexically normalise both inputs to the same workspace-relative form the
    // validator's scan produces, and reject anchored/escaping shapes. Without
    // this, `./src/x.ts`, `src//x.ts`, or backslash separators fail to match
    // any layer glob and fall through to the "unassigned-layer, allowed by
    // default" verdict — a representation trick that bypasses boundary policy.
    let source_file =
        normalise_workspace_relative_path("sourceFile", source_file, WorkspacePathKind::Policy)?;
    let target_file =
        normalise_workspace_relative_path("targetFile", target_file, WorkspacePathKind::Policy)?;

    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;

    let workspace_str = redact_workspace_root(&workspace_path, &server_root);

    let (layers, rules) = match crate::architecture_source::resolve_architecture(&workspace_path) {
        Ok(Some((definition, _origin))) => (definition.layers, definition.rules),
        Ok(None) => return Ok(no_architecture_payload(&workspace_str)),
        Err(_) => return Ok(architecture_load_failed_payload(&workspace_str)),
    };
    let explicit = boundaries_from_rules(&rules);

    Ok(resolve_query(
        &layers,
        &explicit,
        &source_file,
        &target_file,
        &workspace_str,
    ))
}

fn no_architecture_payload(workspace_str: &str) -> Value {
    json!({
        "allowed": false,
        "evaluated": false,
        "reason": "no-architecture",
        "message": "No architecture definition found. Add an `architecture` section to the project config, or create `.anvil/architecture.yaml`. This query did not evaluate a boundary.",
        "workspaceRoot": workspace_str,
        "backend": "local",
        "daemonStatus": "not-wired"
    })
}

fn architecture_load_failed_payload(workspace_str: &str) -> Value {
    json!({
        "allowed": false,
        "evaluated": false,
        "reason": "architecture-load-failed",
        "message": "Could not load the architecture definition.",
        "workspaceRoot": workspace_str,
        "backend": "local",
        "daemonStatus": "not-wired"
    })
}

fn boundaries_from_rules(
    rules: &[anvil_architecture::definition::ArchitectureRule],
) -> Vec<Boundary> {
    use anvil_architecture::definition::RuleSeverity;
    rules
        .iter()
        .filter_map(|rule| {
            let severity = match rule.severity {
                RuleSeverity::Error => BoundarySeverity::Error,
                RuleSeverity::Warn => BoundarySeverity::Warning,
                RuleSeverity::Info => BoundarySeverity::Info,
                RuleSeverity::Ignore => return None,
            };
            // An allow-list rule overrides the default deny with a non-blocking
            // explicit entry so `effective_boundaries` drops it from the block set.
            let severity = if rule.allowed {
                BoundarySeverity::Info
            } else {
                severity
            };
            Some(Boundary {
                name: rule.name.clone(),
                from: rule.from.clone(),
                to: rule.to.clone(),
                severity,
                message: rule.message.clone().unwrap_or_default(),
                confidence: None,
            })
        })
        .collect()
}

fn resolve_query(
    layers: &Layers,
    explicit: &[Boundary],
    source_file: &str,
    target_file: &str,
    workspace_str: &str,
) -> Value {
    let source_layer = match_layer(source_file, layers);
    let target_layer = match_layer(target_file, layers);

    if source_layer.is_none() || target_layer.is_none() {
        let mut unassigned: Vec<String> = Vec::new();
        if source_layer.is_none() {
            unassigned.push(format!("source ({source_file})"));
        }
        if target_layer.is_none() {
            unassigned.push(format!("target ({target_file})"));
        }
        return json!({
            "allowed": true,
            "reason": "unassigned-layer",
            "message": format!(
                "Cannot determine layer for: {}. Import is allowed by default.",
                unassigned.join(", "),
            ),
            "sourceLayer": source_layer,
            "targetLayer": target_layer,
            "workspaceRoot": workspace_str,
            "backend": "local",
            "daemonStatus": "not-wired"
        });
    }

    let source_layer = source_layer.unwrap();
    let target_layer = target_layer.unwrap();

    if source_layer == target_layer {
        return json!({
            "allowed": true,
            "reason": "same-layer",
            "message": format!(
                "Both files are in the \"{source_layer}\" layer. Same-layer imports are always allowed.",
            ),
            "sourceLayer": source_layer,
            "targetLayer": target_layer,
            "workspaceRoot": workspace_str,
            "backend": "local",
            "daemonStatus": "not-wired"
        });
    }

    // Default boundaries forbid every cross-layer pair not listed in
    // `depends_on`. Explicit `boundaries` entries override severity but the
    // schema only uses these for blocking checks (Error severity blocks).
    let boundaries = effective_boundaries(layers, explicit);

    if let Some(boundary) = boundaries
        .iter()
        .find(|b| b.from == source_layer && b.to == target_layer)
    {
        let severity = severity_str(&boundary.severity);
        return json!({
            "allowed": false,
            "reason": "boundary-violation",
            "message": format!(
                "Import from \"{source_layer}\" to \"{target_layer}\" violates architecture boundaries.",
            ),
            "sourceLayer": source_layer,
            "targetLayer": target_layer,
            "violation": {
                "from": source_layer,
                "to": target_layer,
                "boundary": boundary.name,
                "severity": severity
            },
            "workspaceRoot": workspace_str,
            "backend": "local",
            "daemonStatus": "not-wired"
        });
    }

    json!({
        "allowed": true,
        "reason": "boundary-ok",
        "message": format!(
            "Import from \"{source_layer}\" to \"{target_layer}\" is allowed by architecture rules.",
        ),
        "sourceLayer": source_layer,
        "targetLayer": target_layer,
        "workspaceRoot": workspace_str,
        "backend": "local",
        "daemonStatus": "not-wired"
    })
}

/// Match a file to a layer by delegating to `anvil_architecture::assign_layers`,
/// the same matcher the validator uses, so the boundary check stays aligned
/// with `anvil check` output.
fn match_layer(file: &str, layers: &Layers) -> Option<String> {
    let assignments = assign_layers(&[file.to_string()], layers);
    assignments.into_iter().next().and_then(|a| a.layer)
}

/// Build the effective deny-boundary set: default deny boundaries derived from
/// `depends_on`, with explicit `boundaries` entries that share a `(from, to)`
/// pair overriding the default. We only consider Error-severity boundaries as
/// blocking, matching the archived TS behaviour (where the layer detector's
/// `isAllowedDependency` returned `false` for any forbidden cross-layer pair).
fn effective_boundaries(layers: &Layers, explicit: &[Boundary]) -> Vec<Boundary> {
    let mut merged: Vec<Boundary> = create_default_boundaries(layers);
    for rule in explicit {
        if let Some(existing) = merged
            .iter_mut()
            .find(|b| b.from == rule.from && b.to == rule.to)
        {
            *existing = rule.clone();
        } else {
            merged.push(rule.clone());
        }
    }
    merged
        .into_iter()
        .filter(|b| matches!(b.severity, BoundarySeverity::Error))
        .collect()
}

fn severity_str(severity: &BoundarySeverity) -> &'static str {
    match severity {
        BoundarySeverity::Error => "error",
        BoundarySeverity::Warning => "warning",
        BoundarySeverity::Info => "info",
    }
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("query_boundary payload serialises");
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": payload.get("error").is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_architecture::{
        ArchitectureDefinition, ArchitectureTemplate, Layer, Layers, write_architecture_yaml,
    };
    use std::collections::BTreeMap;
    use std::path::Path;

    fn sample_layers() -> Layers {
        let mut layers: Layers = BTreeMap::new();
        layers.insert(
            "presentation".into(),
            Layer {
                patterns: vec!["**/controllers/**".into(), "**/routes/**".into()],
                depends_on: vec!["application".into(), "shared".into()],
                description: None,
            },
        );
        layers.insert(
            "application".into(),
            Layer {
                patterns: vec!["**/services/**".into()],
                depends_on: vec!["domain".into(), "shared".into()],
                description: None,
            },
        );
        layers.insert(
            "domain".into(),
            Layer {
                patterns: vec!["**/domain/**".into()],
                depends_on: vec!["shared".into()],
                description: None,
            },
        );
        layers.insert(
            "shared".into(),
            Layer {
                patterns: vec!["**/utils/**".into()],
                depends_on: vec![],
                description: None,
            },
        );
        layers
    }

    fn write_definition(root: &Path, layers: Layers) {
        let definition = ArchitectureDefinition {
            schema_version: "0.1.0".into(),
            template: ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![],
            options: None,
        };
        write_architecture_yaml(root, &definition).expect("definition persists");
    }

    fn workspace_with_baseline() -> tempfile::TempDir {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_definition(workspace.path(), sample_layers());
        workspace
    }

    /// Layers whose globs are **anchored** (no leading `**/`), so an
    /// unnormalised `./`, `//`, or `\` input fails to match. This makes the
    /// normalisation load-bearing: with the raw string these paths would fall
    /// through to `unassigned-layer`; only after normalisation do they resolve.
    fn anchored_layers() -> Layers {
        let mut layers: Layers = BTreeMap::new();
        layers.insert(
            "domain".into(),
            Layer {
                patterns: vec!["src/domain/**".into()],
                depends_on: vec![],
                description: None,
            },
        );
        layers
    }

    fn workspace_with_anchored_baseline() -> tempfile::TempDir {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_definition(workspace.path(), anchored_layers());
        workspace
    }

    #[test]
    fn descriptor_advertises_required_fields() {
        let descriptor = descriptor();
        assert_eq!(descriptor["name"], TOOL_NAME);
        let required = descriptor["inputSchema"]["required"]
            .as_array()
            .expect("required is an array");
        assert!(required.contains(&json!("sourceFile")));
        assert!(required.contains(&json!("targetFile")));
        assert!(required.contains(&json!("workspaceRoot")));
    }

    #[test]
    fn rejects_missing_workspace_root() {
        let result = call(&json!({
            "sourceFile": "src/domain/a.ts",
            "targetFile": "src/utils/b.ts"
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "workspaceRoot is required");
    }

    #[test]
    fn rejects_empty_source_file() {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "sourceFile": "",
            "targetFile": "src/utils/b.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "sourceFile must not be empty");
    }

    #[test]
    fn rejects_workspace_outside_server_root() {
        let other = tempfile::tempdir().expect("foreign workspace exists");
        let result = call(&json!({
            "sourceFile": "src/a.ts",
            "targetFile": "src/b.ts",
            "workspaceRoot": other.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            crate::mcp::tools::shared::WORKSPACE_ROOT_NOT_ADMITTED
        );
    }

    #[test]
    fn returns_no_architecture_when_workspace_has_none() {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "sourceFile": "src/controllers/user.ts",
            "targetFile": "src/domain/user.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], false);
        assert_eq!(payload["evaluated"], false);
        assert_eq!(payload["reason"], "no-architecture");
        assert!(
            payload["message"]
                .as_str()
                .unwrap()
                .contains("architecture.yaml")
        );
        assert_eq!(payload["backend"], "local");
    }

    #[test]
    fn architecture_json_snapshot_is_not_a_source() {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let anvil_dir = workspace.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).expect("dir created");
        std::fs::write(anvil_dir.join("architecture.json"), "{}").expect("file written");
        let result = call(&json!({
            "sourceFile": "src/controllers/user.ts",
            "targetFile": "src/services/user-service.ts",
            "workspaceRoot": workspace.path()
        }));
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["reason"], "no-architecture");
        assert_eq!(payload["allowed"], false);
        assert_eq!(payload["evaluated"], false);
    }

    #[test]
    fn returns_same_layer_when_files_share_a_layer() {
        let workspace = workspace_with_baseline();
        let result = call(&json!({
            "sourceFile": "src/domain/user.ts",
            "targetFile": "src/domain/order.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], true);
        assert_eq!(payload["reason"], "same-layer");
        assert_eq!(payload["sourceLayer"], "domain");
        assert_eq!(payload["targetLayer"], "domain");
    }

    #[test]
    fn returns_boundary_ok_for_allowed_cross_layer_import() {
        let workspace = workspace_with_baseline();
        let result = call(&json!({
            "sourceFile": "src/controllers/user.ts",
            "targetFile": "src/services/user-service.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], true);
        assert_eq!(payload["reason"], "boundary-ok");
        assert_eq!(payload["sourceLayer"], "presentation");
        assert_eq!(payload["targetLayer"], "application");
    }

    #[test]
    fn returns_boundary_violation_for_blocked_cross_layer_import() {
        let workspace = workspace_with_baseline();
        let result = call(&json!({
            "sourceFile": "src/domain/user.ts",
            "targetFile": "src/controllers/user-controller.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], false);
        assert_eq!(payload["reason"], "boundary-violation");
        assert_eq!(payload["sourceLayer"], "domain");
        assert_eq!(payload["targetLayer"], "presentation");
        assert_eq!(payload["violation"]["from"], "domain");
        assert_eq!(payload["violation"]["to"], "presentation");
        assert_eq!(payload["violation"]["severity"], "error");
    }

    #[test]
    fn reports_boundary_violation_for_rust_files() {
        // RSTLAN-007: the MCP boundary-query surface assigns layers and flags
        // violations for `.rs` files exactly as for `.ts` — layer matching is
        // path-glob based, so Rust crates participate with no language gate.
        let workspace = workspace_with_baseline();
        let result = call(&json!({
            "sourceFile": "src/domain/entity.rs",
            "targetFile": "src/controllers/handler.rs",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], false);
        assert_eq!(payload["reason"], "boundary-violation");
        assert_eq!(payload["sourceLayer"], "domain");
        assert_eq!(payload["targetLayer"], "presentation");
    }

    #[test]
    fn returns_unassigned_layer_when_source_does_not_match() {
        let workspace = workspace_with_baseline();
        let result = call(&json!({
            "sourceFile": "src/misc/helper.ts",
            "targetFile": "src/domain/user.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], true);
        assert_eq!(payload["reason"], "unassigned-layer");
        assert!(payload["sourceLayer"].is_null());
        assert_eq!(payload["targetLayer"], "domain");
        assert!(
            payload["message"]
                .as_str()
                .unwrap()
                .contains("source (src/misc/helper.ts)")
        );
    }

    #[test]
    fn returns_architecture_load_failed_for_invalid_yaml() {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let anvil_dir = workspace.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).expect("dir created");
        std::fs::write(anvil_dir.join("architecture.yaml"), "not: [unterminated")
            .expect("file written");

        let result = call(&json!({
            "sourceFile": "src/a.ts",
            "targetFile": "src/b.ts",
            "workspaceRoot": workspace.path()
        }));

        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], false);
        assert_eq!(payload["evaluated"], false);
        assert_eq!(payload["reason"], "architecture-load-failed");
    }

    #[test]
    fn explicit_warning_severity_does_not_block_import() {
        // Sanity: only Error-severity boundaries are blocking. Layers where
        // a default deny is downgraded by an explicit warning rule should
        // allow the import.
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let layers = sample_layers();
        let definition = ArchitectureDefinition {
            schema_version: "0.1.0".into(),
            template: ArchitectureTemplate::Custom,
            layers,
            bounded_contexts: None,
            rules: vec![anvil_architecture::definition::ArchitectureRule {
                name: "presentation-domain-warning".into(),
                from: "presentation".into(),
                to: "domain".into(),
                severity: anvil_architecture::definition::RuleSeverity::Warn,
                allowed: false,
                message: Some("warning".into()),
            }],
            options: None,
        };
        write_architecture_yaml(workspace.path(), &definition).expect("definition persists");

        let result = call(&json!({
            "sourceFile": "src/controllers/user.ts",
            "targetFile": "src/domain/user.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], true);
        assert_eq!(payload["reason"], "boundary-ok");
    }

    // --- CIB-148: path normalisation before layer matching ----------------

    /// Helper: resolve a `(source, target)` pair against the anchored baseline.
    fn query_anchored(source: &str, target: &str) -> Value {
        let workspace = workspace_with_anchored_baseline();
        let result = call(&json!({
            "sourceFile": source,
            "targetFile": target,
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    #[test]
    fn normalises_leading_dot_slash_to_same_layer() {
        // Baseline: raw `src/domain/x.ts` -> `domain`.
        let plain = query_anchored("src/domain/a.ts", "src/domain/b.ts");
        assert_eq!(plain["sourceLayer"], "domain");
        // `./`-prefixed inputs must resolve identically, not fall through to
        // `unassigned-layer`.
        let dotted = query_anchored("./src/domain/a.ts", "./src/domain/b.ts");
        assert_eq!(dotted["reason"], "same-layer");
        assert_eq!(dotted["sourceLayer"], "domain");
        assert_eq!(dotted["targetLayer"], "domain");
    }

    #[test]
    fn normalises_redundant_separators_to_same_layer() {
        let doubled = query_anchored("src//domain/a.ts", "src/domain//b.ts");
        assert_eq!(doubled["reason"], "same-layer");
        assert_eq!(doubled["sourceLayer"], "domain");
        assert_eq!(doubled["targetLayer"], "domain");
    }

    #[test]
    fn normalises_backslash_separators_to_same_layer() {
        let back = query_anchored("src\\domain\\a.ts", "src\\domain\\b.ts");
        assert_eq!(back["reason"], "same-layer");
        assert_eq!(back["sourceLayer"], "domain");
        assert_eq!(back["targetLayer"], "domain");
    }

    #[test]
    fn rejects_parent_dir_escape_rather_than_allowing() {
        let workspace = workspace_with_anchored_baseline();
        let result = call(&json!({
            "sourceFile": "../escape.ts",
            "targetFile": "src/domain/b.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "sourceFile must not escape the workspace via \"..\""
        );
        // Crucially, the escaping input must NOT produce an allow verdict.
        assert!(payload.get("allowed").is_none());
    }

    #[test]
    fn rejects_absolute_source_rather_than_allowing() {
        let workspace = workspace_with_anchored_baseline();
        let result = call(&json!({
            "sourceFile": "/abs/path.ts",
            "targetFile": "src/domain/b.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "sourceFile must be a workspace-relative path"
        );
        assert!(payload.get("allowed").is_none());
    }

    #[test]
    fn rejects_windows_drive_target_rather_than_allowing() {
        let workspace = workspace_with_anchored_baseline();
        let result = call(&json!({
            "sourceFile": "src/domain/a.ts",
            "targetFile": "C:\\Windows\\system32\\evil.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "targetFile must be a workspace-relative path"
        );
        assert!(payload.get("allowed").is_none());
    }

    #[test]
    fn wellformed_unmatched_path_stays_unassigned_allowed() {
        // A genuinely unmatched-but-well-formed path keeps the by-design
        // fail-open posture: only representation tricks are being closed.
        let workspace = workspace_with_anchored_baseline();
        let result = call(&json!({
            "sourceFile": "docs/readme.md",
            "targetFile": "src/domain/b.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["allowed"], true);
        assert_eq!(payload["reason"], "unassigned-layer");
        assert!(payload["sourceLayer"].is_null());
        assert_eq!(payload["targetLayer"], "domain");
    }

    #[test]
    fn empty_after_normalisation_is_rejected_as_empty() {
        // `./` normalises to nothing — the extended emptiness check catches it
        // with the same message shape as a literal empty string.
        let workspace = workspace_with_anchored_baseline();
        let result = call(&json!({
            "sourceFile": "./",
            "targetFile": "src/domain/b.ts",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "sourceFile must not be empty");
        assert!(payload.get("allowed").is_none());
    }
}
