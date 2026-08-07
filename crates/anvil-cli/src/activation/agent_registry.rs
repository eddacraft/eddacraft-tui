//! Canonical agent-client registry for skill and MCP integrations (ADR-106).

use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use super::detect_agents::DetectionEnv;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum AgentClientId {
    ClaudeCode,
    Cursor,
    Codex,
    #[value(name = "opencode", alias = "open-code")]
    OpenCode,
    GeminiCli,
    Antigravity,
    #[value(name = "openclaw", alias = "open-claw")]
    OpenClaw,
    #[value(name = "vscode", alias = "vs-code")]
    VsCode,
    CopilotCli,
    Grok,
    Warp,
    Zed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InstallScope {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpConfigKind {
    McpServersJson,
    ServersJson,
    OpenCodeJson,
    CodexToml,
    GrokToml,
    ZedContextServersJson,
    OpenClawJson,
}

#[derive(Debug, Clone, Copy)]
pub struct AgentClient {
    pub id: AgentClientId,
    pub display_name: &'static str,
    pub binaries: &'static [&'static str],
    pub strong_global_markers: &'static [&'static str],
    pub skill_global_root: Option<&'static str>,
    pub skill_project_root: Option<&'static str>,
    pub mcp_global_path: Option<&'static str>,
    pub mcp_project_path: Option<&'static str>,
    pub mcp_kind: Option<McpConfigKind>,
    pub reload_hint: &'static str,
}

impl AgentClient {
    #[must_use]
    pub fn label(self) -> &'static str {
        self.id.label()
    }

    #[must_use]
    pub fn supports_skill(self, scope: InstallScope) -> bool {
        match scope {
            InstallScope::Global => self.skill_global_root.is_some(),
            InstallScope::Project => self.skill_project_root.is_some(),
        }
    }

    #[must_use]
    pub fn supports_mcp(self, scope: InstallScope) -> bool {
        self.mcp_kind.is_some()
            && match scope {
                InstallScope::Global => self.mcp_global_path.is_some(),
                InstallScope::Project => self.mcp_project_path.is_some(),
            }
    }

    #[must_use]
    pub fn detected(self, env: &impl DetectionEnv) -> bool {
        self.binaries.iter().any(|binary| env.has_binary(binary))
            || env.home_dir().is_some_and(|home| {
                self.strong_global_markers
                    .iter()
                    .map(|marker| Path::new(&home).join(marker))
                    .any(|path| env.path_exists(&path.to_string_lossy()))
            })
    }

    /// Detect an MCP client for a selected scope. Project configuration at the
    /// client's exact documented path is strong evidence even when neither a
    /// binary nor user-global marker is visible; this also keeps an existing
    /// Anvil entry under management after switching activation scope.
    #[must_use]
    pub fn detected_for_mcp(
        self,
        env: &impl DetectionEnv,
        scope: InstallScope,
        root: &Path,
    ) -> bool {
        self.detected(env)
            || (scope == InstallScope::Project
                && self
                    .mcp_path(scope, root)
                    .is_some_and(|path| env.path_exists(&path.to_string_lossy())))
    }

    #[must_use]
    pub fn skill_root(self, scope: InstallScope, root: &Path) -> Option<PathBuf> {
        let relative = match scope {
            InstallScope::Global => self.skill_global_root?,
            InstallScope::Project => self.skill_project_root?,
        };
        // CIB-237: the registry literals are `/`-separated, and `Path::join`
        // keeps them verbatim, so skill-install output printed
        // `C:\Users\dev\.claude/skills`.
        Some(crate::display_path::join_relative(root, relative))
    }

    #[must_use]
    pub fn mcp_path(self, scope: InstallScope, root: &Path) -> Option<PathBuf> {
        if scope == InstallScope::Global && crate::util::user_home_dir().as_deref() == Some(root) {
            if self.id == AgentClientId::CopilotCli
                && let Some(home) = std::env::var_os("COPILOT_HOME")
            {
                return Some(PathBuf::from(home).join("mcp-config.json"));
            }
            if self.id == AgentClientId::Grok
                && let Some(home) = std::env::var_os("GROK_HOME")
            {
                return Some(PathBuf::from(home).join("config.toml"));
            }
        }
        let relative = match scope {
            InstallScope::Global => self.mcp_global_path?,
            InstallScope::Project => self.mcp_project_path?,
        };
        Some(crate::display_path::join_relative(root, relative))
    }
}

