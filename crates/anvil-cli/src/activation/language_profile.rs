//! Repo language profile (LAUNCH-015).
//!
//! Walks a working tree, classifies each detected language by extension
//! against a single registry, and produces a structured profile that
//! activation copy can render. This is the single source of truth for
//! "what languages does anvil claim coverage for in this release?" —
//! surfaces never duplicate the registry inline.
//!
//! The profile feeds three places:
//! 1. `ActivationDiagnostic.all_languages_unsupported` (PR 2 stub) so
//!    the protection-state mapping can return `Unsupported` honestly.
//! 2. Activation render (`render_human` / `render_json`) so users see
//!    a per-language breakdown next to the protection state.
//! 3. The antipattern scanner's skip ledger (LAUNCH-016) so files
//!    belonging to `Unsupported` languages are excluded from
//!    language-specific antipattern checks while still being scanned
//!    for cross-language concerns (secrets).
//!
//! Detection uses file extensions only. Vendored / generated paths are
//! filtered by an inline denylist matching `anvil-checks::filter`'s
//! conventions.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Coverage tier for a single language. Surfaces must respect the
/// tier when rendering protection claims — `Unsupported` files are
/// not eligible for `protecting` / `watching` claims (architecture /
/// antipattern checks won't fire on them); `Partial` languages get
/// some checks today and more as language packs ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageTier {
    /// anvil ships first-class checks for this language in the
    /// current release.
    Supported,
    /// anvil ships some checks for this language; full coverage is
    /// in flight (e.g. SQL pending SURFSQL Phase 1).
    Partial,
    /// anvil does not yet ship language-specific checks for this
    /// language. Cross-language checks (secrets) still apply.
    Unsupported,
}

impl CoverageTier {
    pub fn label(self) -> &'static str {
        match self {
            CoverageTier::Supported => "supported",
            CoverageTier::Partial => "partial",
            CoverageTier::Unsupported => "unsupported",
        }
    }
}

/// A row in the [`LANGUAGE_REGISTRY`] — names a language, the
/// extensions anvil treats as belonging to it, the coverage tier in
/// the current release, and the human-readable basis for the tier so
/// surfaces can answer "why is X partial?" without reaching into the
/// docs.
#[derive(Debug, Clone, Copy)]
pub struct LanguageEntry {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub coverage_tier: CoverageTier,
    pub basis: &'static str,
}

