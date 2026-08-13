use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::format::ConfigFormat;

/// MLP2-060: maximum on-disk size for an `.anvil.*` config file, in
/// bytes. Picked at 1 MiB because a hand-edited project config that
/// big is almost certainly an authored mistake (or an attempted
/// resource-exhaustion payload); valid configs are <10 KiB in
/// practice. The cap fires before `read_to_string`, so a hostile
/// file never lands in memory.
pub const MAX_CONFIG_FILE_BYTES: u64 = 1024 * 1024;

/// MLP2-060: maximum allowed nesting depth of the parsed
/// `serde_json::Value`. Picked at 32 — operator-realistic
/// `.anvil.*` configs never go deeper than ~6 (top-level →
/// `enforcement` → `session` → scalar), so 32 leaves ample
/// headroom while still defending against pathological inputs
/// that survived the alias-rejection pre-pass.
pub const MAX_PARSED_DEPTH: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("io reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// UCFG-014: a FIFO, device, directory, or other non-regular
    /// target. Opened with `O_NONBLOCK` (Unix) so this surfaces
    /// promptly instead of hanging the command.
    #[error("config path {path} is not a regular file (FIFO, device, or directory)")]
    NotARegularFile { path: PathBuf },
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
    /// MLP2-060: the file exceeds [`MAX_CONFIG_FILE_BYTES`]. Surfaces
    /// before any parsing so a hostile payload cannot land in memory.
    #[error(
        "config file {path} is {size} bytes; exceeds the {cap}-byte limit \
         (operator configs are <10 KiB in practice)"
    )]
    FileTooLarge { path: PathBuf, size: u64, cap: u64 },
    /// MLP2-060: a YAML alias (`*name`) was found at parse time.
    /// `.anvil.*` configs are hand-edited; no operator legitimately
    /// needs anchors and aliases here, so the cheap defence is to
    /// reject them outright. This eliminates the billion-laughs
    /// expansion vector before `serde_yaml` materialises the graph.
    #[error(
        "yaml in {path} contains an alias / anchor; `.anvil.*` configs \
         do not support YAML anchors (rewrite without `&`/`*` or use \
         JSON/TOML instead)"
    )]
    AliasNotPermitted { path: PathBuf },
    /// MLP2-060: the parsed `serde_json::Value` nests deeper than
    /// [`MAX_PARSED_DEPTH`]. Defence-in-depth against deeply-nested
    /// payloads that arrived without aliases (operator hand-typing
    /// something pathological, JSON or TOML inputs that the alias
    /// check does not apply to).
    #[error(
        "parsed config in {path} nests {depth} levels deep; exceeds the \
         {cap}-level limit"
    )]
    DepthExceeded {
        path: PathBuf,
        depth: usize,
        cap: usize,
    },
}

/// Parse `contents` as `format` into a `serde_json::Value`.
///
/// `path` is used only for error annotation — the function never reads
/// from it. Pass [`Path::new("<inline>")`](std::path::Path::new) when
/// parsing string literals in tests.
///
/// MLP2-060 resource bounds apply uniformly across formats:
///
/// 1. **Parse-time alias rejection** (YAML only). The byte scanner in
///    [`scan_for_yaml_aliases`] rejects any `*alias` or `&anchor`
///    token before `serde_yaml` materialises the alias graph. This
///    eliminates the billion-laughs expansion vector at the gate,
///    not after the damage is done. Documented limitation: the
///    scanner is conservative — a `&` or `*` in an unquoted scalar
///    context that genuinely intends a literal will be flagged.
///    Mitigation: quote the value, or use JSON / TOML for
///    pathological configs (no project should hit this in
///    practice).
/// 2. **Post-parse depth cap** (every format). Applied after a
///    successful parse via [`enforce_depth_cap`]. Catches a
///    deeply-nested JSON / TOML that arrived without YAML
///    aliases, or a hand-typed pathology.
pub fn parse_str(contents: &str, format: ConfigFormat, path: &Path) -> Result<Value, ParseError> {
    let parsed = match format {
        ConfigFormat::Yaml | ConfigFormat::Yml => {
            // MLP2-060 step 1: reject aliases at the gate.
            scan_for_yaml_aliases(contents, path)?;
            // `serde_yaml::from_str::<Value>` deserialises straight to a
            // JSON value because both crates share the same intermediate
            // shape for scalar/map/seq. yaml-specific types (Tagged
            // values, aliases) collapse to the plain payload — which is
            // what `rules_sha` callers want anyway.
            serde_yaml::from_str(contents).map_err(|source| ParseError::Yaml {
                path: path.to_path_buf(),
                source,
            })?
        }
        ConfigFormat::Json => {
            serde_json::from_str(contents).map_err(|source| ParseError::Json {
                path: path.to_path_buf(),
                source,
            })?
        }
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
            toml_value_to_json(&toml_value, path)?
        }
    };
    // MLP2-060 step 2: post-parse depth-cap. Runs on every format
    // so a deeply-nested JSON / TOML payload that bypassed the
    // YAML-only alias check still surfaces a typed error.
    enforce_depth_cap(&parsed, path)?;
    Ok(parsed)
}