impl AgentClientId {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::GeminiCli => "gemini-cli",
            Self::Antigravity => "antigravity",
            Self::OpenClaw => "openclaw",
            Self::VsCode => "vscode",
            Self::CopilotCli => "copilot-cli",
            Self::Grok => "grok",
            Self::Warp => "warp",
            Self::Zed => "zed",
        }
    }

    #[must_use]
    pub fn entry(self) -> &'static AgentClient {
        REGISTRY
            .iter()
            .find(|entry| entry.id == self)
            .expect("every AgentClientId has one registry entry")
    }

    #[must_use]
    pub fn all() -> &'static [AgentClient] {
        REGISTRY
    }
}

const REGISTRY: &[AgentClient] = &[
    AgentClient {
        id: AgentClientId::ClaudeCode,
        display_name: "Claude Code",
        binaries: &["claude"],
        strong_global_markers: &[".claude.json", ".claude"],
        skill_global_root: Some(".claude/skills"),
        skill_project_root: Some(".claude/skills"),
        mcp_global_path: Some(".claude.json"),
        mcp_project_path: Some(".mcp.json"),
        mcp_kind: Some(McpConfigKind::McpServersJson),
        reload_hint: "Restart Claude Code.",
    },
    AgentClient {
        id: AgentClientId::Cursor,
        display_name: "Cursor",
        binaries: &["cursor"],
        strong_global_markers: &[".cursor"],
        skill_global_root: Some(".cursor/skills"),
        skill_project_root: Some(".cursor/skills"),
        mcp_global_path: Some(".cursor/mcp.json"),
        mcp_project_path: Some(".cursor/mcp.json"),
        mcp_kind: Some(McpConfigKind::McpServersJson),
        reload_hint: "Restart Cursor.",
    },
    AgentClient {
        id: AgentClientId::Codex,
        display_name: "Codex",
        binaries: &["codex"],
        strong_global_markers: &[".codex/config.toml"],
        skill_global_root: Some(".agents/skills"),
        skill_project_root: Some(".agents/skills"),
        mcp_global_path: Some(".codex/config.toml"),
        mcp_project_path: Some(".codex/config.toml"),
        mcp_kind: Some(McpConfigKind::CodexToml),
        reload_hint: "Start a new Codex session, then run `codex mcp list`.",
    },
    AgentClient {
        id: AgentClientId::OpenCode,
        display_name: "OpenCode",
        binaries: &["opencode"],
        strong_global_markers: &[".config/opencode/opencode.json"],
        skill_global_root: Some(".config/opencode/skills"),
        skill_project_root: Some(".opencode/skills"),
        mcp_global_path: Some(".config/opencode/opencode.json"),
        mcp_project_path: Some("opencode.json"),
        mcp_kind: Some(McpConfigKind::OpenCodeJson),
        reload_hint: "Restart OpenCode, then run `opencode mcp list`.",
    },
    AgentClient {
        id: AgentClientId::GeminiCli,
        display_name: "Gemini CLI",
        binaries: &["gemini"],
        strong_global_markers: &[".gemini/settings.json"],
        skill_global_root: Some(".agents/skills"),
        skill_project_root: Some(".agents/skills"),
        mcp_global_path: Some(".gemini/settings.json"),
        mcp_project_path: Some(".gemini/settings.json"),
        mcp_kind: Some(McpConfigKind::McpServersJson),
        reload_hint: "Run `/mcp reload` or restart Gemini CLI.",
    },
    AgentClient {
        id: AgentClientId::Antigravity,
        display_name: "Antigravity",
        binaries: &["antigravity"],
        strong_global_markers: &[".gemini/config/mcp_config.json"],
        skill_global_root: None,
        skill_project_root: None,
        mcp_global_path: Some(".gemini/config/mcp_config.json"),
        mcp_project_path: Some(".agents/mcp_config.json"),
        mcp_kind: Some(McpConfigKind::McpServersJson),
        reload_hint: "Restart Antigravity.",
    },
    AgentClient {
        id: AgentClientId::OpenClaw,
        display_name: "OpenClaw",
        binaries: &["openclaw"],
        strong_global_markers: &[".openclaw/openclaw.json"],
        skill_global_root: Some(".agents/skills"),
        skill_project_root: Some(".agents/skills"),
        mcp_global_path: Some(".openclaw/openclaw.json"),
        mcp_project_path: Some(".openclaw/openclaw.json"),
        mcp_kind: Some(McpConfigKind::OpenClawJson),
        reload_hint: "Restart OpenClaw.",
    },
    AgentClient {
        id: AgentClientId::VsCode,
        display_name: "VS Code",
        binaries: &["code"],
        strong_global_markers: &[".config/Code/User"],
        skill_global_root: None,
        skill_project_root: None,
        mcp_global_path: None,
        mcp_project_path: Some(".vscode/mcp.json"),
        mcp_kind: Some(McpConfigKind::ServersJson),
        reload_hint: "Trust and start the server in VS Code, or reload the window.",
    },
    AgentClient {
        id: AgentClientId::CopilotCli,
        display_name: "GitHub Copilot CLI",
        binaries: &["copilot"],
        strong_global_markers: &[".copilot/mcp-config.json"],
        skill_global_root: Some(".agents/skills"),
        skill_project_root: Some(".agents/skills"),
        mcp_global_path: Some(".copilot/mcp-config.json"),
        mcp_project_path: Some(".github/mcp.json"),
        mcp_kind: Some(McpConfigKind::McpServersJson),
        reload_hint: "Restart Copilot CLI.",
    },
    AgentClient {
        id: AgentClientId::Grok,
        display_name: "Grok Build",
        binaries: &["grok"],
        strong_global_markers: &[".grok/config.toml"],
        skill_global_root: None,
        skill_project_root: None,
        mcp_global_path: Some(".grok/config.toml"),
        mcp_project_path: Some(".grok/config.toml"),
        mcp_kind: Some(McpConfigKind::GrokToml),
        reload_hint: "Restart Grok Build.",
    },
    AgentClient {
        id: AgentClientId::Warp,
        display_name: "Warp",
        binaries: &["warp-terminal"],
        strong_global_markers: &[".warp/.mcp.json"],
        skill_global_root: None,
        skill_project_root: None,
        mcp_global_path: Some(".warp/.mcp.json"),
        mcp_project_path: Some(".warp/.mcp.json"),
        mcp_kind: Some(McpConfigKind::McpServersJson),
        reload_hint: "Reload Warp; project servers require session approval.",
    },
    AgentClient {
        id: AgentClientId::Zed,
        display_name: "Zed",
        binaries: &["zed"],
        strong_global_markers: &[".config/zed/settings.json"],
        skill_global_root: None,
        skill_project_root: None,
        mcp_global_path: None,
        mcp_project_path: Some(".zed/settings.json"),
        mcp_kind: Some(McpConfigKind::ZedContextServersJson),
        reload_hint: "Restart Zed or reload the workspace.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    struct FakeEnv {
        binaries: BTreeSet<String>,
        paths: BTreeSet<String>,
        home: String,
    }

    impl DetectionEnv for FakeEnv {
        fn has_binary(&self, name: &str) -> bool {
            self.binaries.contains(name)
        }

        fn path_exists(&self, path: &str) -> bool {
            self.paths.contains(path)
        }

        fn env(&self, _name: &str) -> Option<String> {
            None
        }

        fn home_dir(&self) -> Option<String> {
            Some(self.home.clone())
        }
    }

    #[test]
    fn ids_and_registry_are_one_to_one() {
        let ids = AgentClientId::all()
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 12);
        for id in ids {
            assert_eq!(id.entry().id, id);
        }
    }

    #[test]
    fn capabilities_are_independent_and_zed_is_project_only() {
        let antigravity = AgentClientId::Antigravity.entry();
        assert!(!antigravity.supports_skill(InstallScope::Global));
        assert!(antigravity.supports_mcp(InstallScope::Global));

        let zed = AgentClientId::Zed.entry();
        assert!(!zed.supports_mcp(InstallScope::Global));
        assert!(zed.supports_mcp(InstallScope::Project));
    }

    /// Build path strings the same way production detection does
    /// (`Path::join`), so `FakeEnv` matches on Windows and Unix separators.
    fn joined(home_or_root: &str, relative: &str) -> String {
        Path::new(home_or_root)
            .join(relative)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn detection_requires_binary_or_exact_marker() {
        let home = if cfg!(windows) {
            r"C:\Users\test"
        } else {
            "/home/test"
        };
        let generic = FakeEnv {
            binaries: BTreeSet::new(),
            paths: BTreeSet::from([joined(home, ".agents")]),
            home: home.to_string(),
        };
        assert!(!AgentClientId::Codex.entry().detected(&generic));
        assert!(!AgentClientId::Antigravity.entry().detected(&generic));

        let exact = FakeEnv {
            binaries: BTreeSet::new(),
            paths: BTreeSet::from([joined(home, ".codex/config.toml")]),
            home: home.to_string(),
        };
        assert!(AgentClientId::Codex.entry().detected(&exact));
    }

    #[test]
    fn project_mcp_detection_uses_the_exact_scoped_config_path() {
        let home = if cfg!(windows) {
            r"C:\Users\test"
        } else {
            "/home/test"
        };
        let workspace = if cfg!(windows) {
            r"C:\workspace"
        } else {
            "/workspace"
        };
        let other = if cfg!(windows) {
            r"C:\another-workspace"
        } else {
            "/another-workspace"
        };
        let env = FakeEnv {
            binaries: BTreeSet::new(),
            paths: BTreeSet::from([joined(workspace, ".mcp.json")]),
            home: home.to_string(),
        };

        assert!(AgentClientId::ClaudeCode.entry().detected_for_mcp(
            &env,
            InstallScope::Project,
            Path::new(workspace),
        ));
        assert!(!AgentClientId::ClaudeCode.entry().detected_for_mcp(
            &env,
            InstallScope::Project,
            Path::new(other),
        ));
    }

    /// CIB-237: skill-install output printed `C:\\Users\\dev\\.claude/skills`
    /// because the registry literals are `/`-separated and `Path::join` keeps
    /// them verbatim. Every registry path must come back natively separated.
    #[test]
    fn registry_paths_use_native_separators_for_every_client() {
        let home = tempfile::tempdir().expect("native home fixture");
        let root = home.path();
        for entry in AgentClientId::all() {
            for scope in [InstallScope::Global, InstallScope::Project] {
                if let Some(path) = entry.skill_root(scope, root) {
                    assert_native_separators(&path, entry.display_name, "skill root");
                }
                if let Some(path) = entry.mcp_path(scope, root) {
                    assert_native_separators(&path, entry.display_name, "mcp path");
                }
            }
        }
    }

    /// A path is natively separated when rebuilding it component-wise yields
    /// the same STRING.
    ///
    /// Comparing `PathBuf`s here would be useless: `Path` equality is
    /// component-wise and Windows accepts `/` as a separator, so
    /// `C:\\x\\.claude/skills` and `C:\\x\\.claude\\skills` compare equal on every
    /// platform. Only the string form can catch the mixed-separator bug.
    fn assert_native_separators(path: &Path, client: &str, what: &str) {
        let rebuilt: PathBuf = path.components().collect();
        assert_eq!(
            rebuilt.to_str(),
            path.to_str(),
            "{client} {what} has mixed separators: {}",
            path.display()
        );
    }
}