/// Single-source-of-truth registry. Adding language coverage means
/// editing this table — no other file should hand-code these tiers.
/// Order is "most-supported first" so iteration is stable.
///
/// **Anchoring (2026-05-04):** TS/JS supported via the antipattern
/// check defaults; SQL partial pending SURFSQL Phase 1 (RELEASE-PLAN
/// A5); Markdown partial pending MDGOV.
///
/// **Rust → supported (2026-06, RSTLAN-003/-004/-005/-006):** Rust
/// ships the AST-aware antipattern catalogue, default `.rs` scan-set
/// inclusion, and symbol/import + entry-point + layer/boundary
/// analysis. The tier was lifted from `Unsupported` so `anvil start` /
/// `anvil status` stop reporting a Rust-only repo as `unsupported`
/// while the engine fully analyses it.
///
/// **Python → supported (2026-06-30, CIB-123):** PYLAN shipped the
/// `python-reliability` antipattern catalogue, default `.py` scan-set
/// inclusion, and symbol/import + boundary analysis (T3) — the same bar
/// that lifted Rust. The tier was lifted from `Unsupported`; the stale
/// "PYLAN parked" basis is retired.
///
/// **Tier ≠ parser capability (CIB-123):** the kernel parses the LANGTAIL
/// /LTW2 tail (Go, Java, Kotlin, C#, C/C++, Dart, Zig, WebAssembly-text)
/// at T1 — symbol/import extraction + symbol-graph inclusion — but ships
/// no per-language anti-pattern catalogue for them, so they stay
/// `Unsupported` here (a tier reflects shipped language-specific
/// governance, not whether the parser can read the file). They are listed
/// explicitly so they are recognised, not counted as unclassified.
pub const LANGUAGE_REGISTRY: &[LanguageEntry] = &[
    LanguageEntry {
        name: "TypeScript",
        extensions: &[".ts", ".tsx", ".mts", ".cts"],
        coverage_tier: CoverageTier::Supported,
        basis: "antipattern + secret checks ship",
    },
    LanguageEntry {
        name: "JavaScript",
        extensions: &[".js", ".jsx", ".mjs", ".cjs"],
        coverage_tier: CoverageTier::Supported,
        basis: "antipattern + secret checks ship",
    },
    LanguageEntry {
        name: "Web (HTML/CSS)",
        extensions: &[".html", ".htm", ".css", ".scss", ".less"],
        coverage_tier: CoverageTier::Supported,
        basis: "antipattern checks ship",
    },
    LanguageEntry {
        name: "Rust",
        extensions: &[".rs"],
        coverage_tier: CoverageTier::Supported,
        basis: "antipattern + secret checks ship",
    },
    LanguageEntry {
        name: "SQL",
        extensions: &[".sql"],
        coverage_tier: CoverageTier::Partial,
        basis: "secret checks ship; structural governance not yet shipped",
    },
    LanguageEntry {
        name: "Markdown",
        extensions: &[".md", ".mdx"],
        coverage_tier: CoverageTier::Partial,
        basis: "secret checks ship; structural governance not yet shipped",
    },
    LanguageEntry {
        name: "Python",
        extensions: &[".py", ".pyw"],
        coverage_tier: CoverageTier::Supported,
        basis: "antipattern + secret checks ship",
    },
    // Tail languages (LANGTAIL + LTW2) are parsed at T1 — symbol/import
    // extraction + symbol-graph inclusion — but ship no language-specific
    // anti-pattern catalogue yet, so they stay Unsupported per the tier
    // semantics (cross-language secret checks still apply). Listed explicitly so
    // they are recognised rather than falling into `unclassified_files_seen`.
    LanguageEntry {
        name: "Go",
        extensions: &[".go"],
        coverage_tier: CoverageTier::Unsupported,
        basis: "parsed; no language-specific anti-pattern catalogue yet",
    },
    LanguageEntry {
        name: "Java/Kotlin",
        extensions: &[".java", ".kt", ".kts"],
        coverage_tier: CoverageTier::Unsupported,
        basis: "parsed; no language-specific anti-pattern catalogue yet",
    },
    LanguageEntry {
        name: "C#",
        extensions: &[".cs"],
        coverage_tier: CoverageTier::Unsupported,
        basis: "parsed; no language-specific anti-pattern catalogue yet",
    },
    LanguageEntry {
        name: "C/C++",
        extensions: &[
            ".c", ".cc", ".cpp", ".cxx", ".c++", ".h", ".hh", ".hpp", ".hxx", ".h++",
        ],
        coverage_tier: CoverageTier::Unsupported,
        basis: "parsed; no language-specific anti-pattern catalogue yet",
    },
    LanguageEntry {
        name: "Dart",
        extensions: &[".dart"],
        coverage_tier: CoverageTier::Unsupported,
        basis: "parsed; no language-specific anti-pattern catalogue yet",
    },
    LanguageEntry {
        name: "Zig",
        extensions: &[".zig", ".zon"],
        coverage_tier: CoverageTier::Unsupported,
        basis: "parsed; no language-specific anti-pattern catalogue yet",
    },
    LanguageEntry {
        name: "WebAssembly text",
        extensions: &[".wat", ".wast"],
        coverage_tier: CoverageTier::Unsupported,
        basis: "parsed; no language-specific anti-pattern catalogue yet",
    },
    LanguageEntry {
        name: "Ruby",
        extensions: &[".rb"],
        coverage_tier: CoverageTier::Unsupported,
        basis: "not parsed; no language pack",
    },
];

