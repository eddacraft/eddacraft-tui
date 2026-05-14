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

use serde::{Deserialize, Serialize};

/// Closed enum of AI tools Anvil knows how to detect today.
///
/// The order is the order [`detect_all`] reports them in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    ClaudeCode,
    Cursor,
    Aider,
    Windsurf,
    Codex,
}

impl AgentKind {
    /// Stable short id used in JSON, log lines, and CLI summary.
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

impl AgentInventory {
    /// `true` if the inventory contains an agent of `kind`.
    #[must_use]
    pub fn contains(&self, kind: AgentKind) -> bool {
        self.detected.iter().any(|a| a.kind == kind)
    }
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

fn join_home(home: &str, rel: &str) -> String {
    let trimmed = home.trim_end_matches(['/', '\\']);
    format!("{trimmed}/{rel}")
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
            config_paths: &[".cursor"],
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
            self.paths.insert(path.to_string());
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
            self.paths.contains(path)
        }
        fn env(&self, name: &str) -> Option<String> {
            self.env_vars.get(name).cloned()
        }
        fn home_dir(&self) -> Option<String> {
            self.home.clone()
        }
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
        // We accept either form from the host.
        let env = StubEnv::default()
            .with_home("/home/dev/")
            .path("/home/dev/.claude");
        let inv = detect_all(&env);
        assert_eq!(inv.detected.len(), 1);
        assert_eq!(inv.detected[0].kind, AgentKind::ClaudeCode);
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
            assert!(inv.contains(*k), "missing {k:?}");
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
        // Stable kind ids appear in the JSON.
        assert!(json.contains("\"claude_code\"") || json.contains("\"claude-code\""));
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
