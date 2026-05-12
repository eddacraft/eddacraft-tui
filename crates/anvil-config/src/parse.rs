use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::format::ConfigFormat;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("io reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unrecognised extension on {path}: only yaml/yml/json/toml are accepted")]
    UnrecognisedExtension { path: PathBuf },
    #[error("yaml parse error in {path}: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("json parse error in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("toml parse error in {path}: {source}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("toml float in {path} is not representable in canonical JSON: {value}")]
    NonFiniteTomlFloat { path: PathBuf, value: f64 },
}

/// Parse `contents` as `format` into a `serde_json::Value`.
///
/// `path` is used only for error annotation — the function never reads
/// from it. Pass [`Path::new("<inline>")`](std::path::Path::new) when
/// parsing string literals in tests.
pub fn parse_str(contents: &str, format: ConfigFormat, path: &Path) -> Result<Value, ParseError> {
    match format {
        ConfigFormat::Yaml | ConfigFormat::Yml => {
            // `serde_yaml::from_str::<Value>` deserialises straight to a
            // JSON value because both crates share the same intermediate
            // shape for scalar/map/seq. yaml-specific types (Tagged
            // values, aliases) collapse to the plain payload — which is
            // what `rules_sha` callers want anyway.
            serde_yaml::from_str(contents).map_err(|source| ParseError::Yaml {
                path: path.to_path_buf(),
                source,
            })
        }
        ConfigFormat::Json => serde_json::from_str(contents).map_err(|source| ParseError::Json {
            path: path.to_path_buf(),
            source,
        }),
        ConfigFormat::Toml => {
            // toml -> toml::Value -> serde_json::Value. The double-parse
            // is the simplest path that preserves toml's stricter type
            // model (notably its date types collapse to string scalars,
            // matching the rest of the loader's coercion rules).
            let toml_value: toml::Value =
                toml::from_str(contents).map_err(|source| ParseError::Toml {
                    path: path.to_path_buf(),
                    source,
                })?;
            toml_value_to_json(&toml_value, path)
        }
    }
}

