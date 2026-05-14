use std::path::Path;

use serde_json::{Value, json};

use crate::commands::check_catalog;
use crate::mcp::validation::DaemonStatus;

pub const TOOL_NAME: &str = "anvil_status";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Quick project health summary. Returns available checks, configuration info, and baseline status.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                }
            },
            "required": ["workspaceRoot"],
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
    let payload = match status_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn status_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = std::env::current_dir()
        .map_err(|err| format!("MCP server cwd is not accessible: {err}"))?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot must be an absolute path".to_string())?;
    let workspace_path = Path::new(workspace_root);
    validate_workspace_root(workspace_path, &server_root)?;

    let config = load_config_info(workspace_path);
    let has_baseline = workspace_path.join(".anvil/architecture.json").is_file();
    let available_checks = check_catalog::gate_canonical_names();

    Ok(json!({
        "status": "ok",
        "workspaceRoot": ".",
        "availableChecks": available_checks,
        "config": config,
        "hasBaseline": has_baseline,
        "version": env!("CARGO_PKG_VERSION"),
        "backend": "local",
        "daemonStatus": DaemonStatus::NotWired.as_str()
    }))
}

fn validate_workspace_root(workspace_root: &Path, server_root: &Path) -> Result<(), String> {
    if !workspace_root.is_absolute() {
        return Err("workspaceRoot must be an absolute path".to_string());
    }
    if !workspace_root.exists() {
        return Err("workspaceRoot does not exist".to_string());
    }
    if !workspace_root.is_dir() {
        return Err("workspaceRoot must be a directory".to_string());
    }
    let server_root = server_root
        .canonicalize()
        .map_err(|err| format!("MCP server root is not accessible: {err}"))?;
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|err| format!("workspaceRoot is not accessible: {err}"))?;
    if !workspace_root.starts_with(&server_root) {
        return Err("workspaceRoot must be inside the MCP server root".to_string());
    }
    Ok(())
}

fn load_config_info(workspace_root: &Path) -> Value {
    let source = workspace_root.join(".anvilrc");
    let contents = match std::fs::read_to_string(&source) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return json!({
                "loaded": false,
                "source": null,
                "checks": []
            });
        }
        Err(err) => {
            return json!({
                "loaded": false,
                "source": null,
                "checks": [],
                "error": format!("Failed to read .anvilrc: {err}")
            });
        }
    };

    match parse_config_checks(&contents) {
        Ok(checks) => json!({
            "loaded": true,
            "source": ".anvilrc",
            "checks": checks
        }),
        Err(error) => json!({
            "loaded": false,
            "source": null,
            "checks": [],
            "error": error
        }),
    }
}

fn parse_config_checks(contents: &str) -> Result<Vec<String>, String> {
    let checks = if let Ok(value) = serde_json::from_str::<Value>(contents) {
        if !value.is_object() {
            return Err("Failed to parse .anvilrc: JSON config must be an object".to_string());
        }
        extract_checks_from_json(&value)
    } else if let Ok(value) = toml::from_str::<toml::Value>(contents) {
        if !value.is_table() {
            return Err("Failed to parse .anvilrc: TOML config must be a table".to_string());
        }
        extract_checks_from_toml(&value)
    } else if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(contents) {
        if !value.is_mapping() {
            return Err("Failed to parse .anvilrc: YAML config must be a mapping".to_string());
        }
        extract_checks_from_yaml(&value)
    } else {
        return Err("Failed to parse .anvilrc as JSON, YAML, or TOML".to_string());
    };

    Ok(checks
        .into_iter()
        .map(|name| {
            check_catalog::canonical_check_name(&name)
                .unwrap_or(&name)
                .to_string()
        })
        .collect())
}