/// MLP2-060: scan `contents` for YAML alias (`*name`) / anchor
/// (`&name`) tokens at value positions, rejecting if any are found.
///
/// The scanner walks bytes left-to-right tracking three contexts:
///
/// - inside a `"..."` double-quoted scalar
/// - inside a `'...'` single-quoted scalar
/// - inside a `#` comment (until newline)
///
/// While in any of those contexts, `&` / `*` bytes are scalar
/// content and ignored. Outside, the scanner looks for `&` or `*`
/// followed by `[A-Za-z0-9_-]` — the YAML alias-name production —
/// and returns [`ParseError::AliasNotPermitted`] on the first hit.
///
/// **Conservative on purpose.** A `*` operator in a flow-style
/// numeric scalar (`value: 5*3`) or `&` in an unquoted email-like
/// string would false-positive. `.anvil.*` configs are simple
/// key/value/list YAML; the conservative scanner is fine in
/// practice, and operators with a legitimate need can switch the
/// file to JSON or TOML (also recognised by `discover()`).
pub fn scan_for_yaml_aliases(contents: &str, path: &Path) -> Result<(), ParseError> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Ctx {
        Body,
        DoubleQuote,
        SingleQuote,
        Comment,
    }
    let mut ctx = Ctx::Body;
    let bytes = contents.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match ctx {
            Ctx::Body => {
                if b == b'#' {
                    ctx = Ctx::Comment;
                } else if b == b'"' {
                    ctx = Ctx::DoubleQuote;
                } else if b == b'\'' {
                    ctx = Ctx::SingleQuote;
                } else if (b == b'&' || b == b'*') && {
                    let next = bytes.get(i + 1).copied().unwrap_or(0);
                    next.is_ascii_alphanumeric() || next == b'_' || next == b'-'
                } {
                    return Err(ParseError::AliasNotPermitted {
                        path: path.to_path_buf(),
                    });
                }
            }
            Ctx::DoubleQuote => {
                // YAML double-quoted scalars honour backslash escapes;
                // skip the next byte after `\` so an escaped `\"`
                // does not terminate the string.
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    ctx = Ctx::Body;
                }
            }
            Ctx::SingleQuote => {
                // YAML single-quoted scalars use `''` to escape an
                // embedded apostrophe — they do NOT honour backslash.
                if b == b'\'' {
                    ctx = Ctx::Body;
                }
            }
            Ctx::Comment => {
                if b == b'\n' {
                    ctx = Ctx::Body;
                }
            }
        }
        i += 1;
    }
    Ok(())
}

/// MLP2-060: refuse parsed values nested deeper than
/// [`MAX_PARSED_DEPTH`].
pub fn enforce_depth_cap(value: &Value, path: &Path) -> Result<(), ParseError> {
    fn walk(value: &Value, depth: usize, max: usize) -> Result<usize, usize> {
        if depth > max {
            return Err(depth);
        }
        match value {
            Value::Object(map) => {
                let mut deepest = depth;
                for v in map.values() {
                    deepest = deepest.max(walk(v, depth + 1, max)?);
                }
                Ok(deepest)
            }
            Value::Array(items) => {
                let mut deepest = depth;
                for v in items {
                    deepest = deepest.max(walk(v, depth + 1, max)?);
                }
                Ok(deepest)
            }
            _ => Ok(depth),
        }
    }
    match walk(value, 1, MAX_PARSED_DEPTH) {
        Ok(_) => Ok(()),
        Err(depth) => Err(ParseError::DepthExceeded {
            path: path.to_path_buf(),
            depth,
            cap: MAX_PARSED_DEPTH,
        }),
    }
}

