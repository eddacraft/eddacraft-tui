//! Remembered integration intent (ADR-145 / JSIMP-004).
//!
//! Client, scope, executable and optional protection choices have one
//! durable owner, inferred from existing owned artefacts (project config,
//! ADR-044 MCP entries, hook/workflow presence). There is no
//! decline-preference database (ADR-114).

use std::path::Path;

use super::diagnostic::{ConfigStatus, McpClientId, config_status};
use super::mcp_client::{AnvilEntry, ConfigScope, DriftClass};
use super::orchestrator::install::{Candidate, collect_candidates};

/// How anvil treats one client on the daily ensure path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientChoice {
    /// Owned anvil entry — restore quietly on bare ensure.
    Selected,
    /// Owned but operator-disabled (`disabled: true` / `enabled: false`).
    Disabled,
    /// `NotPresent` — omitted or declined; not a failure. Reconsider via start.
    Omitted,
    /// Foreign / unsafe entry — leave alone.
    Foreign,
}

/// One client's durable choice, inferred from on-disk artefacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientIntent {
    pub id: McpClientId,
    pub scope: ConfigScope,
    pub choice: ClientChoice,
    pub executable: Option<String>,
}

/// Project-wide remembered integration intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationIntent {
    pub config: ConfigStatus,
    pub clients: Vec<ClientIntent>,
    pub hooks_present: bool,
    pub workflows_present: bool,
}

impl IntegrationIntent {
    pub(crate) fn infer(root: &Path, home: Option<&Path>) -> Self {
        Self::infer_with_entry(root, home, &AnvilEntry::preferred_stdio())
    }

    pub(crate) fn infer_with_entry(root: &Path, home: Option<&Path>, fresh: &AnvilEntry) -> Self {
        let candidates = collect_candidates(root, home, fresh);
        let clients = candidates
            .iter()
            .map(client_intent_from_candidate)
            .collect();
        Self {
            config: config_status(root),
            clients,
            hooks_present: crate::commands::hooks::activation_hooks_active(root).unwrap_or(false),
            workflows_present: anvil_workflow_present(root),
        }
    }

    pub(crate) fn selected_or_disabled(&self) -> impl Iterator<Item = &ClientIntent> {
        self.clients.iter().filter(|client| {
            matches!(
                client.choice,
                ClientChoice::Selected | ClientChoice::Disabled
            )
        })
    }

    pub(crate) fn has_selected_mcp(&self) -> bool {
        self.clients
            .iter()
            .any(|client| client.choice == ClientChoice::Selected)
    }

    /// True when no owned MCP entry exists (declined or never installed).
    pub(crate) fn mcp_omitted(&self) -> bool {
        self.selected_or_disabled().next().is_none()
    }
}

fn client_intent_from_candidate(candidate: &Candidate) -> ClientIntent {
    let existing = candidate
        .parsed
        .as_ref()
        .and_then(|parsed| parsed.existing_entry.as_ref());
    let disabled = existing.is_some_and(entry_is_disabled);
    let executable = existing.and_then(entry_command);
    let choice = match &candidate.drift {
        DriftClass::NotPresent => ClientChoice::Omitted,
        DriftClass::UnsafeDrift { .. } => ClientChoice::Foreign,
        DriftClass::UpToDate
        | DriftClass::SafeDrift { .. }
        | DriftClass::ExplicitOverride { .. } => {
            if disabled {
                ClientChoice::Disabled
            } else {
                ClientChoice::Selected
            }
        }
    };
    ClientIntent {
        id: candidate.id,
        scope: candidate.scope,
        choice,
        executable,
    }
}

fn entry_is_disabled(entry: &serde_json::Value) -> bool {
    entry.get("disabled").and_then(serde_json::Value::as_bool) == Some(true)
        || entry.get("enabled").and_then(serde_json::Value::as_bool) == Some(false)
}

fn entry_command(entry: &serde_json::Value) -> Option<String> {
    match entry.get("command") {
        Some(serde_json::Value::String(command)) => Some(command.clone()),
        Some(serde_json::Value::Array(parts)) => parts
            .first()
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        _ => None,
    }
}

