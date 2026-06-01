//! AI-tool auto-detection primitive (ADOPT-003).
//!
//! Pure detection: given a [`DetectionEnv`] (PATH lookup, file-exists,
//! env-var, home-dir queries), return an [`AgentInventory`] of the
//! installed AI tools we know how to recognise. The result is what
//! `anvil start` / `anvil status` print as the "AI tools detected"
//! summary, and is what gets cached at
//! `.anvil/cache/detected-agents.json`.
//!
//! Detection precedence per agent: a binary on `PATH` or a
//! well-known config path under the user's home directory is a
//! **strong** signal and is enough on its own. Environment
//! variables are recorded as **weak** supporting evidence — they
//! never gate detection by themselves because the same variable
//! (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) is commonly set by users
//! who do not have the tool installed.
//!
//! All five tools called out by the ADOPT-003 task are covered:
//! Claude Code, Cursor, Aider, Windsurf, Codex. The detection rules
//! are listed in [`detection_rule`] and intentionally compact so
//! they can be reviewed in a diff when a tool ships a renamed
//! binary or moves its config directory.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Closed enum of AI tools Anvil knows how to detect today.
///
/// The order is the order [`detect_all`] reports them in. JSON
/// representation is kebab-case to match [`AgentKind::id`] —
/// keep these two views in lock-step so the cache key and the
/// log-line label are interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentKind {
    ClaudeCode,
    Cursor,
    Aider,
    Windsurf,
    Codex,
}

impl AgentKind {
    /// Stable short id used in JSON, log lines, and CLI summary.
    /// Matches the kebab-case serde representation; the
    /// `agent_kind_id_matches_serde_representation` test pins them.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            AgentKind::ClaudeCode => "claude-code",
            AgentKind::Cursor => "cursor",
            AgentKind::Aider => "aider",
            AgentKind::Windsurf => "windsurf",
            AgentKind::Codex => "codex",
        }
    }

    /// Every variant. Order is the reporting order.
    #[must_use]
    pub fn all() -> &'static [AgentKind] {
        &[
            AgentKind::ClaudeCode,
            AgentKind::Cursor,
            AgentKind::Aider,
            AgentKind::Windsurf,
            AgentKind::Codex,
        ]
    }
}

/// Per-evidence-item record. Multiple variants can fire for the
/// same agent (binary + config + env all set) — order in the
/// `evidence` vector reflects the lookup order in
/// [`detect_kind`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DetectionEvidence {
    /// Named binary resolved on `PATH`.
    BinaryOnPath { name: String },
    /// Config path (file or directory) exists, relative to the
    /// user's home directory at detection time. The path is the
    /// resolved, joined string the [`DetectionEnv`] saw.
    ConfigPath { path: String },
    /// Named environment variable was set. Weak signal; never the
    /// sole reason an agent appears in the inventory.
    EnvVar { name: String },
}

impl DetectionEvidence {
    /// `true` for evidence types that are enough on their own to
    /// declare the agent detected.
    #[must_use]
    pub fn is_strong(&self) -> bool {
        matches!(
            self,
            DetectionEvidence::BinaryOnPath { .. } | DetectionEvidence::ConfigPath { .. }
        )
    }
}

/// One agent the detector decided is present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedAgent {
    pub kind: AgentKind,
    pub evidence: Vec<DetectionEvidence>,
}

/// The set of agents the detector found. Stored at
/// `.anvil/cache/detected-agents.json` on the host.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInventory {
    pub detected: Vec<DetectedAgent>,
}

/// Trait abstracting the host so the pure primitive stays
/// testable. The CLI implements this with `which`, `Path::exists`,
/// `std::env::var`, and `dirs::home_dir`.
pub trait DetectionEnv {
    /// Returns `true` if a binary by this name is resolvable on
    /// `PATH`. Implementations should follow the platform's normal
    /// extension rules (`name.exe` on Windows is the consumer's
    /// concern, not the rule's).
    fn has_binary(&self, name: &str) -> bool;

    /// Returns `true` if a file or directory exists at this path.
    fn path_exists(&self, path: &str) -> bool;

    /// Returns the value of the named environment variable, or
    /// `None` if unset / non-UTF-8.
    fn env(&self, name: &str) -> Option<String>;