/// Parse the file at `path` according to its extension.
///
/// Combines [`ConfigFormat::from_path`] + size-cap check + read +
/// [`parse_str`] in the expected order so callers don't have to
/// assemble the trio themselves.
///
/// MLP2-060: the size of the file is checked via `fs::metadata`
/// before `read_to_string` so a hostile payload never lands in
/// memory. Files over [`MAX_CONFIG_FILE_BYTES`] surface as
/// [`ParseError::FileTooLarge`].
pub fn parse_file(path: &Path) -> Result<Value, ParseError> {
    let format =
        ConfigFormat::from_path(path).ok_or_else(|| ParseError::UnrecognisedExtension {
            path: path.to_path_buf(),
        })?;
    let contents = read_to_string_bounded(path)?;
    parse_str(&contents, format, path)
}

/// MLP2-063: read `path` into a `String`, refusing files larger than
/// [`MAX_CONFIG_FILE_BYTES`] before the body lands in memory.
///
/// Exposed so policy loaders in `anvil-cli` (pre-push hook and
/// `l4-validate`) share the same bounded read path as `.anvil.*`
/// config parsing — a hostile or malformed
/// `anvil/policy.{yml,yaml,json,toml}` cannot now exhaust memory
/// through the policy loader.
///
/// **TOCTOU note (Council quick review).** Opening the file once and
/// then querying `metadata` on the resulting file descriptor binds
/// the size check to the same inode the read will consume; a
/// concurrent rename or truncate cannot swap a larger payload in
/// between. The read is additionally bounded by
/// [`std::io::Read::take`] at `MAX_CONFIG_FILE_BYTES + 1` so a file
/// that grows past the cap *after* fstat but before EOF still
/// surfaces as [`ParseError::FileTooLarge`] rather than overflowing
/// the buffer.
///
/// **UCFG-014.** The open uses `O_NONBLOCK` on Unix and then
/// `fstat`s the held descriptor, so a FIFO (or other non-regular
/// target) cannot hang `anvil gate` / `config` / `doctor` waiting
/// for a writer. The delegation-layer `stat` guard is now
/// defence-in-depth against the same class.
pub fn read_to_string_bounded(path: &Path) -> Result<String, ParseError> {
    use std::io::Read;
    let file = open_regular_nonblocking(path)?;
    let metadata = file.metadata().map_err(|source| ParseError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.len() > MAX_CONFIG_FILE_BYTES {
        return Err(ParseError::FileTooLarge {
            path: path.to_path_buf(),
            size: metadata.len(),
            cap: MAX_CONFIG_FILE_BYTES,
        });
    }
    // The `+ 1` lets us detect a file that has grown past the cap
    // since `fstat`: if `read_to_string` consumes more than the cap,
    // we treat it identically to the pre-read size failure.
    let cap_plus_one = MAX_CONFIG_FILE_BYTES.saturating_add(1);
    let mut limited = file.take(cap_plus_one);
    let mut contents = String::new();
    limited
        .read_to_string(&mut contents)
        .map_err(|source| ParseError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if contents.len() as u64 > MAX_CONFIG_FILE_BYTES {
        return Err(ParseError::FileTooLarge {
            path: path.to_path_buf(),
            size: contents.len() as u64,
            cap: MAX_CONFIG_FILE_BYTES,
        });
    }
    Ok(contents)
}

/// Open `path` without blocking on a FIFO, then refuse anything
/// that is not a regular file on the held descriptor.
fn open_regular_nonblocking(path: &Path) -> Result<std::fs::File, ParseError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        // O_NONBLOCK: a FIFO open does not wait for a writer.
        // O_CLOEXEC: keep the handle out of child processes.
        // The type check is fstat-on-the-held-fd (not a second path
        // lookup) so a regular→FIFO swap between a prior stat and
        // this open cannot hang the command.
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)
            .map_err(|source| ParseError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let meta = file.metadata().map_err(|source| ParseError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if !meta.is_file() {
            return Err(ParseError::NotARegularFile {
                path: path.to_path_buf(),
            });
        }
        Ok(file)
    }

    #[cfg(not(unix))]
    {
        // Best-effort: follow the path (so a symlink to a regular
        // config still works) and refuse anything that is not a
        // regular file. A concurrent swap remains possible here.
        let meta = std::fs::metadata(path).map_err(|source| ParseError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if !meta.is_file() {
            return Err(ParseError::NotARegularFile {
                path: path.to_path_buf(),
            });
        }
        std::fs::File::open(path).map_err(|source| ParseError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
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

    // ── MLP2-060 — YAML resource bounds ────────────────────────────

    use tempfile::TempDir;

    /// MLP2-060: a hand-crafted billion-laughs payload (~200 bytes,
    /// would expand to gigabytes under unbounded alias-resolution)
    /// is rejected at the alias-scanner gate **before** `serde_yaml`
    /// materialises the graph. This is the headline test the spec
    /// called for — pinning the primary attack surface.
    #[test]
    fn billion_laughs_payload_is_rejected_at_alias_scanner() {
        // Classic YAML billion-laughs structure. Each `*aN` reference
        // would expand to the previous level's content; nine nested
        // levels yield ~9^9 ≈ 400 million leaf nodes when expanded.
        let payload = "\
a0: &a0 [lol]
a1: &a1 [*a0, *a0, *a0, *a0, *a0, *a0, *a0, *a0, *a0]
a2: &a2 [*a1, *a1, *a1, *a1, *a1, *a1, *a1, *a1, *a1]
a3: &a3 [*a2, *a2, *a2, *a2, *a2, *a2, *a2, *a2, *a2]
a4: &a4 [*a3, *a3, *a3, *a3, *a3, *a3, *a3, *a3, *a3]
";
        let err = parse_str(payload, ConfigFormat::Yaml, Path::new("billion.yaml"))
            .expect_err("billion-laughs payload must be rejected");
        match err {
            ParseError::AliasNotPermitted { path } => {
                assert_eq!(path, PathBuf::from("billion.yaml"));
            }
            other => panic!("expected AliasNotPermitted, got {other:?}"),
        }
    }

    /// MLP2-060: a single anchor declaration is rejected even
    /// without any alias references. The anchor itself is the entry
    /// point to the expansion vector — refusing both halves keeps
    /// the rule simple.
    #[test]
    fn yaml_anchor_alone_is_rejected() {
        let payload = "anchor: &a foo\n";
        let err = parse_str(payload, ConfigFormat::Yaml, Path::new("x.yaml"))
            .expect_err("&-anchor must be rejected");
        assert!(matches!(err, ParseError::AliasNotPermitted { .. }));
    }

    /// MLP2-060: a single alias reference is rejected.
    #[test]
    fn yaml_alias_alone_is_rejected() {
        let payload = "ref: *a\n";
        let err = parse_str(payload, ConfigFormat::Yaml, Path::new("x.yaml"))
            .expect_err("*-alias must be rejected");
        assert!(matches!(err, ParseError::AliasNotPermitted { .. }));
    }

    /// MLP2-060: `&` / `*` bytes inside double-quoted strings are
    /// scalar content, not anchors / aliases — the scanner correctly
    /// treats them as data.
    #[test]
    fn ampersand_or_star_inside_double_quoted_string_is_accepted() {
        let payload = "url: \"https://example.com/a&b=*\"\n";
        let v = parse_str(payload, ConfigFormat::Yaml, Path::new("x.yaml")).unwrap();
        assert_eq!(v, json!({"url": "https://example.com/a&b=*"}));
    }

    /// MLP2-060: `&` / `*` inside single-quoted strings are also
    /// scalar content. Single-quote escaping is `''` for an
    /// embedded apostrophe, not backslash.
    #[test]
    fn ampersand_or_star_inside_single_quoted_string_is_accepted() {
        let payload = "label: 'a&b *foo'\n";
        let v = parse_str(payload, ConfigFormat::Yaml, Path::new("x.yaml")).unwrap();
        assert_eq!(v, json!({"label": "a&b *foo"}));
    }

    /// MLP2-060: `&` / `*` inside comments are scanner-invisible.
    #[test]
    fn ampersand_or_star_inside_comment_is_accepted() {
        let payload = "mode: warn # see &a or *b for context\n";
        let v = parse_str(payload, ConfigFormat::Yaml, Path::new("x.yaml")).unwrap();
        assert_eq!(v, json!({"mode": "warn"}));
    }

    /// MLP2-060: an operator-realistic `.anvil.yaml` (no anchors,
    /// no aliases) parses cleanly through the new gate.
    #[test]
    fn operator_realistic_yaml_passes_alias_scanner() {
        let payload = "\
enforcement:
  mode: warn
  session:
    per_worktree_max: 8
telemetry:
  allow_cross_session: false
";
        let v = parse_str(payload, ConfigFormat::Yaml, Path::new("anvil.yaml")).unwrap();
        assert_eq!(v["enforcement"]["mode"], "warn");
        assert_eq!(v["enforcement"]["session"]["per_worktree_max"], 8);
    }

    /// MLP2-060: a >1 MiB file is rejected by `parse_file` BEFORE
    /// `read_to_string`. The cap fires at the `fs::metadata` check.
    #[test]
    fn parse_file_rejects_oversized_payload() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("huge.json");
        // 1 MiB + 1 byte of valid JSON. Use a long string value so
        // the file parses as JSON if the size check weren't there.
        // Cast through `usize::try_from` to keep clippy happy on the
        // unlikely 32-bit target; the cap fits in `usize` everywhere
        // anvil-config builds.
        let pad = usize::try_from(MAX_CONFIG_FILE_BYTES + 16).expect("cap fits in usize");
        let body = format!("{{\"x\":\"{}\"}}", "a".repeat(pad));
        std::fs::write(&path, &body).unwrap();
        let err = parse_file(&path).expect_err("oversized payload must be rejected");
        match err {
            ParseError::FileTooLarge { size, cap, .. } => {
                assert!(size > cap);
                assert_eq!(cap, MAX_CONFIG_FILE_BYTES);
            }
            other => panic!("expected FileTooLarge, got {other:?}"),
        }
    }

    /// MLP2-060: a JSON / TOML payload nested past `MAX_PARSED_DEPTH`
    /// is rejected at the post-parse depth-walk. JSON / TOML don't
    /// have aliases, so this is the only defence for those formats.
    #[test]
    fn deeply_nested_json_is_rejected_by_depth_cap() {
        // Build a 40-deep JSON object: `{"k":{"k":{...{"k":1}...}}}`.
        let mut payload = String::new();
        for _ in 0..40 {
            payload.push_str("{\"k\":");
        }
        payload.push('1');
        for _ in 0..40 {
            payload.push('}');
        }
        let err = parse_str(&payload, ConfigFormat::Json, Path::new("deep.json"))
            .expect_err("depth cap must fire");
        match err {
            ParseError::DepthExceeded { depth, cap, .. } => {
                assert!(depth > cap);
                assert_eq!(cap, MAX_PARSED_DEPTH);
            }
            other => panic!("expected DepthExceeded, got {other:?}"),
        }
    }

    /// MLP2-060: depth-cap accepts a payload at the limit.
    #[test]
    fn depth_at_cap_is_accepted() {
        // 30 levels — well under the 32 cap.
        let mut payload = String::new();
        for _ in 0..30 {
            payload.push_str("{\"k\":");
        }
        payload.push('1');
        for _ in 0..30 {
            payload.push('}');
        }
        parse_str(&payload, ConfigFormat::Json, Path::new("ok.json"))
            .expect("under-cap depth must parse");
    }

    /// UCFG-014: a FIFO as the main config must fail promptly, not
    /// hang `File::open` waiting for a writer.
    #[cfg(unix)]
    #[test]
    fn fifo_main_config_is_rejected_without_blocking() {
        use std::sync::mpsc;
        use std::time::Duration;

        let tmp = TempDir::new().unwrap();
        let fifo = tmp.path().join(".anvil.yaml");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo");
        assert!(status.success(), "mkfifo failed: {status}");

        let path = fifo.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(read_to_string_bounded(&path));
        });
        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("FIFO open must not block");
        let err = result.expect_err("FIFO must be refused");
        assert!(
            matches!(err, ParseError::NotARegularFile { .. }),
            "got {err:?}"
        );
    }

    /// UCFG-014: `parse_file` (gate/config/doctor entry) inherits the
    /// same non-blocking regular-file guard.
    #[cfg(unix)]
    #[test]
    fn parse_file_fifo_is_rejected_without_blocking() {
        use std::sync::mpsc;
        use std::time::Duration;

        let tmp = TempDir::new().unwrap();
        let fifo = tmp.path().join(".anvil.yaml");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo");
        assert!(status.success(), "mkfifo failed: {status}");

        let path = fifo.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(parse_file(&path));
        });
        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("FIFO parse_file must not block");
        let err = result.expect_err("FIFO must be refused");
        assert!(
            matches!(err, ParseError::NotARegularFile { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn directory_is_rejected_as_not_a_regular_file() {
        let tmp = TempDir::new().unwrap();
        let err = read_to_string_bounded(tmp.path()).expect_err("directory must be refused");
        assert!(
            matches!(err, ParseError::NotARegularFile { .. }),
            "got {err:?}"
        );
    }
}
