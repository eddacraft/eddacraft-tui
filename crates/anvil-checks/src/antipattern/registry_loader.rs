//! Registry loader — bridges the compiled `.anvil` pattern registry into the
//! Rust scanner's in-memory `AntiPattern` shape.
//!
//! The compiled registry (`patterns/compiled/registry.json`) is produced by
//! the TypeScript `patterns:compile` script. At runtime the scanner needs an
//! `AntiPattern` list in the shape `patterns.rs` has always exposed, so this
//! module reads the JSON, validates it with `serde_json`, maps each
//! `CompiledPattern` to an `AntiPattern`, and caches the result per resolved
//! path.
//!
//! Resolution order for the registry file mirrors the TypeScript loader:
//!   1. `opts.registry_path` explicit override (tests).
//!   2. `ANVIL_REGISTRY_PATH` env var.
//!   3. Upward walk from the current working directory.
//!   4. Upward walk from the executable's directory (handles installed
//!      binaries run outside the monorepo).
//!
//! If no registry is found, the loader returns an empty catalogue plus a
//! warning diagnostic. This keeps `anvil check` working even when the
//! compiled registry is missing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::antipattern::types::{AntiPattern, AntiPatternCategory, Confidence, WarningSeverity};

const REGISTRY_RELATIVE_PATH: &str = "patterns/compiled/registry.json";
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

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
    /// Absolute path to a `registry.json` — overrides discovery.
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

fn walk_upwards(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(REGISTRY_RELATIVE_PATH);
        if candidate.exists() {
            return Some(candidate);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent.to_path_buf(),
            _ => return None,
        }
    }
}

fn exe_start_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
}

fn resolve_registry_path(opts: &LoadRegistryOptions) -> Option<PathBuf> {
    if let Some(p) = opts.registry_path.as_ref() {
        return if p.exists() { Some(p.clone()) } else { None };
    }

    if let Ok(env_path) = std::env::var("ANVIL_REGISTRY_PATH") {
        let p = PathBuf::from(&env_path);
        return if p.exists() { Some(p) } else { None };
    }

    if let Ok(cwd) = std::env::current_dir()
        && let Some(found) = walk_upwards(&cwd)
    {
        return Some(found);
    }

    exe_start_dir().and_then(|d| walk_upwards(&d))
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

    let registry: CompiledRegistry = match serde_json::from_str(&raw) {
        Ok(r) => r,
        Err(err) => {
            return LoadRegistryResult {
                registry: None,
                source_path: Some(path.to_path_buf()),
                warnings: vec![format!(
                    "Registry at {} failed schema validation: {err}",
                    path.display()
                )],
            };
        }
    };

    if registry.schema_version != SUPPORTED_SCHEMA_VERSION {
        return LoadRegistryResult {
            registry: None,
            source_path: Some(path.to_path_buf()),
            warnings: vec![format!(
                "Registry at {} failed schema validation: expected schema_version={SUPPORTED_SCHEMA_VERSION}, got {}",
                path.display(),
                registry.schema_version
            )],
        };
    }

    LoadRegistryResult {
        registry: Some(registry),
        source_path: Some(path.to_path_buf()),
        warnings: Vec::new(),
    }
}

/// Load and validate the compiled registry.
///
/// Caches the result per resolved path. Pass `registry_path` in tests to
/// target a fixture; omit in production to let discovery find the workspace
/// registry.
#[must_use]
pub fn load_compiled_registry(opts: &LoadRegistryOptions) -> LoadRegistryResult {
    let resolved = resolve_registry_path(opts);
    let key = cache_key(resolved.as_deref());

    if let Ok(guard) = cache().lock()
        && let Some(entry) = guard.as_ref()
        && entry.key == key
    {
        return entry.result.clone();
    }

    let result = match resolved {
        Some(path) => parse_registry(&path),
        None => LoadRegistryResult {
            registry: None,
            source_path: None,
            warnings: vec![
                "Compiled pattern registry not found; scanner catalogue will be empty.".to_string(),
            ],
        },
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
        Some(reg) => reg
            .patterns
            .iter()
            .filter_map(compiled_to_antipattern)
            .collect(),
        None => Vec::new(),
    }
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
    fn returns_empty_catalogue_when_file_does_not_exist() {
        reset_registry_cache();
        let result = load_compiled_registry(&LoadRegistryOptions {
            registry_path: Some(PathBuf::from("/nonexistent/registry.json")),
        });
        assert!(result.registry.is_none());
        assert!(!result.warnings.is_empty());
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

        cp.category = "made-up".to_string();
        assert_eq!(
            compiled_to_antipattern(&cp).unwrap().category,
            AntiPatternCategory::CodeQuality
        );
    }

    #[test]
    fn skips_ast_detection_until_scanner_supports_it() {
        let mut cp = sample_compiled();
        cp.detection = Detection::Ast {
            ast_query: "MemberExpression".to_string(),
        };
        assert!(compiled_to_antipattern(&cp).is_none());
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
    fn load_registry_patterns_returns_empty_on_missing_file() {
        reset_registry_cache();
        let patterns = load_registry_patterns(&LoadRegistryOptions {
            registry_path: Some(PathBuf::from("/nonexistent/registry.json")),
        });
        assert!(patterns.is_empty());
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