    /// The user's home directory as a string, if available. Used
    /// only to construct the config-path candidates from the
    /// agent rule.
    fn home_dir(&self) -> Option<String>;
}

/// Real host-backed [`DetectionEnv`] used by `anvil start`. Queries
/// the live process environment for binary lookup (manual PATH
/// search to avoid pulling the `which` dev-dep onto the runtime
/// surface), `Path::exists` for config-path existence,
/// `std::env::var` for environment variables, and `dirs::home_dir`
/// for the user's home directory. Stateless — clone or instantiate
/// freely; one detection pass is one call to [`detect_all`].
///
/// Windows note: the trait contract is "follow the platform's
/// normal extension rules". The current impl tries `name` and
/// `name.exe` on Windows; richer PATHEXT handling is filed as a
/// follow-up because every rule today names a POSIX binary.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealDetectionEnv;

impl DetectionEnv for RealDetectionEnv {
    fn has_binary(&self, name: &str) -> bool {
        let Some(path) = std::env::var_os("PATH") else {
            return false;
        };
        for dir in std::env::split_paths(&path) {
            // POSIX gives empty `PATH` components the meaning of
            // "current working directory". `anvil start` runs from
            // the user's repo root and a planted `claude` in cwd
            // would let any repo claim Claude Code is installed
            // for any user who runs `anvil start` inside it. We
            // intentionally diverge from that POSIX semantic and
            // skip empty components — the cost is false negatives
            // for the (vanishingly rare) user who genuinely keeps
            // an AI-tool binary in cwd; the win is closing a
            // spoofing surface that the detection cache feeds.
            if dir.as_os_str().is_empty() {
                continue;
            }
            let candidate = dir.join(name);
            // On Windows, accept the bare `dir.join(name)` only when
            // the caller already supplied an extension (the agent
            // rules never do, but a future rule might list
            // `tool.bat`). Without this guard a plain text file
            // named `claude` on PATH would match `is_executable_file`
            // because Windows has no execute bit to gate on; the
            // PATHEXT-style `.exe` fallback below is the real check
            // for the extensionless-name case. On Unix the
            // executable-bit check inside `is_executable_file`
            // already screens out plain files.
            #[cfg(windows)]
            let accept_bare = candidate.extension().is_some();
            #[cfg(not(windows))]
            let accept_bare = true;
            if accept_bare && is_executable_file(&candidate) {
                return true;
            }
            #[cfg(windows)]
            {
                let mut with_exe = candidate.clone();
                with_exe.set_extension("exe");
                if is_executable_file(&with_exe) {
                    return true;
                }
            }
        }
        false
    }

    fn path_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }

    fn env(&self, name: &str) -> Option<String> {
        // `std::env::var` returns `Err` for both missing and
        // non-UTF-8 values; the trait contract collapses both to
        // `None`, so swallowing the error variant is correct here.
        std::env::var(name).ok()
    }

    fn home_dir(&self) -> Option<String> {
        dirs::home_dir().map(|p| p.to_string_lossy().into_owned())
    }
}

/// `true` if `path` is a regular file with any Unix execute bit
/// set (owner / group / other), used as a lightweight gate against
/// non-executable files on PATH. This is a coarser check than
/// `access(X_OK)` — a file owned by another user with `0o110`
/// would pass here but fail under the current uid — but it is
/// enough to reject the common false-positive cases (download
/// artefacts, leftover config files, plain text) without taking
/// a libc dep. On Windows there is no execute bit, so the plain
/// existence check is left in place; the `set_extension("exe")`
/// callers in [`RealDetectionEnv::has_binary`] handle the
/// extensionless-name spoof there.
fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(metadata) = std::fs::metadata(path) else {
            return false;
        };
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Relative path of the on-disk cache, joined onto the repo root.
/// The matching consumer in `anvil-run` pins the same path via
/// `anvil_run::detection::cache_path`; the
/// `cache_path_is_relative_to_anvil_cache_dir` test pins both.
#[must_use]
pub fn cache_path(root: &Path) -> PathBuf {
    root.join(".anvil")
        .join("cache")
        .join("detected-agents.json")
}