/// Result row exposed in JSON output and used by activation copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageProfileEntry {
    pub name: String,
    pub files_seen: usize,
    pub coverage_tier: CoverageTier,
    pub basis: String,
}

/// Repo-level profile of detected languages. Iteration order matches
/// the order in [`LANGUAGE_REGISTRY`] so renders are stable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoLanguageProfile {
    pub entries: Vec<LanguageProfileEntry>,
    /// Files whose extension is not in any registry entry. Counted
    /// only — the path is dropped to avoid leaking working-copy
    /// information into the diagnostic.
    pub unclassified_files_seen: usize,
}

impl RepoLanguageProfile {
    /// True when every detected language carries the `Unsupported`
    /// coverage tier. Drives `ActivationDiagnostic.all_languages_unsupported`.
    /// An empty profile (no detected files) is NOT all-unsupported —
    /// the user has no files to classify yet, so falling back to
    /// `NeedsAction` rather than `Unsupported` is the honest call.
    pub fn all_unsupported(&self) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        self.entries
            .iter()
            .all(|e| matches!(e.coverage_tier, CoverageTier::Unsupported))
    }

    /// True when at least one supported / partial language is
    /// present. Surfaces use this to decide whether to claim any
    /// language-specific protection at all.
    #[allow(dead_code)] // contract surface for downstream PRs
    pub fn has_covered_language(&self) -> bool {
        self.entries
            .iter()
            .any(|e| !matches!(e.coverage_tier, CoverageTier::Unsupported))
    }

    /// Set of extensions belonging to `Unsupported` languages
    /// detected in this repo. Returned with the leading dot so
    /// callers can match against `Path::extension`-shaped strings.
    pub fn unsupported_extensions(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for entry in &self.entries {
            if !matches!(entry.coverage_tier, CoverageTier::Unsupported) {
                continue;
            }
            for reg in LANGUAGE_REGISTRY {
                if reg.name == entry.name {
                    for ext in reg.extensions {
                        out.push((*ext).to_string());
                    }
                }
            }
        }
        out
    }
}

/// Skip ledger surfaced in run summaries when files belonging to
/// `Unsupported` languages are excluded from language-specific
/// antipattern checks. Cross-language checks (secrets) still run on
/// the skipped files — the ledger names the skip honestly so the
/// behaviour is visible to the user, not silent.
///
/// The map's key is the registry language name (e.g. `"Python"`);
/// the value is the count of files skipped for that language. The
/// `reason` is fixed at `"unsupported"` for v1; future tiers (e.g.
/// "partial-no-pattern-pack") can extend this enum.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageSkipLedger {
    pub by_language: BTreeMap<String, usize>,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SkipReason {
    #[default]
    Unsupported,
}

impl SkipReason {
    pub fn label(self) -> &'static str {
        match self {
            SkipReason::Unsupported => "unsupported",
        }
    }
}

impl LanguageSkipLedger {
    #[allow(dead_code)] // contract surface for downstream PRs
    pub fn total(&self) -> usize {
        self.by_language.values().sum()
    }

    pub fn is_empty(&self) -> bool {
        self.by_language.is_empty()
    }
}