/// Parse the file at `path` according to its extension.
///
/// Combines [`ConfigFormat::from_path`] + read + [`parse_str`] in the
/// expected order so callers don't have to assemble the trio
/// themselves.
pub fn parse_file(path: &Path) -> Result<Value, ParseError> {
    let format =
        ConfigFormat::from_path(path).ok_or_else(|| ParseError::UnrecognisedExtension {
            path: path.to_path_buf(),
        })?;
    let contents = std::fs::read_to_string(path).map_err(|source| ParseError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_str(&contents, format, path)
}

/// Convert a `toml::Value` to a `serde_json::Value`.
///
/// Coercion rules:
/// - String, Integer, Float, Boolean → matching JSON scalar.
/// - Datetime → JSON string (preserves the original lexical form).
/// - Array, Table → recursive conversion.
///
/// Non-finite floats (NaN, ±Infinity) are rejected with
/// [`ParseError::NonFiniteTomlFloat`]. Silently mapping them to JSON
/// `null` would collapse distinct inputs to the same canonical bytes
/// and break the format-independent `rules_sha` invariant.
///
/// `Map` keys are inserted in their natural toml-iteration order;
/// [`crate::canonical_json_bytes`] re-sorts at serialisation time so
/// key ordering at the `Value` level is not load-bearing.
fn toml_value_to_json(value: &toml::Value, path: &Path) -> Result<Value, ParseError> {
    use serde_json::Map;
    Ok(match value {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::Number((*i).into()),
        toml::Value::Float(f) => match serde_json::Number::from_f64(*f) {
            Some(n) => Value::Number(n),
            None => {
                return Err(ParseError::NonFiniteTomlFloat {
                    path: path.to_path_buf(),
                    value: *f,
                });
            }
        },
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| toml_value_to_json(v, path))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        toml::Value::Table(table) => {
            let mut map = Map::with_capacity(table.len());
            for (k, v) in table {
                map.insert(k.clone(), toml_value_to_json(v, path)?);
            }
            Value::Object(map)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_str_yaml_basic() {
        let v = parse_str(
            "version: 1\nchecks:\n  - secrets\n",
            ConfigFormat::Yaml,
            Path::new("x"),
        )
        .unwrap();
        assert_eq!(v, json!({"version": 1, "checks": ["secrets"]}));
    }

    #[test]
    fn parse_str_yml_basic() {
        // .yml and .yaml share the same parser path.
        let v = parse_str("version: 1\n", ConfigFormat::Yml, Path::new("x")).unwrap();
        assert_eq!(v, json!({"version": 1}));
    }

    #[test]
    fn parse_str_json_basic() {
        let v = parse_str(
            r#"{"version": 1, "checks": ["secrets"]}"#,
            ConfigFormat::Json,
            Path::new("x"),
        )
        .unwrap();
        assert_eq!(v, json!({"version": 1, "checks": ["secrets"]}));
    }

    #[test]
    fn parse_str_toml_basic() {
        let v = parse_str(
            "version = 1\nchecks = [\"secrets\"]\n",
            ConfigFormat::Toml,
            Path::new("x"),
        )
        .unwrap();
        assert_eq!(v, json!({"version": 1, "checks": ["secrets"]}));
    }

    #[test]
    fn parse_str_yaml_nested() {
        let v = parse_str(
            "version: 1\nthresholds:\n  overall_score: 80\n",
            ConfigFormat::Yaml,
            Path::new("x"),
        )
        .unwrap();
        assert_eq!(
            v,
            json!({"version": 1, "thresholds": {"overall_score": 80}})
        );
    }

    #[test]
    fn parse_str_toml_nested() {
        let v = parse_str(
            "version = 1\n[thresholds]\noverall_score = 80\n",
            ConfigFormat::Toml,
            Path::new("x"),
        )
        .unwrap();
        assert_eq!(
            v,
            json!({"version": 1, "thresholds": {"overall_score": 80}})
        );
    }

    #[test]
    fn parse_str_yaml_null_preserved() {
        let v = parse_str("a: null\n", ConfigFormat::Yaml, Path::new("x")).unwrap();
        assert_eq!(v, json!({"a": null}));
    }

    #[test]
    fn parse_str_json_invalid_returns_error() {
        let err = parse_str("not json", ConfigFormat::Json, Path::new("bad.json")).unwrap_err();
        assert!(matches!(err, ParseError::Json { .. }));
        let msg = err.to_string();
        assert!(
            msg.contains("bad.json"),
            "error should reference the path: {msg}"
        );
    }

    #[test]
    fn parse_str_yaml_invalid_returns_error() {
        // unbalanced flow mapping — yaml rejects.
        let err = parse_str("a: {\n", ConfigFormat::Yaml, Path::new("bad.yaml")).unwrap_err();
        assert!(matches!(err, ParseError::Yaml { .. }));
    }

    #[test]
    fn parse_str_toml_invalid_returns_error() {
        let err = parse_str("= bad", ConfigFormat::Toml, Path::new("bad.toml")).unwrap_err();
        assert!(matches!(err, ParseError::Toml { .. }));
    }

    #[test]
    fn parse_file_dispatches_on_extension() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(&path, r#"{"k": 1}"#).unwrap();
        let v = parse_file(&path).unwrap();
        assert_eq!(v, json!({"k": 1}));
    }

    #[test]
    fn parse_file_rejects_unknown_extension() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("c.ini");
        std::fs::write(&path, "ignored").unwrap();
        let err = parse_file(&path).unwrap_err();
        assert!(matches!(err, ParseError::UnrecognisedExtension { .. }));
    }

    #[test]
    fn parse_file_missing_returns_io_error() {
        // Use a guaranteed-missing path inside a TempDir so the test
        // works identically on Windows and Unix.
        let dir = tempfile::TempDir::new().unwrap();
        let missing = dir.path().join("definitely-not-there.json");
        let err = parse_file(&missing).unwrap_err();
        assert!(matches!(err, ParseError::Io { .. }));
    }

    #[test]
    fn parse_str_toml_nan_is_rejected() {
        // TOML allows `nan` as a literal; the parser must surface it
        // as `NonFiniteTomlFloat` rather than silently mapping to
        // JSON null (which would collapse distinct inputs to the same
        // canonical bytes and break `rules_sha`).
        let err = parse_str("x = nan\n", ConfigFormat::Toml, Path::new("t.toml")).unwrap_err();
        assert!(
            matches!(err, ParseError::NonFiniteTomlFloat { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn parse_str_toml_inf_is_rejected() {
        let err = parse_str("x = inf\n", ConfigFormat::Toml, Path::new("t.toml")).unwrap_err();
        assert!(
            matches!(err, ParseError::NonFiniteTomlFloat { .. }),
            "got {err:?}"
        );
        let err = parse_str("x = -inf\n", ConfigFormat::Toml, Path::new("t.toml")).unwrap_err();
        assert!(
            matches!(err, ParseError::NonFiniteTomlFloat { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn parse_str_toml_datetime_coerces_to_string() {
        // Datetimes preserve their lexical form so the canonical hash
        // doesn't depend on tz normalisation choices.
        let v = parse_str(
            "ts = 2026-05-13T12:00:00Z\n",
            ConfigFormat::Toml,
            Path::new("x"),
        )
        .unwrap();
        assert_eq!(v, json!({"ts": "2026-05-13T12:00:00Z"}));
    }
}