/// Outcome of [`detect_and_cache`]. Always carries the in-memory
/// inventory the detector observed, even when the disk write
/// failed — the caller chooses how to surface the write error
/// (typically by logging the `write_error` field) without losing
/// the data needed to render the human summary line.
#[derive(Debug)]
pub struct CacheOutcome {
    pub inventory: AgentInventory,
    /// `Some` when the cache write failed. The inventory above is
    /// still the live detection result; the error is informational.
    pub write_error: Option<anyhow::Error>,
}

/// Run detection against the given [`DetectionEnv`] and write the
/// inventory to `<root>/.anvil/cache/detected-agents.json`. The
/// detection result is returned in [`CacheOutcome::inventory`]
/// regardless of whether the cache write succeeded — Council
/// review surfaced that swallowing the inventory on write failure
/// silently degraded the user-facing summary line. A write error
/// is reported via [`CacheOutcome::write_error`] so the caller can
/// log it and still print the correct "AI tools detected: …" line.
///
/// The write is read-compare-skipped: if the existing cache file
/// already deserialises to the same `AgentInventory`, the write is
/// suppressed. This keeps the cache file's mtime stable across
/// repeated `anvil start` runs on an unchanged host, so backup /
/// sync / file-watch consumers do not see spurious churn on every
/// activation. A serialisation drift would still rewrite because
/// the comparison is structural, not textual.
///
/// The write is best-effort atomic via [`crate::util::atomic_write`]
/// (same primitive the warmup cache and `.anvilrc` paths use) so a
/// crashed mid-write never leaves a half-serialised JSON document
/// for `anvil-run` to read.
pub fn detect_and_cache(root: &Path, env: &dyn DetectionEnv) -> CacheOutcome {
    let inventory = detect_all(env);
    let write_error = write_inventory_cache(root, &inventory).err();
    CacheOutcome {
        inventory,
        write_error,
    }
}

fn write_inventory_cache(root: &Path, inventory: &AgentInventory) -> anyhow::Result<()> {
    let path = cache_path(root);

    // Read-compare-skip: avoid rewriting an unchanged inventory so
    // the cache file's mtime stays stable across activations.
    if let Ok(existing) = std::fs::read_to_string(&path)
        && let Ok(prev) = serde_json::from_str::<AgentInventory>(&existing)
        && &prev == inventory
    {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(inventory).context("serialising AI tool inventory")?;
    crate::util::atomic_write(&path, json.as_bytes())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Render the inventory as a single-line summary suitable for the
/// `anvil start` human output. Returns the empty string when no
/// agents were detected — the caller decides whether to print a
/// "no AI tools detected" placeholder or skip the line entirely.
#[must_use]
pub fn render_inventory_summary(inv: &AgentInventory) -> String {
    if inv.detected.is_empty() {
        return String::new();
    }
    let ids: Vec<&str> = inv.detected.iter().map(|a| a.kind.id()).collect();
    format!("AI tools detected: {}", ids.join(", "))
}

/// Detect every known agent. Order in the returned inventory
/// matches [`AgentKind::all`].
#[must_use]
pub fn detect_all(env: &dyn DetectionEnv) -> AgentInventory {
    let detected = AgentKind::all()
        .iter()
        .copied()
        .filter_map(|k| detect_kind(env, k))
        .collect();
    AgentInventory { detected }
}

/// Detect a single agent. Returns `Some` only when at least one
/// strong signal (binary on PATH or known config path) fires.
/// All matching evidence (including weak env-var hints) is
/// captured for transparency.
#[must_use]
pub fn detect_kind(env: &dyn DetectionEnv, kind: AgentKind) -> Option<DetectedAgent> {
    let rule = detection_rule(kind);
    let mut evidence = Vec::new();

    for binary in rule.binaries {
        if env.has_binary(binary) {
            evidence.push(DetectionEvidence::BinaryOnPath {
                name: (*binary).to_string(),
            });
        }
    }

    if let Some(home) = env.home_dir() {
        for rel in rule.config_paths {
            let full = join_home(&home, rel);
            if env.path_exists(&full) {
                evidence.push(DetectionEvidence::ConfigPath { path: full });
            }
        }
    }

    for var in rule.env_vars {
        if env.env(var).is_some() {
            evidence.push(DetectionEvidence::EnvVar {
                name: (*var).to_string(),
            });
        }
    }

    if evidence.iter().any(DetectionEvidence::is_strong) {
        Some(DetectedAgent { kind, evidence })
    } else {
        None
    }
}

/// Join a home-relative path onto the user's home directory in a
/// platform-correct way. Uses [`PathBuf`] so Windows
/// (`C:\Users\foo`) and POSIX home directories produce native
/// separators rather than the mixed `C:\Users\foo/.claude` that a
/// naive string format would emit.
fn join_home(home: &str, rel: &str) -> String {
    let mut path = PathBuf::from(home);
    // `rel` is forward-slash by convention (so the table reads
    // the same on every platform); split and push each component
    // through `Path::push` so the resulting separators match the
    // host's native form.
    for component in rel.split('/').filter(|s| !s.is_empty()) {
        path.push(component);
    }
    path.to_string_lossy().into_owned()
}

/// Detection heuristics for one agent. Public for documentation
/// in CLI help; the [`AgentKind::all`] / [`detect_kind`] pair is
/// the consumer entry point.
#[derive(Debug, Clone, Copy)]
pub struct DetectionRule {
    pub binaries: &'static [&'static str],
    /// Paths relative to the user's home directory. Forward
    /// slashes only — [`join_home`] strips trailing separators.
    pub config_paths: &'static [&'static str],
    /// Environment variables treated as weak hints.
    pub env_vars: &'static [&'static str],
}