/// Partition `files` into the subset scannable for language-specific
/// antipattern checks and the skipped subset, based on `profile`.
///
/// Files belonging to `Unsupported` languages in the profile are
/// dropped from the scannable list and tallied in the
/// [`LanguageSkipLedger`]. Files of `Supported` / `Partial`
/// languages, and files whose extension is not in the registry, are
/// retained — antipattern's existing extension allowlist
/// (`AntipatternCheckConfig::default().extensions`) is the final
/// gate on what actually gets scanned, so this function only narrows
/// further when the profile says a language is out of scope. That
/// matches the LAUNCH-016 acceptance: the existing allowlist is the
/// fallback default, the profile is an override.
///
/// Cross-language checks (secrets, env-template, etc.) must NOT use
/// this partition — they run on all files. Callers should call this
/// only before invoking language-specific antipattern checks.
///
/// **Adoption status:** PR 5 lands this helper as the LAUNCH-016
/// contract; PR 1 (LAUNCH-006) and follow-up scan/watch refactors
/// adopt it at the call sites that build their own candidate file
/// lists. The `run_post_init_analysis` path in
/// `services::sample_analyser` records the skip ledger directly
/// from the language profile (without invoking this partition,
/// since its candidate list is already extension-allowlisted) so
/// LAUNCH-016 is honestly surfaced for the activation flow today.
#[allow(dead_code)] // contract surface for downstream PRs
#[must_use]
pub fn partition_for_language_specific_checks<'a>(
    files: &[&'a str],
    profile: &RepoLanguageProfile,
) -> (Vec<&'a str>, LanguageSkipLedger) {
    let unsupported_exts = profile.unsupported_extensions();
    if unsupported_exts.is_empty() {
        return (files.to_vec(), LanguageSkipLedger::default());
    }

    let mut scannable: Vec<&'a str> = Vec::with_capacity(files.len());
    let mut by_language: BTreeMap<String, usize> = BTreeMap::new();
    for path in files {
        let Some(raw_ext) = Path::new(path).extension().and_then(|s| s.to_str()) else {
            scannable.push(*path);
            continue;
        };
        let ext = format!(".{}", raw_ext.to_ascii_lowercase());
        if unsupported_exts
            .iter()
            .any(|u| u.eq_ignore_ascii_case(&ext))
        {
            if let Some(language) = classify_extension(&ext) {
                *by_language.entry(language.to_string()).or_insert(0) += 1;
            }
            continue;
        }
        scannable.push(*path);
    }

    (
        scannable,
        LanguageSkipLedger {
            by_language,
            reason: SkipReason::Unsupported,
        },
    )
}

/// Profile the languages of files at and beneath `root`.
///
/// Walks the tree in a single pass, classifying each file's
/// extension against [`LANGUAGE_REGISTRY`] and accumulating counts.
/// The walker uses the same denylist conventions as `ScanFilter` —
/// `node_modules`, `target`, `.git`, etc. are skipped. Errors during
/// the walk are silently elided; this is a best-effort honesty probe,
/// not a strict scan.
pub fn profile_repo(root: &Path) -> RepoLanguageProfile {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut unclassified = 0_usize;

    let walker = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_excluded_directory(entry.path()));

    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext_with_dot) = extension_with_dot(path) else {
            unclassified += 1;
            continue;
        };
        if let Some(language) = classify_extension(&ext_with_dot) {
            *counts.entry(language).or_insert(0) += 1;
        } else {
            unclassified += 1;
        }
    }

    let entries: Vec<LanguageProfileEntry> = LANGUAGE_REGISTRY
        .iter()
        .filter_map(|reg| {
            let count = counts.get(reg.name).copied().unwrap_or(0);
            if count == 0 {
                None
            } else {
                Some(LanguageProfileEntry {
                    name: reg.name.to_string(),
                    files_seen: count,
                    coverage_tier: reg.coverage_tier,
                    basis: reg.basis.to_string(),
                })
            }
        })
        .collect();

    RepoLanguageProfile {
        entries,
        unclassified_files_seen: unclassified,
    }
}

/// Return the lowercase file extension prefixed by `.` if the path
/// has one — e.g. `Foo.TS` returns `Some(".ts")`. Files without an
/// extension return `None`.
pub fn extension_with_dot(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(format!(".{ext}"))
}

/// Look up a registry entry by extension. Returns the registry
/// `name` slice so callers can use it as a stable key.
pub fn classify_extension(ext_with_dot: &str) -> Option<&'static str> {
    let lower = ext_with_dot.to_ascii_lowercase();
    for entry in LANGUAGE_REGISTRY {
        if entry
            .extensions
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&lower))
        {
            return Some(entry.name);
        }
    }
    None
}