fn anvil_workflow_present(root: &Path) -> bool {
    let dir = root.join(".github/workflows");
    ["anvil.yml", "anvil-audit.yml"]
        .iter()
        .any(|name| dir.join(name).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activation::orchestrator::install::ensure_existing_mcp_entries;
    use std::fs;
    use tempfile::TempDir;

    fn write_cursor_entry(dir: &Path, body: &str) {
        let path = dir.join(".cursor/mcp.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn cursor_managed(command: &str, disabled: bool) -> String {
        format!(
            r#"{{
  "mcpServers": {{
    "anvil": {{
      "command": "{command}",
      "args": ["mcp", "serve", "--stdio"],
      "disabled": {disabled}
    }}
  }}
}}"#
        )
    }

    #[test]
    fn empty_home_is_omitted_not_failed() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let intent = IntegrationIntent::infer(ws.path(), Some(home.path()));
        assert!(intent.mcp_omitted());
        assert!(!intent.has_selected_mcp());
        assert!(
            intent
                .clients
                .iter()
                .all(|client| client.choice == ClientChoice::Omitted)
        );
    }

    #[test]
    fn owned_cursor_entry_is_selected_with_scope_and_executable() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        write_cursor_entry(home.path(), &cursor_managed("anvil", false));
        fs::write(ws.path().join(".anvil.yaml"), "schema: 1\n").unwrap();

        let intent = IntegrationIntent::infer(ws.path(), Some(home.path()));
        let cursor = intent
            .clients
            .iter()
            .find(|client| client.id == McpClientId::Cursor)
            .expect("cursor intent");
        assert_eq!(cursor.choice, ClientChoice::Selected);
        assert_eq!(cursor.scope, ConfigScope::Global);
        assert_eq!(cursor.executable.as_deref(), Some("anvil"));
        assert!(intent.has_selected_mcp());
        assert!(!intent.mcp_omitted());
        assert_eq!(intent.config, ConfigStatus::Valid);
    }

    #[test]
    fn disabled_owned_entry_is_not_omission_or_failure() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        write_cursor_entry(home.path(), &cursor_managed("anvil", true));

        let intent = IntegrationIntent::infer(ws.path(), Some(home.path()));
        let cursor = intent
            .clients
            .iter()
            .find(|client| client.id == McpClientId::Cursor)
            .expect("cursor intent");
        assert_eq!(cursor.choice, ClientChoice::Disabled);
        assert!(!intent.has_selected_mcp());
        assert!(!intent.mcp_omitted());
    }

    #[test]
    fn repeat_ensure_restores_selected_coverage_without_rewriting_healthy_entries() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let path = home.path().join(".cursor/mcp.json");
        write_cursor_entry(home.path(), &cursor_managed("anvil", false));
        let before = fs::read(&path).unwrap();

        let first = ensure_existing_mcp_entries(
            ws.path(),
            Some(home.path()),
            &AnvilEntry::preferred_stdio(),
        );
        assert!(first.managed >= 1);
        assert_eq!(first.absent_for_recovery, 0);
        assert_eq!(
            fs::read(&path).unwrap(),
            before,
            "healthy ensure must not rewrite"
        );

        let second = ensure_existing_mcp_entries(
            ws.path(),
            Some(home.path()),
            &AnvilEntry::preferred_stdio(),
        );
        assert!(second.managed >= 1);
        assert_eq!(fs::read(&path).unwrap(), before);

        let intent = IntegrationIntent::infer(ws.path(), Some(home.path()));
        assert!(intent.has_selected_mcp());
    }

    #[test]
    fn adding_a_second_client_keeps_the_first_selected() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        write_cursor_entry(home.path(), &cursor_managed("anvil", false));
        let claude = home.path().join(".claude.json");
        fs::write(
            &claude,
            r#"{
  "mcpServers": {
    "anvil": {
      "command": "anvil",
      "args": ["mcp", "serve", "--stdio"]
    }
  }
}"#,
        )
        .unwrap();

        let intent = IntegrationIntent::infer(ws.path(), Some(home.path()));
        let selected: Vec<_> = intent
            .selected_or_disabled()
            .map(|client| client.id)
            .collect();
        assert!(selected.contains(&McpClientId::Cursor), "{selected:?}");
        assert!(selected.contains(&McpClientId::ClaudeCode), "{selected:?}");
        assert!(
            fs::read_to_string(home.path().join(".cursor/mcp.json"))
                .unwrap()
                .contains("anvil")
        );
        assert!(claude.exists());
    }

    #[test]
    fn hooks_and_workflows_are_inferred_from_artefacts() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let empty = IntegrationIntent::infer(ws.path(), Some(home.path()));
        assert!(!empty.hooks_present);
        assert!(!empty.workflows_present);

        let workflow_dir = ws.path().join(".github/workflows");
        fs::create_dir_all(&workflow_dir).unwrap();
        fs::write(workflow_dir.join("anvil.yml"), "name: anvil\n").unwrap();
        let with_workflow = IntegrationIntent::infer(ws.path(), Some(home.path()));
        assert!(with_workflow.workflows_present);
    }
}