/// Return the detection rule for one agent. Kept as a `match`
/// rather than a static table so the rule sits adjacent to the
/// enum and survives diff review when a tool renames its
/// binary.
#[must_use]
pub fn detection_rule(kind: AgentKind) -> DetectionRule {
    match kind {
        AgentKind::ClaudeCode => DetectionRule {
            binaries: &["claude"],
            config_paths: &[".claude"],
            env_vars: &["ANTHROPIC_API_KEY", "CLAUDE_CODE_HOME"],
        },
        AgentKind::Cursor => DetectionRule {
            binaries: &["cursor"],
            // 2026 Cursor stores per-user state under the OS-native
            // app-config directory. `.cursor` is the legacy
            // dot-file location older installs still create. We
            // probe all three relative to `$HOME`; the consumer's
            // `path_exists` short-circuits on the first hit. The
            // Windows `%APPDATA%\Cursor` path is not addressable
            // through `home_dir` alone — that requires extending
            // `DetectionEnv` and is filed as a follow-up.
            config_paths: &[
                ".cursor",
                ".config/Cursor",
                "Library/Application Support/Cursor",
            ],
            env_vars: &["CURSOR_HOME"],
        },
        AgentKind::Aider => DetectionRule {
            binaries: &["aider"],
            config_paths: &[".aider.conf.yml", ".aider"],
            env_vars: &["AIDER_MODEL", "AIDER_API_KEY"],
        },
        AgentKind::Windsurf => DetectionRule {
            binaries: &["windsurf"],
            config_paths: &[".codeium/windsurf"],
            env_vars: &["WINDSURF_HOME"],
        },
        AgentKind::Codex => DetectionRule {
            binaries: &["codex", "codex-cli"],
            config_paths: &[".codex"],
            env_vars: &["CODEX_HOME"],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::path::Path;

    /// Stub env for deterministic tests.
    #[derive(Default)]
    struct StubEnv {
        binaries: HashSet<String>,
        paths: HashSet<String>,
        env_vars: HashMap<String, String>,
        home: Option<String>,
    }

    impl StubEnv {
        fn with_home(mut self, home: &str) -> Self {
            self.home = Some(home.to_string());
            self
        }
        fn binary(mut self, name: &str) -> Self {
            self.binaries.insert(name.to_string());
            self
        }
        fn path(mut self, path: &str) -> Self {
            self.paths.insert(normalize_separators(path));
            self
        }
        fn var(mut self, name: &str, value: &str) -> Self {
            self.env_vars.insert(name.to_string(), value.to_string());
            self
        }
    }

    impl DetectionEnv for StubEnv {
        fn has_binary(&self, name: &str) -> bool {
            self.binaries.contains(name)
        }
        fn path_exists(&self, path: &str) -> bool {
            self.paths.contains(&normalize_separators(path))
        }
        fn env(&self, name: &str) -> Option<String> {
            self.env_vars.get(name).cloned()
        }
        fn home_dir(&self) -> Option<String> {
            self.home.clone()
        }
    }

    // The detection probes paths via `join_home`, which emits the host's
    // native separator (`\` on Windows) while the test table registers
    // forward-slash literals. Normalising both the registered and probed
    // paths to `/` makes the in-memory stub separator-agnostic, mirroring
    // the real `Path::exists` it stands in for.
    fn normalize_separators(path: &str) -> String {
        path.replace('\\', "/")
    }

    #[test]
    fn agent_kind_ids_are_stable() {
        // These appear in JSON and log lines — don't rename
        // without a release note.
        assert_eq!(AgentKind::ClaudeCode.id(), "claude-code");
        assert_eq!(AgentKind::Cursor.id(), "cursor");
        assert_eq!(AgentKind::Aider.id(), "aider");
        assert_eq!(AgentKind::Windsurf.id(), "windsurf");
        assert_eq!(AgentKind::Codex.id(), "codex");
    }

    #[test]
    fn agent_kind_id_matches_serde_representation() {
        // The cache JSON serialises `AgentKind` via serde; CLI
        // log lines use `id()`. Two divergent representations of
        // the same identity would silently break consumers that
        // cross-reference them. Lock the two together.
        for kind in AgentKind::all() {
            let json = serde_json::to_string(kind).unwrap();
            let expected = format!("\"{}\"", kind.id());
            assert_eq!(json, expected, "drift for {kind:?}");
        }
    }

    #[test]
    fn agent_kind_all_covers_the_full_adopt_003_set() {
        // ADOPT-003 names exactly these five tools.
        let kinds: HashSet<_> = AgentKind::all().iter().copied().collect();
        assert!(kinds.contains(&AgentKind::ClaudeCode));
        assert!(kinds.contains(&AgentKind::Cursor));
        assert!(kinds.contains(&AgentKind::Aider));
        assert!(kinds.contains(&AgentKind::Windsurf));
        assert!(kinds.contains(&AgentKind::Codex));
        assert_eq!(AgentKind::all().len(), 5);
    }

    #[test]
    fn detect_returns_empty_when_nothing_installed() {
        let env = StubEnv::default().with_home("/home/dev");
        let inv = detect_all(&env);
        assert!(inv.detected.is_empty());
    }

    #[test]
    fn binary_on_path_alone_is_strong_enough() {
        let env = StubEnv::default().with_home("/home/dev").binary("claude");
        let inv = detect_all(&env);
        assert_eq!(inv.detected.len(), 1);
        assert_eq!(inv.detected[0].kind, AgentKind::ClaudeCode);
        assert!(matches!(
            inv.detected[0].evidence[0],
            DetectionEvidence::BinaryOnPath { .. }
        ));
    }

    #[test]
    fn config_path_alone_is_strong_enough() {
        let env = StubEnv::default()
            .with_home("/home/dev")
            .path("/home/dev/.claude");
        let inv = detect_all(&env);
        assert_eq!(inv.detected.len(), 1);
        assert_eq!(inv.detected[0].kind, AgentKind::ClaudeCode);
        assert!(
            inv.detected[0]
                .evidence
                .iter()
                .any(|e| matches!(e, DetectionEvidence::ConfigPath { .. }))
        );
    }

    #[test]
    fn env_var_alone_is_not_enough_to_detect() {
        // ANTHROPIC_API_KEY is set by many users who do not have
        // Claude Code installed.
        let env = StubEnv::default()
            .with_home("/home/dev")
            .var("ANTHROPIC_API_KEY", "sk-...");
        let inv = detect_all(&env);
        assert!(inv.detected.is_empty());
    }

    #[test]
    fn env_var_with_strong_signal_is_recorded_as_supporting_evidence() {
        let env = StubEnv::default()
            .with_home("/home/dev")
            .binary("claude")
            .var("ANTHROPIC_API_KEY", "sk-...");
        let inv = detect_all(&env);
        assert_eq!(inv.detected.len(), 1);
        let agent = &inv.detected[0];
        assert!(
            agent
                .evidence
                .iter()
                .any(|e| matches!(e, DetectionEvidence::BinaryOnPath { .. }))
        );
        assert!(agent.evidence.iter().any(
            |e| matches!(e, DetectionEvidence::EnvVar { name } if name == "ANTHROPIC_API_KEY")
        ));
    }

    #[test]
    fn missing_home_dir_still_detects_via_path() {
        // home_dir() can return None on minimal containers; binary
        // detection must still work.
        let env = StubEnv {
            home: None,
            ..StubEnv::default()
        }
        .binary("cursor");
        let inv = detect_all(&env);
        assert_eq!(inv.detected.len(), 1);
        assert_eq!(inv.detected[0].kind, AgentKind::Cursor);
    }

    #[test]
    fn home_dir_trailing_slash_does_not_double_up() {
        // PathBuf::push handles trailing-separator normalisation
        // on every platform, so the consumer's `path_exists` sees
        // the canonical joined path regardless of how the host
        // reports `home_dir`.
        let env = StubEnv::default()
            .with_home("/home/dev/")
            .path("/home/dev/.claude");
        let inv = detect_all(&env);
        assert_eq!(inv.detected.len(), 1);
        assert_eq!(inv.detected[0].kind, AgentKind::ClaudeCode);
    }

    #[test]
    fn join_home_normalises_separators_per_platform() {
        // The probed path matches the host's native separator
        // form so `Path::exists` works without further
        // normalisation in the real `DetectionEnv` impl.
        let joined = join_home("/home/dev", ".claude");
        let expected = Path::new("/home/dev").join(".claude");
        assert_eq!(joined, expected.to_string_lossy());
    }

    #[test]
    fn cursor_modern_config_paths_are_probed() {
        // Modern Cursor stores per-user state under the OS-native
        // app-config directory; the legacy `.cursor` dot-file is
        // not always present on recent installs.
        for probed in [
            ".cursor",
            ".config/Cursor",
            "Library/Application Support/Cursor",
        ] {
            let full = join_home("/home/dev", probed);
            let env = StubEnv::default().with_home("/home/dev").path(&full);
            let inv = detect_all(&env);
            assert_eq!(inv.detected.len(), 1, "expected detection via {probed}");
            assert_eq!(inv.detected[0].kind, AgentKind::Cursor);
        }
    }

    #[test]
    fn detect_all_reports_in_kind_order() {
        let env = StubEnv::default()
            .with_home("/home/dev")
            .binary("claude")
            .binary("aider")
            .binary("codex");
        let inv = detect_all(&env);
        let kinds: Vec<_> = inv.detected.iter().map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            vec![AgentKind::ClaudeCode, AgentKind::Aider, AgentKind::Codex]
        );
    }

    #[test]
    fn windsurf_uses_codeium_config_path() {
        let env = StubEnv::default()
            .with_home("/home/dev")
            .path("/home/dev/.codeium/windsurf");
        let inv = detect_all(&env);
        assert_eq!(inv.detected.len(), 1);
        assert_eq!(inv.detected[0].kind, AgentKind::Windsurf);
    }

    #[test]
    fn codex_recognises_both_codex_and_codex_cli_binaries() {
        for bin in ["codex", "codex-cli"] {
            let env = StubEnv::default().with_home("/home/dev").binary(bin);
            let inv = detect_all(&env);
            assert_eq!(inv.detected.len(), 1, "binary {bin} should detect codex");
            assert_eq!(inv.detected[0].kind, AgentKind::Codex);
        }
    }

    #[test]
    fn all_five_agents_can_be_detected_simultaneously() {
        let env = StubEnv::default()
            .with_home("/home/dev")
            .binary("claude")
            .binary("cursor")
            .binary("aider")
            .binary("windsurf")
            .binary("codex");
        let inv = detect_all(&env);
        assert_eq!(inv.detected.len(), 5);
        for k in AgentKind::all() {
            assert!(inv.detected.iter().any(|a| a.kind == *k), "missing {k:?}");
        }
    }

    #[test]
    fn detect_kind_returns_some_when_only_that_kind_present() {
        let env = StubEnv::default().with_home("/home/dev").binary("aider");
        assert!(detect_kind(&env, AgentKind::Aider).is_some());
        assert!(detect_kind(&env, AgentKind::ClaudeCode).is_none());
    }

    #[test]
    fn detection_evidence_is_strong_flags_only_binary_and_config() {
        assert!(
            DetectionEvidence::BinaryOnPath {
                name: "claude".into(),
            }
            .is_strong()
        );
        assert!(
            DetectionEvidence::ConfigPath {
                path: "/home/dev/.claude".into(),
            }
            .is_strong()
        );
        assert!(
            !DetectionEvidence::EnvVar {
                name: "ANTHROPIC_API_KEY".into(),
            }
            .is_strong()
        );
    }

    #[test]
    fn inventory_round_trips_through_json() {
        let env = StubEnv::default()
            .with_home("/home/dev")
            .binary("claude")
            .path("/home/dev/.cursor")
            .var("CLAUDE_CODE_HOME", "/opt/claude");
        let inv = detect_all(&env);
        let json = serde_json::to_string_pretty(&inv).unwrap();
        let parsed: AgentInventory = serde_json::from_str(&json).unwrap();
        assert_eq!(inv, parsed);
        // Stable kebab-case kind id appears in the JSON. The
        // snake_case fallback that used to live here would have
        // hidden an accidental `rename_all` regression.
        assert!(
            json.contains("\"claude-code\""),
            "expected kebab-case kind id; got:\n{json}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn is_executable_file_rejects_non_executable_regular_files() {
        // Council MAJOR — a stray non-executable file named
        // `claude` on PATH would otherwise mark Claude Code as
        // installed and pollute the cache that anvil-run reads.
        // PATH-mutation tests are blocked by the workspace's
        // `unsafe_code = "forbid"` lint (Rust 1.83 made
        // `set_var` unsafe), so we exercise the helper directly.
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let plain = tmp.path().join("claude");
        std::fs::write(&plain, b"#!/bin/sh\necho not really installed\n").unwrap();

        std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            !is_executable_file(&plain),
            "non-executable regular file must not pass the binary check"
        );

        std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(
            is_executable_file(&plain),
            "regular file with any exec bit must pass the binary check"
        );

        let missing = tmp.path().join("nope");
        assert!(!is_executable_file(&missing));
    }

    #[test]
    fn detect_and_cache_writes_inventory_json_at_pinned_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let env = StubEnv::default().with_home("/home/dev").binary("claude");
        let outcome = detect_and_cache(tmp.path(), &env);
        assert!(outcome.write_error.is_none(), "write must succeed");

        let cache = tmp.path().join(".anvil/cache/detected-agents.json");
        assert!(
            cache.is_file(),
            "cache file must be written at the pinned path"
        );
        let raw = std::fs::read_to_string(&cache).unwrap();
        let reparsed: AgentInventory = serde_json::from_str(&raw).unwrap();
        assert_eq!(reparsed, outcome.inventory);
        // The wire contract that anvil-run reads against: kebab-case ids
        // in the `kind` field. Pin the literal so a serde rename in
        // detect_agents.rs is caught here before anvil-run drifts.
        assert!(raw.contains("\"kind\": \"claude-code\""), "got:\n{raw}");
    }

    #[test]
    fn detect_and_cache_creates_missing_parent_directories() {
        // `.anvil/cache/` may not exist when `anvil start` runs on a
        // fresh repo. The cache writer must create the directory tree
        // before atomic_write touches the file.
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(!tmp.path().join(".anvil").exists());
        let env = StubEnv::default().with_home("/home/dev");
        let outcome = detect_and_cache(tmp.path(), &env);
        assert!(outcome.write_error.is_none());
        assert!(tmp.path().join(".anvil/cache").is_dir());
    }

    #[test]
    fn detect_and_cache_overwrites_stale_inventory_on_rerun() {
        // The cache is a snapshot of the latest detection; the writer
        // must not append or grow the file across runs.
        let tmp = tempfile::TempDir::new().unwrap();
        let env_first = StubEnv::default()
            .with_home("/home/dev")
            .binary("claude")
            .binary("cursor");
        let _ = detect_and_cache(tmp.path(), &env_first);

        let env_second = StubEnv::default().with_home("/home/dev").binary("aider");
        let outcome = detect_and_cache(tmp.path(), &env_second);

        let cache_text =
            std::fs::read_to_string(tmp.path().join(".anvil/cache/detected-agents.json")).unwrap();
        let reparsed: AgentInventory = serde_json::from_str(&cache_text).unwrap();
        assert_eq!(reparsed, outcome.inventory);
        assert_eq!(reparsed.detected.len(), 1);
        assert_eq!(reparsed.detected[0].kind, AgentKind::Aider);
    }

    #[test]
    fn detect_and_cache_does_not_rewrite_when_inventory_is_unchanged() {
        // Council MAJOR — `atomic_write` always rewrites, so a naive
        // call on every `anvil start` thrashes the cache's mtime even
        // when nothing changed. The read-compare-skip in
        // `write_inventory_cache` must hold the mtime stable across
        // identical activations so file-watch / backup / sync
        // consumers see no spurious churn.
        let tmp = tempfile::TempDir::new().unwrap();
        let env = StubEnv::default().with_home("/home/dev").binary("claude");

        let outcome = detect_and_cache(tmp.path(), &env);
        assert!(outcome.write_error.is_none());

        let cache = tmp.path().join(".anvil/cache/detected-agents.json");
        let mtime_before = std::fs::metadata(&cache).unwrap().modified().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1100));
        let _ = detect_and_cache(tmp.path(), &env);
        let mtime_after = std::fs::metadata(&cache).unwrap().modified().unwrap();

        assert_eq!(
            mtime_before, mtime_after,
            "rerun with unchanged inventory must not rewrite the cache"
        );
    }

    #[test]
    fn detect_and_cache_returns_inventory_even_when_write_fails() {
        // Council MAJOR — when the write side fails, the detector's
        // observation must still reach the caller so the human
        // summary line can be rendered honestly. The `write_error`
        // field carries the failure for the caller to log; the
        // `inventory` field must reflect what `detect_all` saw.
        let tmp = tempfile::TempDir::new().unwrap();
        // Pre-create a *file* at the cache directory location so
        // `create_dir_all(parent)` cannot succeed and the write
        // fails before atomic_write runs.
        let blocked_dir = tmp.path().join(".anvil");
        std::fs::write(&blocked_dir, b"not a directory").unwrap();

        let env = StubEnv::default().with_home("/home/dev").binary("claude");
        let outcome = detect_and_cache(tmp.path(), &env);

        assert!(
            outcome.write_error.is_some(),
            "blocked cache path must surface a write error"
        );
        assert_eq!(
            outcome.inventory.detected.len(),
            1,
            "inventory must reflect live detection even on write failure"
        );
        assert_eq!(outcome.inventory.detected[0].kind, AgentKind::ClaudeCode);
    }

    #[test]
    fn render_inventory_summary_is_empty_when_nothing_detected() {
        let inv = AgentInventory::default();
        assert_eq!(render_inventory_summary(&inv), "");
    }

    #[test]
    fn render_inventory_summary_lists_kebab_case_ids() {
        let env = StubEnv::default()
            .with_home("/home/dev")
            .binary("claude")
            .binary("cursor");
        let inv = detect_all(&env);
        let line = render_inventory_summary(&inv);
        // Stable, comma-separated, kebab-case — matches the JSON
        // wire shape so a user reading the human line and grepping
        // the cache see the same labels.
        assert_eq!(line, "AI tools detected: claude-code, cursor");
    }

    #[test]
    fn cache_path_is_relative_to_anvil_cache_dir() {
        // Pin the writer-side cache path so it cannot drift away from
        // the consumer's pinned path in `anvil_run::detection::cache_path`.
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = cache_path(tmp.path());
        let rel = cache.strip_prefix(tmp.path()).unwrap();
        assert_eq!(rel, Path::new(".anvil/cache/detected-agents.json"));
    }

    #[test]
    fn detection_rules_have_at_least_one_strong_candidate_per_agent() {
        // A rule with no binaries AND no config paths can never
        // detect anything — guard against an accidental edit that
        // removes both lists.
        for kind in AgentKind::all() {
            let rule = detection_rule(*kind);
            assert!(
                !rule.binaries.is_empty() || !rule.config_paths.is_empty(),
                "rule for {kind:?} has no strong signals"
            );
        }
    }
}