/// True when a path has a directory component matching one of the
/// well-known vendored / generated paths the activation walk skips.
///
/// Drawn from two sources in
/// `crates/anvil-checks/src/filter.rs`, plus one intentional addition:
///
/// - `DEFAULT_DIR_EXCLUDES` — fixtures / VCS / dependency caches /
///   build trees the scanner always skips.
/// - `BUILD_ARTEFACT_DIRS` — framework-specific generated trees the
///   scanner doesn't always denylist (a user may want to scan `dist/`
///   for secrets), but the activation language profile must not count
///   generated files as first-party source. Without these, a repo
///   using Angular or `SvelteKit` gets `.angular/` / `.svelte-kit/`
///   generated TypeScript counted, which can flip the protection
///   state away from `unsupported` incorrectly.
/// - `.anvil` — intentional activation-only addition (the scanner
///   doesn't denylist it because secret-scan call sites can target
///   it explicitly, but the language profile should never count
///   anvil's own state as user source).
///
/// Keep the first two sections in sync with `anvil-checks::filter`
/// — additions there should land here too. The `.anvil` entry is
/// activation-specific and stays.
fn is_excluded_directory(path: &Path) -> bool {
    const EXCLUDED: &[&str] = &[
        // DEFAULT_DIR_EXCLUDES — sync with anvil-checks::filter.
        "__fixtures__",
        "__mocks__",
        "__tests__",
        "test-data",
        "fixtures",
        "node_modules",
        "target",
        ".git",
        // Activation-only addition (see doc above).
        ".anvil",
        // BUILD_ARTEFACT_DIRS — sync with anvil-checks::filter.
        "dist",
        "build",
        "out",
        "coverage",
        ".next",
        ".nuxt",
        ".nx",
        ".turbo",
        ".cache",
        ".angular",
        ".svelte-kit",
    ];
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    EXCLUDED.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn registry_extensions_are_unique_and_lowercase() {
        let mut seen: std::collections::BTreeSet<&'static str> = std::collections::BTreeSet::new();
        for entry in LANGUAGE_REGISTRY {
            for ext in entry.extensions {
                assert!(
                    ext.starts_with('.'),
                    "extension `{ext}` (in {}) must start with `.`",
                    entry.name
                );
                assert_eq!(
                    *ext,
                    ext.to_ascii_lowercase(),
                    "extension `{ext}` (in {}) must be lowercase",
                    entry.name
                );
                assert!(
                    seen.insert(*ext),
                    "extension `{ext}` (in {}) is duplicated across languages",
                    entry.name
                );
            }
        }
    }

    #[test]
    fn registry_basis_strings_do_not_leak_internal_module_ids() {
        // `basis` is rendered to end users by `anvil start` / `anvil status`.
        // It must describe what ships today in plain language and must not
        // name internal APS module / anchor IDs (e.g. SURFSQL, MDGOV, PYLAN,
        // RSTLAN). This guard flags any token shaped like `[A-Z][A-Z0-9]{3,}`
        // — the conventional APS-ID shape. If a future basis legitimately
        // needs a 4+ char uppercase acronym (e.g. "JSON", "YAML"), extend
        // ALLOWED_ACRONYMS rather than weakening the check.
        const ALLOWED_ACRONYMS: &[&str] = &[];

        fn looks_like_module_id(word: &str) -> bool {
            word.len() >= 4
                && word.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && word
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        }

        for entry in LANGUAGE_REGISTRY {
            for word in entry.basis.split(|c: char| !c.is_ascii_alphanumeric()) {
                if word.is_empty() || ALLOWED_ACRONYMS.contains(&word) {
                    continue;
                }
                assert!(
                    !looks_like_module_id(word),
                    "Language `{}` basis `{}` contains uppercase token `{}` that looks like an internal APS module / anchor ID. User-facing copy must not leak internal codes. If `{}` is a legitimate acronym, add it to ALLOWED_ACRONYMS.",
                    entry.name,
                    entry.basis,
                    word,
                    word,
                );
            }
        }
    }

    #[test]
    fn classify_extension_handles_case_and_dots() {
        assert_eq!(classify_extension(".ts"), Some("TypeScript"));
        assert_eq!(classify_extension(".TS"), Some("TypeScript"));
        assert_eq!(classify_extension(".tsx"), Some("TypeScript"));
        assert_eq!(classify_extension(".py"), Some("Python"));
        assert_eq!(classify_extension(".rs"), Some("Rust"));
        assert_eq!(classify_extension(".unknown"), None);
        assert_eq!(classify_extension(""), None);
    }

    #[test]
    fn empty_repo_profile_is_empty() {
        let dir = TempDir::new().unwrap();
        let profile = profile_repo(dir.path());
        assert!(profile.entries.is_empty());
        assert_eq!(profile.unclassified_files_seen, 0);
        assert!(!profile.all_unsupported(), "empty repo is not unsupported");
        assert!(!profile.has_covered_language());
    }

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, "").unwrap();
    }

    #[test]
    fn ts_only_repo_profile_is_supported() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/a.ts");
        touch(dir.path(), "src/b.tsx");
        let profile = profile_repo(dir.path());
        assert_eq!(profile.entries.len(), 1);
        assert_eq!(profile.entries[0].name, "TypeScript");
        assert_eq!(profile.entries[0].files_seen, 2);
        assert_eq!(profile.entries[0].coverage_tier, CoverageTier::Supported);
        assert!(!profile.all_unsupported());
        assert!(profile.has_covered_language());
    }

    #[test]
    fn tail_only_repo_profile_is_unsupported() {
        // A tail T1 language (Go) is parsed but ships no language-specific
        // catalogue, so it is the `Unsupported` tier (CIB-123). Python is no
        // longer the example here — it became `Supported` once PYLAN shipped.
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "app.go");
        touch(dir.path(), "lib/util.go");
        let profile = profile_repo(dir.path());
        assert_eq!(profile.entries.len(), 1);
        assert_eq!(profile.entries[0].name, "Go");
        assert_eq!(profile.entries[0].coverage_tier, CoverageTier::Unsupported);
        assert!(profile.all_unsupported());
        assert!(!profile.has_covered_language());
    }

    #[test]
    fn python_only_repo_profile_is_supported() {
        // CIB-123: PYLAN lifted Python to the `Supported` tier (same bar as
        // Rust), so a Python-only repo is covered, not all-unsupported.
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "app.py");
        touch(dir.path(), "lib/util.py");
        let profile = profile_repo(dir.path());
        assert_eq!(profile.entries.len(), 1);
        assert_eq!(profile.entries[0].name, "Python");
        assert_eq!(profile.entries[0].coverage_tier, CoverageTier::Supported);
        assert!(!profile.all_unsupported());
        assert!(profile.has_covered_language());
    }

    #[test]
    fn mixed_repo_profile_aggregates_counts() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/a.ts");
        touch(dir.path(), "src/b.ts");
        touch(dir.path(), "scripts/util.py");
        touch(dir.path(), "schema.sql");
        touch(dir.path(), "README.md");
        touch(dir.path(), "Makefile"); // unclassified
        let profile = profile_repo(dir.path());
        let by_name: std::collections::BTreeMap<_, _> = profile
            .entries
            .iter()
            .map(|e| (e.name.as_str(), e))
            .collect();
        assert_eq!(by_name["TypeScript"].files_seen, 2);
        assert_eq!(by_name["Python"].files_seen, 1);
        assert_eq!(by_name["SQL"].files_seen, 1);
        assert_eq!(by_name["Markdown"].files_seen, 1);
        assert_eq!(profile.unclassified_files_seen, 1);
        assert!(profile.has_covered_language());
        assert!(!profile.all_unsupported());
    }

    #[test]
    fn vendored_dirs_are_excluded() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/a.ts");
        touch(dir.path(), "node_modules/dep/index.ts");
        touch(dir.path(), "target/debug/foo.rs");
        touch(dir.path(), ".git/HEAD");
        let profile = profile_repo(dir.path());
        // Only `src/a.ts` should be counted.
        assert_eq!(profile.entries.len(), 1);
        assert_eq!(profile.entries[0].name, "TypeScript");
        assert_eq!(profile.entries[0].files_seen, 1);
    }

    #[test]
    fn framework_build_artefact_dirs_are_excluded() {
        // Round-1 follow-up review (PR #1268): `.angular/` and
        // `.svelte-kit/` are recognised build-artefact dirs in
        // `anvil-checks::filter::BUILD_ARTEFACT_DIRS` but were not
        // mirrored in this list. Generated TS / JS in those
        // directories must not count toward `repo_languages` —
        // otherwise the protection state can flip away from the
        // truthful label for an Angular or SvelteKit user.
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/app.ts");
        touch(dir.path(), ".angular/cache/0/foo.ts");
        touch(dir.path(), ".svelte-kit/output/server/bar.js");
        touch(dir.path(), "dist/index.js");
        touch(dir.path(), ".turbo/cache/baz.ts");
        let profile = profile_repo(dir.path());
        assert_eq!(
            profile.entries.len(),
            1,
            "only src/app.ts should be counted: {profile:?}"
        );
        assert_eq!(profile.entries[0].name, "TypeScript");
        assert_eq!(profile.entries[0].files_seen, 1);
    }

    #[test]
    fn fixture_and_test_dirs_are_excluded() {
        // Round-2 follow-up review (PR #1274): the doc-comment
        // sync-contract claimed the list mirrors
        // `DEFAULT_DIR_EXCLUDES`, but the fixture / mock / test
        // directories were missing. Generated stub TS / Python in
        // those locations would otherwise inflate `repo_languages`.
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/app.ts");
        touch(dir.path(), "__fixtures__/sample.ts");
        touch(dir.path(), "__mocks__/api.ts");
        touch(dir.path(), "__tests__/util.ts");
        touch(dir.path(), "test-data/seed.py");
        touch(dir.path(), "fixtures/payload.json");
        let profile = profile_repo(dir.path());
        assert_eq!(
            profile.entries.len(),
            1,
            "only src/app.ts should be counted: {profile:?}"
        );
        assert_eq!(profile.entries[0].name, "TypeScript");
        assert_eq!(profile.entries[0].files_seen, 1);
    }

    #[test]
    fn unsupported_extensions_returns_dotted_extensions() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "app.go");
        touch(dir.path(), "main.rs");
        let profile = profile_repo(dir.path());
        let unsupported = profile.unsupported_extensions();
        assert!(unsupported.contains(&".go".to_string()));
        // Rust and Python are supported tiers — their extensions must NOT be
        // listed as unsupported (CIB-123 lifted Python).
        assert!(!unsupported.contains(&".rs".to_string()));
        assert!(!unsupported.contains(&".py".to_string()));
        // Supported / partial extensions must NOT be included.
        assert!(!unsupported.contains(&".ts".to_string()));
        assert!(!unsupported.contains(&".sql".to_string()));
    }

    #[test]
    fn partition_drops_unsupported_files_and_tallies_ledger() {
        // LAUNCH-016 acceptance: Supported-tier files (TS, Rust) keep
        // going to language-specific antipattern checks; Unsupported-tier
        // files (Go) are dropped and the ledger records the skip with
        // language and count.
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/a.ts");
        touch(dir.path(), "lib/util.go");
        touch(dir.path(), "scripts/cleanup.go");
        touch(dir.path(), "main.rs");
        let profile = profile_repo(dir.path());

        let files = vec!["src/a.ts", "lib/util.go", "scripts/cleanup.go", "main.rs"];
        let (scannable, ledger) = partition_for_language_specific_checks(&files, &profile);

        // TS and Rust are supported — both stay scannable (input order
        // preserved); only the two Go files are skipped.
        assert_eq!(scannable, vec!["src/a.ts", "main.rs"]);
        assert_eq!(ledger.by_language.get("Go"), Some(&2));
        assert!(!ledger.by_language.contains_key("Rust"));
        assert_eq!(ledger.total(), 2);
        assert_eq!(ledger.reason, SkipReason::Unsupported);
    }

    #[test]
    fn partition_passes_through_when_no_unsupported_languages_detected() {
        // A repo whose languages are all Supported / Partial gets an
        // empty ledger and an unfiltered list — the partition is a
        // no-op, consistent with the council-locked rule that the
        // existing allowlist is the default.
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/a.ts");
        touch(dir.path(), "src/b.tsx");
        touch(dir.path(), "schema.sql");
        let profile = profile_repo(dir.path());

        let files = vec!["src/a.ts", "src/b.tsx", "schema.sql"];
        let (scannable, ledger) = partition_for_language_specific_checks(&files, &profile);
        assert_eq!(scannable.len(), 3);
        assert!(ledger.is_empty());
    }

    #[test]
    fn partition_keeps_unclassified_files() {
        // Files without a registered extension (e.g. `Makefile`, no
        // extension) pass through unchanged so existing antipattern
        // logic decides what to do with them.
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/a.ts");
        touch(dir.path(), "lib/util.go");
        touch(dir.path(), "Makefile");
        touch(dir.path(), "README.txt");
        let profile = profile_repo(dir.path());

        let files = vec!["src/a.ts", "lib/util.go", "Makefile", "README.txt"];
        let (scannable, ledger) = partition_for_language_specific_checks(&files, &profile);

        assert!(scannable.contains(&"src/a.ts"));
        assert!(scannable.contains(&"Makefile"));
        assert!(scannable.contains(&"README.txt"));
        assert!(!scannable.contains(&"lib/util.go"));
        assert_eq!(ledger.by_language.get("Go"), Some(&1));
    }

    #[test]
    fn partition_is_case_insensitive_on_extensions() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "src/a.ts");
        touch(dir.path(), "lib/Util.GO");
        let profile = profile_repo(dir.path());

        let files = vec!["src/a.ts", "lib/Util.GO"];
        let (scannable, ledger) = partition_for_language_specific_checks(&files, &profile);
        assert_eq!(scannable, vec!["src/a.ts"]);
        assert_eq!(ledger.by_language.get("Go"), Some(&1));
    }

    #[test]
    fn skip_ledger_serialisation_is_stable() {
        // Use two still-`Unsupported` languages — Rust and Python are now
        // supported tiers and can no longer appear in a skip ledger produced
        // by the normal code path.
        let mut ledger = LanguageSkipLedger::default();
        ledger.by_language.insert("Dart".to_string(), 3);
        ledger.by_language.insert("Go".to_string(), 1);
        let json = serde_json::to_value(&ledger).unwrap();
        assert_eq!(json["reason"], "unsupported");
        assert_eq!(json["by_language"]["Dart"], 3);
        assert_eq!(json["by_language"]["Go"], 1);
    }

    #[test]
    fn profile_iteration_order_matches_registry_order() {
        let dir = TempDir::new().unwrap();
        // Touch in reverse-registry order to confirm the output
        // sticks to registry order, not insertion order.
        touch(dir.path(), "main.rs");
        touch(dir.path(), "app.py");
        touch(dir.path(), "src/a.ts");
        let profile = profile_repo(dir.path());
        let names: Vec<&str> = profile.entries.iter().map(|e| e.name.as_str()).collect();
        let ts_pos = names.iter().position(|n| *n == "TypeScript").unwrap();
        let py_pos = names.iter().position(|n| *n == "Python").unwrap();
        let rs_pos = names.iter().position(|n| *n == "Rust").unwrap();
        // "Most-supported first": Rust (Supported) is grouped with the
        // supported tier ahead of Python (Unsupported).
        assert!(ts_pos < rs_pos, "names: {names:?}");
        assert!(rs_pos < py_pos, "names: {names:?}");
    }
}
