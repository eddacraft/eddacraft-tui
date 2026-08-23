//! Load the antipattern catalogue from `patterns/compiled/registry.json`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::antipattern::types::{AntiPattern, AntiPatternCategory, Confidence, WarningSeverity};

#[cfg(test)]
const REGISTRY_RELATIVE_PATH: &str = "patterns/compiled/registry.json";
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// Compile-time-embedded copy of `patterns/compiled/registry.json`. Used as
/// the final fallback when no on-disk registry is discovered, so stock
/// installs (`~/.cargo/bin/anvil`, Homebrew, cargo-dist tarballs) enforce
/// the same rule pack the binary was built against. Refreshed by
/// `build.rs`'s `rerun-if-changed` directive whenever the source JSON
/// changes.
const EMBEDDED_REGISTRY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../patterns/compiled/registry.json"
));

/// Sentinel `source_path` value used when the embedded fallback is loaded.
/// Carried on `LoadRegistryResult.source_path` so diagnostics can
/// distinguish on-disk loads from the baked-in pack without leaking a
/// real filesystem path the operator could mistake for an editable file.
const EMBEDDED_REGISTRY_SOURCE_LABEL: &str = "<embedded>";

// =============================================================================
// Compiled registry wire format (mirrors format/schemas.ts)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Detection {
    Regex {
        pattern: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        flags: Option<String>,
    },
    Ast {
        ast_query: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledPattern {
    pub id: String,
    pub family: String,
    pub title: String,
    pub version: u32,
    pub severity: WarningSeverity,
    pub confidence: Confidence,
    pub spectrum_position: u32,
    pub targets: Vec<String>,
    pub detection: Detection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_extensions: Option<Vec<String>>,
    #[serde(default)]
    pub allowlist: Vec<String>,
    pub nudge: String,
    #[serde(default)]
    pub related: Vec<String>,
    pub enabled: bool,
    pub opt_in: bool,
    pub family_name: String,
    pub category: String,
    pub explanation: String,
    pub suggestion: String,
    pub definition_ref: String,
    #[serde(default)]
    pub tensions: Vec<String>,
    #[serde(default)]
    pub related_families: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FamilyEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub definition_ref: String,
    pub rules: Vec<String>,
    #[serde(default)]
    pub related: Vec<String>,
    #[serde(default)]
    pub tensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledRegistry {
    pub schema_version: u32,
    pub compiled_at: String,
    pub source_root: String,
    pub patterns: Vec<CompiledPattern>,
    pub prefixes: BTreeMap<String, String>,
    pub families: Vec<FamilyEntry>,
}

// =============================================================================
// Loader API
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct LoadRegistryOptions {
    /// Absolute path to a `registry.json` — unsigned override of the
    /// embedded catalogue (POLFIT-008 / ADR-131). Tests and operator
    /// tooling; not implicit project discovery.
    pub registry_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LoadRegistryResult {
    pub registry: Option<CompiledRegistry>,
    /// Source path that was actually used, for diagnostics.
    pub source_path: Option<PathBuf>,
    /// Non-fatal issues (missing file, parse error, schema mismatch).
    pub warnings: Vec<String>,
}

// =============================================================================
// Path resolution
// =============================================================================

/// Outcome of path resolution. Distinguishes "a path was supplied / set
/// but does not exist" (`OverrideMissing`) from "no explicit override"
/// (`NoneFound`) so the loader can decide whether to warn the operator
/// or silently fall through to the embedded fallback.
///
/// Implicit cwd and executable-directory walks are closed (POLFIT-008 /
/// ADR-131). A cloned `patterns/compiled/registry.json` does not replace
/// the compile-time catalogue.
enum ResolvedPath {
    Found(PathBuf),
    OverrideMissing { source: &'static str, value: String },
    NoneFound,
}

fn resolve_registry_path(opts: &LoadRegistryOptions) -> ResolvedPath {
    if let Some(p) = opts.registry_path.as_ref() {
        return if p.exists() {
            ResolvedPath::Found(p.clone())
        } else {
            ResolvedPath::OverrideMissing {
                source: "registry_path",
                value: p.display().to_string(),
            }
        };
    }

    if let Ok(env_path) = std::env::var("ANVIL_REGISTRY_PATH") {
        let p = PathBuf::from(&env_path);
        return if p.exists() {
            ResolvedPath::Found(p)
        } else {
            ResolvedPath::OverrideMissing {
                source: "ANVIL_REGISTRY_PATH",
                value: env_path,
            }
        };
    }

    ResolvedPath::NoneFound
}

// =============================================================================
// Cache
// =============================================================================

struct CacheEntry {
    key: String,
    result: LoadRegistryResult,
}

fn cache() -> &'static Mutex<Option<CacheEntry>> {
    static CELL: std::sync::OnceLock<Mutex<Option<CacheEntry>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
}

/// Reset the cached registry. Intended for tests that need to exercise
/// discovery or simulate a different registry per case.
pub fn reset_registry_cache() {
    if let Ok(mut guard) = cache().lock() {
        *guard = None;
    }
}

fn cache_key(resolved: Option<&Path>) -> String {
    resolved.map_or_else(
        || "__none__".to_string(),
        |p| p.to_string_lossy().into_owned(),
    )
}

// =============================================================================
// Parsing
// =============================================================================

fn parse_registry(path: &Path) -> LoadRegistryResult {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => {
            return LoadRegistryResult {
                registry: None,
                source_path: Some(path.to_path_buf()),
                warnings: vec![format!(
                    "Failed to read registry at {}: {err}",
                    path.display()
                )],
            };
        }
    };

    parse_registry_str(&raw, Some(path.to_path_buf()), &path.display().to_string())
}

/// Shared parse + schema-validate path used by both on-disk and embedded
/// loads. `source_path` is `Some(path)` for disk loads and a synthetic
/// `<embedded>` sentinel for the compile-time fallback; `display_label`
/// is the string used in warning messages.
fn parse_registry_str(
    raw: &str,
    source_path: Option<PathBuf>,
    display_label: &str,
) -> LoadRegistryResult {
    let registry: CompiledRegistry = match serde_json::from_str(raw) {
        Ok(r) => r,
        Err(err) => {
            return LoadRegistryResult {
                registry: None,
                source_path,
                warnings: vec![format!(
                    "Registry at {display_label} failed schema validation: {err}"
                )],
            };
        }
    };

    if registry.schema_version != SUPPORTED_SCHEMA_VERSION {
        return LoadRegistryResult {
            registry: None,
            source_path,
            warnings: vec![format!(
                "Registry at {display_label} failed schema validation: expected schema_version={SUPPORTED_SCHEMA_VERSION}, got {}",
                registry.schema_version
            )],
        };
    }

    LoadRegistryResult {
        registry: Some(registry),
        source_path,
        warnings: Vec::new(),
    }
}

/// Parse the compile-time-embedded registry. The bytes are validated at
/// build time only in the sense that the file existed when `cargo build`
/// ran; serde parsing happens here at first load. A failure here means a
/// corrupt or schema-mismatched JSON shipped with the binary — surface
/// as a warning so operators see the cause rather than a silent empty
/// catalogue.
fn embedded_registry_result() -> LoadRegistryResult {
    parse_registry_str(
        EMBEDDED_REGISTRY,
        Some(PathBuf::from(EMBEDDED_REGISTRY_SOURCE_LABEL)),
        EMBEDDED_REGISTRY_SOURCE_LABEL,
    )
}

/// Load and validate the compiled registry.
///
/// Caches the result per resolved path. Production default is the
/// compile-time embedded catalogue. Pass `registry_path` or set
/// `ANVIL_REGISTRY_PATH` to load an unsigned on-disk override. There is
/// no cwd or executable-directory walk (POLFIT-008 / ADR-131).
#[must_use]
pub fn load_compiled_registry(opts: &LoadRegistryOptions) -> LoadRegistryResult {
    let resolved = resolve_registry_path(opts);
    let key = match &resolved {
        ResolvedPath::Found(path) => cache_key(Some(path)),
        ResolvedPath::OverrideMissing { source, value } => {
            format!("__override_missing__:{source}:{value}")
        }
        ResolvedPath::NoneFound => format!("__embedded__:{EMBEDDED_REGISTRY_SOURCE_LABEL}"),
    };

    if let Ok(guard) = cache().lock()
        && let Some(entry) = guard.as_ref()
        && entry.key == key
    {
        return entry.result.clone();
    }

    let result = match resolved {
        ResolvedPath::Found(path) => parse_registry(&path),
        ResolvedPath::OverrideMissing { source, value } => {
            // An explicit override was supplied but does not exist.
            // Warn the operator (this is a config bug) and still fall
            // back to the embedded catalogue so the scanner stays
            // useful — the previous behaviour was to silently return
            // an empty catalogue here.
            let mut embedded = embedded_registry_result();
            embedded.warnings.insert(
                0,
                format!(
                    "Configured {source} = {value} does not exist; falling back to embedded registry."
                ),
            );
            embedded
        }
        ResolvedPath::NoneFound => embedded_registry_result(),
    };

    if let Ok(mut guard) = cache().lock() {
        *guard = Some(CacheEntry {
            key,
            result: result.clone(),
        });
    }

    result
}

// =============================================================================
// Mapping compiled → scanner AntiPattern
// =============================================================================

fn map_category(anvil_category: &str) -> AntiPatternCategory {
    match anvil_category {
        "escape-hatch" => AntiPatternCategory::EscapeHatch,
        "error-handling" => AntiPatternCategory::ErrorHandling,
        "type-safety" => AntiPatternCategory::TypeSafety,
        "type-evasion" => AntiPatternCategory::TypeEvasion,
        "accountability" => AntiPatternCategory::Accountability,
        "deferred-debt" => AntiPatternCategory::DeferredDebt,
        "insecure-construction" => AntiPatternCategory::InsecureConstruction,
        "fragile-presentation" => AntiPatternCategory::FragilePresentation,
        _ => AntiPatternCategory::CodeQuality,
    }
}

/// Flag letters that translate 1:1 between Rust's `regex` crate and V8's
/// PCRE-ish engine. `i` is load-bearing (every RL-\* and DD-004 rule uses
/// it). `m` and `s` are supported for forward compatibility. Flags that
/// silently change match semantics in one engine but not the other — `U`
/// (swap greedy/lazy), `x` (verbose / ignore whitespace), `g`, `y`, `u` —
/// are deliberately NOT honoured here. If such a flag is present the
/// returned prefix is a deliberately-invalid inline group so
/// `Regex::new(&pattern.regex)` fails in `prepare_pattern` and
/// `registry_compile_diagnostics()` surfaces the rule. No silent-drop
/// path — adversarial-reviewer M-2.
fn inline_flag_prefix(flags: Option<&str>) -> String {
    let Some(flags) = flags else {
        return String::new();
    };
    let unsupported: String = flags
        .chars()
        .filter(|c| !matches!(c, 'i' | 'm' | 's'))
        .collect();
    if !unsupported.is_empty() {
        // Deliberately-invalid inline group: the `regex` crate will fail
        // to parse `(?Q)` and friends, producing a compile error that
        // `registry_compile_diagnostics()` picks up.
        return format!("(?ANVIL_UNSUPPORTED_FLAG_{unsupported})");
    }
    let supported: String = flags
        .chars()
        .filter(|c| matches!(c, 'i' | 'm' | 's'))
        .collect();
    if supported.is_empty() {
        String::new()
    } else {
        format!("(?{supported})")
    }
}

/// Convert a single compiled pattern into the scanner's `AntiPattern` shape.
///
/// Returns `None` when the detection is not regex-based — the current scanner
/// engine only understands regex detection. Family provenance (family /
/// `definition_ref` / `spectrum_position` / targets) is carried onto the
/// resulting `AntiPattern` so the scanner can attach it to emitted warnings.
///
/// `detection.flags` is honoured by prefixing the regex with an inline
/// group (e.g. `(?i)` for case-insensitive matching). The `AntiPattern.regex`
/// field stays a single string so the scanner's existing `Regex::new` path
/// keeps working unchanged.
#[must_use]
pub fn compiled_to_antipattern(cp: &CompiledPattern) -> Option<AntiPattern> {
    let regex = match &cp.detection {
        Detection::Regex { pattern, flags } => {
            let prefix = inline_flag_prefix(flags.as_deref());
            format!("{prefix}{pattern}")
        }
        Detection::Ast { .. } => return None,
    };

    Some(AntiPattern {
        id: cp.id.clone(),
        name: cp.title.clone(),
        category: map_category(&cp.category),
        severity: cp.severity,
        confidence: cp.confidence,
        regex,
        title: cp.title.clone(),
        explanation: cp.explanation.clone(),
        suggestion: cp.suggestion.clone(),
        nudge: Some(cp.nudge.clone()),
        file_extensions: cp.file_extensions.clone(),
        all_file_types: cp.file_extensions.is_none(),
        allowlist: cp.allowlist.clone(),
        threshold: None,
        enabled: cp.enabled,
        opt_in: cp.opt_in,
        family: Some(cp.family.clone()),
        definition_ref: Some(cp.definition_ref.clone()),
        spectrum_position: Some(cp.spectrum_position),
        targets: Some(cp.targets.clone()),
    })
}

/// Load the registry and return the mapped anti-patterns in the shape the
/// scanner expects. Returns `[]` if no registry is available. AST-detection
/// rules are skipped until the scanner grows AST support.
#[must_use]
pub fn load_registry_patterns(opts: &LoadRegistryOptions) -> Vec<AntiPattern> {
    let result = load_compiled_registry(opts);
    match result.registry {
        Some(reg) => patterns_from_registry(&reg),
        None => Vec::new(),
    }
}

/// Map every regex-backed pattern in an already-loaded registry.
///
/// Prefer this over calling `load_registry_patterns` again when the caller
/// already holds a `CompiledRegistry` (for example after a hard-error load
/// preflight). Re-resolving the path can race with a concurrent overwrite and
/// silently fall back to an empty catalogue.
#[must_use]
pub fn patterns_from_registry(registry: &CompiledRegistry) -> Vec<AntiPattern> {
    registry
        .patterns
        .iter()
        .filter_map(compiled_to_antipattern)
        .collect()
}

/// True when `id` is a compiled-registry rule id (`PY-008`, `RS-001`, …).
/// Includes AST-only rules that [`super::is_valid_pattern_id`] does not
/// see, because those still print from `anvil check`.
#[must_use]
pub fn is_registered_rule_id(id: &str) -> bool {
    let Some(registry) = load_compiled_registry(&LoadRegistryOptions::default()).registry else {
        return false;
    };
    registry
        .patterns
        .iter()
        .any(|pattern| pattern.id.eq_ignore_ascii_case(id))
}

/// Look up a single pattern by id in an already-loaded registry.
///
/// Returns `None` when the id is absent or the entry is not regex-backed
/// (AST-only detections are not yet mapped). Does not consult the process
/// catalogue cache.
#[must_use]
pub fn get_pattern_from_registry(registry: &CompiledRegistry, id: &str) -> Option<AntiPattern> {
    registry
        .patterns
        .iter()
        .find(|pattern| pattern.id == id)
        .and_then(compiled_to_antipattern)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn workspace_registry_path() -> PathBuf {
        // `CARGO_MANIFEST_DIR` = `<workspace>/crates/anvil-checks`.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join(REGISTRY_RELATIVE_PATH)
    }

    fn sample_compiled() -> CompiledPattern {
        CompiledPattern {
            id: "AP-001".to_string(),
            family: "guardrail-suppression".to_string(),
            title: "Broad eslint-disable".to_string(),
            version: 1,
            severity: WarningSeverity::Warning,
            confidence: Confidence::High,
            spectrum_position: 1,
            targets: vec!["source".to_string()],
            detection: Detection::Regex {
                pattern: "eslint-disable".to_string(),
                flags: None,
            },
            file_extensions: Some(vec![".ts".to_string(), ".js".to_string()]),
            allowlist: vec!["**/__tests__/**".to_string()],
            nudge: "Don't disable all rules.".to_string(),
            related: Vec::new(),
            enabled: true,
            opt_in: false,
            family_name: "Guardrail Suppression".to_string(),
            category: "escape-hatch".to_string(),
            explanation: "Blanket disables hide real bugs.".to_string(),
            suggestion: "Disable just the failing rule instead.".to_string(),
            definition_ref: "patterns/guardrail-suppression/definition.anvil".to_string(),
            tensions: Vec::new(),
            related_families: Vec::new(),
        }
    }

    #[test]
    fn loads_the_workspace_registry_when_explicit_path_is_provided() {
        reset_registry_cache();
        let path = workspace_registry_path();
        let result = load_compiled_registry(&LoadRegistryOptions {
            registry_path: Some(path),
        });
        assert!(
            result.registry.is_some(),
            "expected workspace registry to load: warnings={:?}",
            result.warnings
        );
        assert!(!result.registry.unwrap().patterns.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn missing_override_warns_and_falls_back_to_embedded() {
        // #1630: a configured override that points at a non-existent path
        // is a config bug — surface it via a warning — but the scanner
        // must still get a useful catalogue from the embedded fallback
        // rather than going silent. Previous behaviour returned an empty
        // catalogue, which made `CommitAntipatternEngine` look broken.
        reset_registry_cache();
        let result = load_compiled_registry(&LoadRegistryOptions {
            registry_path: Some(PathBuf::from("/nonexistent/registry.json")),
        });
        let registry = result
            .registry
            .as_ref()
            .expect("embedded fallback must load even when override is missing");
        assert!(
            !registry.patterns.is_empty(),
            "embedded fallback must carry a non-empty pattern set"
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("does not exist")),
            "missing override must surface a warning; got={:?}",
            result.warnings
        );
        // source_path reflects the embedded fallback, not the missing path.
        assert_eq!(
            result.source_path.as_deref(),
            Some(Path::new(EMBEDDED_REGISTRY_SOURCE_LABEL))
        );
    }

    #[test]
    fn default_load_uses_embedded_even_when_workspace_registry_exists() {
        // POLFIT-008: a clone that contains `patterns/compiled/registry.json`
        // (this workspace does) must not silently replace the compile-time
        // catalogue. Default resolution is embedded unless an explicit path
        // or ANVIL_REGISTRY_PATH is set.
        reset_registry_cache();
        assert!(
            workspace_registry_path().is_file(),
            "this test needs the workspace registry to exist as the silent-walk bait"
        );
        assert!(
            std::env::var_os("ANVIL_REGISTRY_PATH").is_none(),
            "ANVIL_REGISTRY_PATH is set; cannot assert implicit resolution"
        );
        let result = load_compiled_registry(&LoadRegistryOptions::default());
        assert_eq!(
            result.source_path.as_deref(),
            Some(Path::new(EMBEDDED_REGISTRY_SOURCE_LABEL)),
            "default load must use the embedded catalogue, not cwd/exe discovery; warnings={:?}",
            result.warnings
        );
        assert!(result.registry.is_some());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn returns_warning_when_file_is_malformed_json() {
        reset_registry_cache();
        let tmp = tempdir_for("malformed");
        let path = tmp.join("registry.json");
        std::fs::write(&path, "{ not json").unwrap();

        let result = load_compiled_registry(&LoadRegistryOptions {
            registry_path: Some(path.clone()),
        });
        assert!(result.registry.is_none());
        let warnings = result.warnings.join(" ");
        assert!(
            warnings.contains("schema validation") || warnings.contains("Failed to read"),
            "unexpected warnings: {warnings}"
        );

        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        reset_registry_cache();
        let tmp = tempdir_for("schema");
        let path = tmp.join("registry.json");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"schema_version": 99}}"#).unwrap();

        let result = load_compiled_registry(&LoadRegistryOptions {
            registry_path: Some(path.clone()),
        });
        assert!(result.registry.is_none());
        assert!(result.warnings.join(" ").contains("schema validation"));

        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn caches_result_for_the_same_path() {
        reset_registry_cache();
        let path = workspace_registry_path();
        let first = load_compiled_registry(&LoadRegistryOptions {
            registry_path: Some(path.clone()),
        });
        let second = load_compiled_registry(&LoadRegistryOptions {
            registry_path: Some(path),
        });
        // Compare source paths — cache returns the same content.
        assert_eq!(first.source_path, second.source_path);
    }

    #[test]
    fn maps_compiled_fields_onto_antipattern_shape() {
        let cp = sample_compiled();
        let ap = compiled_to_antipattern(&cp).expect("regex detection maps");
        assert_eq!(ap.id, "AP-001");
        assert_eq!(ap.name, "Broad eslint-disable");
        assert_eq!(ap.title, "Broad eslint-disable");
        assert_eq!(ap.severity, WarningSeverity::Warning);
        assert_eq!(ap.confidence, Confidence::High);
        assert_eq!(ap.regex, "eslint-disable");
        assert_eq!(ap.nudge.as_deref(), Some("Don't disable all rules."));
        assert_eq!(
            ap.file_extensions,
            Some(vec![".ts".to_string(), ".js".to_string()])
        );
        assert_eq!(ap.allowlist, vec!["**/__tests__/**".to_string()]);
        assert_eq!(ap.category, AntiPatternCategory::EscapeHatch);
    }

    #[test]
    fn carries_family_provenance_onto_antipattern() {
        let cp = sample_compiled();
        let ap = compiled_to_antipattern(&cp).expect("regex detection maps");
        assert_eq!(ap.family.as_deref(), Some("guardrail-suppression"));
        assert_eq!(
            ap.definition_ref.as_deref(),
            Some("patterns/guardrail-suppression/definition.anvil")
        );
        assert_eq!(ap.spectrum_position, Some(1));
        assert_eq!(ap.targets, Some(vec!["source".to_string()]));
    }

    #[test]
    fn registry_backed_antipattern_carries_family_provenance() {
        // Post-RSCAN-004 sanity check: every pattern in the scanner catalogue
        // now comes from the registry, so provenance must be populated.
        let pattern = crate::antipattern::patterns::get_pattern("AP-001").expect("AP-001 exists");
        assert!(pattern.family.is_some(), "AP-001 must carry family");
        assert!(pattern.definition_ref.is_some());
        assert!(pattern.spectrum_position.is_some());
        assert!(pattern.targets.is_some());
    }

    #[test]
    fn maps_family_categories_to_enum() {
        let mut cp = sample_compiled();

        cp.category = "type-evasion".to_string();
        assert_eq!(
            compiled_to_antipattern(&cp).unwrap().category,
            AntiPatternCategory::TypeEvasion
        );

        cp.category = "accountability".to_string();
        assert_eq!(
            compiled_to_antipattern(&cp).unwrap().category,
            AntiPatternCategory::Accountability
        );

        cp.category = "deferred-debt".to_string();
        assert_eq!(
            compiled_to_antipattern(&cp).unwrap().category,
            AntiPatternCategory::DeferredDebt
        );

        cp.category = "error-handling".to_string();
        assert_eq!(
            compiled_to_antipattern(&cp).unwrap().category,
            AntiPatternCategory::ErrorHandling
        );

        // INSEC-001: the new security category resolves to its own variant
        // rather than falling through to the `code-quality` default.
        cp.category = "insecure-construction".to_string();
        assert_eq!(
            compiled_to_antipattern(&cp).unwrap().category,
            AntiPatternCategory::InsecureConstruction
        );

        cp.category = "made-up".to_string();
        assert_eq!(
            compiled_to_antipattern(&cp).unwrap().category,
            AntiPatternCategory::CodeQuality
        );
    }

    #[test]
    fn regex_loader_skips_ast_detection_by_design() {
        // ADR-071 §3: AST-detection rules are owned by the `anvil-checks-ast`
        // gate-time scanner, not this regex loader. The regex path deliberately
        // skips them (returns `None`) so the registry stays the single source of
        // truth with each scanner consuming its own detection kind — not a
        // "not yet supported" gap. The AST scanner ships its own
        // registry-completeness guard so an `ast` rule with no predicate fails
        // loudly there rather than silently producing nothing here.
        let mut cp = sample_compiled();
        cp.detection = Detection::Ast {
            ast_query: "(unsafe_block) @target".to_string(),
        };
        assert!(compiled_to_antipattern(&cp).is_none());
    }

    #[test]
    fn is_registered_rule_id_includes_ast_only_ids() {
        // RS-* are AST-backed; they are still printed by `anvil check`
        // and must resolve as first-class rule ids even though the regex
        // catalogue skips them.
        assert!(is_registered_rule_id("RS-001"));
        assert!(is_registered_rule_id("PY-008"));
        assert!(is_registered_rule_id("AP-008"));
        assert!(is_registered_rule_id("WC-001"));
        assert!(!is_registered_rule_id("PY-999"));
        assert!(!is_registered_rule_id("lnt"));
    }

    #[test]
    fn honours_case_insensitive_flag_via_inline_group() {
        // SPG-001: the registry's `flags: "i"` field on RL-* / DD-004 rules
        // must be honoured by the Rust loader. Achieved by prefixing the
        // regex string with the inline group `(?i)` so the scanner's
        // `Regex::new(&ap.regex)` path matches case-insensitively.
        let mut cp = sample_compiled();
        cp.detection = Detection::Regex {
            pattern: r"\bpre-existing\b".to_string(),
            flags: Some("i".to_string()),
        };
        let ap = compiled_to_antipattern(&cp).expect("regex detection maps");
        assert!(
            ap.regex.starts_with("(?i)"),
            "case-insensitive flag should be inlined; got regex={}",
            ap.regex
        );

        // End-to-end: compile and match against case-varied content.
        let compiled = regex::Regex::new(&ap.regex).expect("inlined regex must compile");
        assert!(compiled.is_match("Pre-Existing failure is noted."));
        assert!(compiled.is_match("PRE-EXISTING failure is noted."));
        assert!(compiled.is_match("pre-existing failure is noted."));
    }

    #[test]
    fn preserves_case_sensitivity_when_flags_absent() {
        // No flags means the regex must remain case-sensitive — no silent
        // broadening of match semantics.
        let mut cp = sample_compiled();
        cp.detection = Detection::Regex {
            pattern: r"\bfoo\b".to_string(),
            flags: None,
        };
        let ap = compiled_to_antipattern(&cp).expect("regex detection maps");
        assert!(!ap.regex.starts_with("(?i)"));
        let compiled = regex::Regex::new(&ap.regex).expect("regex compiles");
        assert!(compiled.is_match("foo"));
        assert!(!compiled.is_match("FOO"));
    }

    #[test]
    fn unrecognised_flags_force_a_compile_error() {
        // Adversarial-reviewer M-2: unrecognised flag letters must surface
        // via `registry_compile_diagnostics()` rather than drop silently.
        // The loader emits an intentionally-invalid inline group so
        // `Regex::new` fails and the scanner records a compile_error.
        let mut cp = sample_compiled();
        cp.detection = Detection::Regex {
            pattern: r"\bfoo\b".to_string(),
            flags: Some("x".to_string()),
        };
        let ap = compiled_to_antipattern(&cp).expect("regex detection maps");
        assert!(
            ap.regex.contains("ANVIL_UNSUPPORTED_FLAG_"),
            "expected sentinel prefix; got {}",
            ap.regex
        );
        assert!(
            regex::Regex::new(&ap.regex).is_err(),
            "the sentinel must fail to compile",
        );
    }

    #[test]
    fn load_registry_patterns_returns_mapped_list() {
        reset_registry_cache();
        let patterns = load_registry_patterns(&LoadRegistryOptions {
            registry_path: Some(workspace_registry_path()),
        });
        assert!(!patterns.is_empty());
        let ap001 = patterns.iter().find(|p| p.id == "AP-001");
        assert!(ap001.is_some(), "AP-001 missing from registry-backed list");
    }

    #[test]
    fn load_registry_patterns_falls_back_to_embedded_on_missing_file() {
        // #1630: missing on-disk registry now falls back to the embedded
        // catalogue rather than producing an empty pattern list.
        reset_registry_cache();
        let patterns = load_registry_patterns(&LoadRegistryOptions {
            registry_path: Some(PathBuf::from("/nonexistent/registry.json")),
        });
        assert!(
            !patterns.is_empty(),
            "embedded fallback must populate the scanner catalogue"
        );
        // Sanity check: a known pattern from the workspace registry is
        // present, proving the embedded snapshot tracks the source file.
        assert!(
            patterns.iter().any(|p| p.id == "AP-001"),
            "embedded pattern set must include AP-001"
        );
    }

    #[test]
    fn embedded_registry_parses_and_is_non_empty() {
        // #1630: the compile-time-embedded registry must always be
        // loadable. A parse failure here means the build embedded a
        // corrupt / schema-mismatched JSON, which would silently degrade
        // every stock install. Pin the contract.
        let result = embedded_registry_result();
        assert!(
            result.registry.is_some(),
            "embedded registry must parse; warnings={:?}",
            result.warnings
        );
        assert!(result.warnings.is_empty());
        let registry = result.registry.unwrap();
        assert_eq!(registry.schema_version, SUPPORTED_SCHEMA_VERSION);
        assert!(!registry.patterns.is_empty());
    }

    // Default resolution (no explicit path, no ANVIL_REGISTRY_PATH) is
    // covered by `default_load_uses_embedded_even_when_workspace_registry_exists`.
    // `ANVIL_REGISTRY_PATH` uses the same OverrideMissing / Found arms as
    // `registry_path`; those arms are tested via explicit `LoadRegistryOptions`.
    // Process-environment mutation is forbidden here (edition 2024 `set_var`
    // is unsafe; this crate forbids `unsafe_code`).

    #[test]
    fn patterns_from_registry_maps_loaded_instance_without_reload() {
        // Binding preflight holds a CompiledRegistry; helpers must map that
        // instance rather than re-resolving (TOCTOU empty-catalogue risk).
        let mut registry = CompiledRegistry {
            schema_version: SUPPORTED_SCHEMA_VERSION,
            compiled_at: "test".to_string(),
            source_root: "test".to_string(),
            patterns: vec![sample_compiled()],
            prefixes: BTreeMap::new(),
            families: Vec::new(),
        };
        let mapped = patterns_from_registry(&registry);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].id, "AP-001");

        registry.patterns.clear();
        assert!(
            patterns_from_registry(&registry).is_empty(),
            "empty supplied registry must map to empty catalogue"
        );
    }

    #[test]
    fn get_pattern_from_registry_looks_up_supplied_instance_only() {
        let registry = CompiledRegistry {
            schema_version: SUPPORTED_SCHEMA_VERSION,
            compiled_at: "test".to_string(),
            source_root: "test".to_string(),
            patterns: vec![sample_compiled()],
            prefixes: BTreeMap::new(),
            families: Vec::new(),
        };
        let found = get_pattern_from_registry(&registry, "AP-001")
            .expect("AP-001 must resolve from the supplied registry");
        assert_eq!(found.id, "AP-001");
        assert!(
            get_pattern_from_registry(&registry, "DEFINITELY-MISSING").is_none(),
            "unknown id must be None even when the process catalogue is non-empty"
        );
    }

    fn tempdir_for(suffix: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "anvil-registry-{suffix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("create tempdir");
        dir
    }
}