fn extract_checks_from_json(value: &Value) -> Vec<String> {
    value
        .get("checks")
        .and_then(Value::as_array)
        .map(|checks| {
            checks
                .iter()
                .filter_map(|check| {
                    check.as_str().map(ToString::to_string).or_else(|| {
                        let enabled = check
                            .get("enabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                        enabled
                            .then(|| check.get("name").and_then(Value::as_str))
                            .flatten()
                            .map(ToString::to_string)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_checks_from_yaml(value: &serde_yaml::Value) -> Vec<String> {
    value
        .get("checks")
        .and_then(serde_yaml::Value::as_sequence)
        .map(|checks| {
            checks
                .iter()
                .filter_map(|check| {
                    check.as_str().map(ToString::to_string).or_else(|| {
                        let enabled = check
                            .get("enabled")
                            .and_then(serde_yaml::Value::as_bool)
                            .unwrap_or(true);
                        enabled
                            .then(|| check.get("name").and_then(serde_yaml::Value::as_str))
                            .flatten()
                            .map(ToString::to_string)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_checks_from_toml(value: &toml::Value) -> Vec<String> {
    value
        .get("checks")
        .and_then(toml::Value::as_array)
        .map(|checks| {
            checks
                .iter()
                .filter_map(|check| {
                    check.as_str().map(ToString::to_string).or_else(|| {
                        let enabled = check
                            .get("enabled")
                            .and_then(toml::Value::as_bool)
                            .unwrap_or(true);
                        enabled
                            .then(|| check.get("name").and_then(toml::Value::as_str))
                            .flatten()
                            .map(ToString::to_string)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("status payload serialises");
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

    #[test]
    fn status_rejects_relative_workspace_root() {
        let result = call(&json!({ "workspaceRoot": "." }));

        assert_eq!(result["isError"], true);
        let payload: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap())
            .expect("payload is JSON");
        assert_eq!(payload["error"], "workspaceRoot must be an absolute path");
    }

    #[test]
    fn status_reports_missing_config_as_not_loaded() {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(cwd).expect("workspace exists");

        let result = call(&json!({ "workspaceRoot": workspace.path() }));

        assert_eq!(result["isError"], false);
        let payload: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap())
            .expect("payload is JSON");
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["config"]["loaded"], false);
        assert_eq!(payload["backend"], "local");
        assert_eq!(payload["daemonStatus"], "not-wired");
    }

    #[test]
    fn config_checks_parse_json_yaml_and_toml() {
        assert_eq!(
            parse_config_checks(r#"{"checks":["secret","policy"]}"#).unwrap(),
            vec!["secret-detection", "policy"]
        );
        assert_eq!(
            parse_config_checks("checks:\n  - secret-detection\n  - import-boundaries\n").unwrap(),
            vec!["secret-detection", "import-boundaries"]
        );
        assert_eq!(
            parse_config_checks(r#"checks = ["architecture", "antipattern-scan"]"#).unwrap(),
            vec!["import-boundaries", "antipattern-scan"]
        );
    }

    #[test]
    fn config_checks_parse_object_entries_and_skip_disabled_checks() {
        assert_eq!(
            parse_config_checks(
                r#"{"checks":[{"name":"secret","enabled":true},{"name":"policy","enabled":false},{"name":"architecture"}]}"#,
            )
            .unwrap(),
            vec!["secret-detection", "import-boundaries"]
        );
        assert_eq!(
            parse_config_checks(
                "checks:\n  - name: secret\n    enabled: true\n  - name: policy\n    enabled: false\n",
            )
            .unwrap(),
            vec!["secret-detection"]
        );
        assert_eq!(
            parse_config_checks(
                "[[checks]]\nname = \"architecture\"\nenabled = true\n\n[[checks]]\nname = \"policy\"\nenabled = false\n",
            )
            .unwrap(),
            vec!["import-boundaries"]
        );
    }

    #[test]
    fn config_checks_reject_malformed_config() {
        let error = parse_config_checks("not: [valid").unwrap_err();

        assert_eq!(error, "Failed to parse .anvilrc as JSON, YAML, or TOML");
    }
}
