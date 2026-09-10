//! Persistent closing receipt (ADR-145 / JSIMP-005).
//!
//! Start, status and doctor consume the same named facts: project,
//! selected coverage, connected/pending client, policy mode, last proof
//! and the next command. Daily use is bare `anvil`. Demos stay isolated.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::diagnostic::{ActivationDiagnostic, ConfigStatus, McpTier};
use super::intent::{ClientChoice, IntegrationIntent};
use super::posture::SharedPostureFacts;
use super::render::repair_hint_for;
use super::state::ProtectionState;

/// Schema id persisted under `.anvil/closing-receipt.json`.
pub(crate) const SCHEMA_VERSION: &str = "anvil.closing-receipt.v1";

/// Daily next-step copy when setup is complete.
pub(crate) const DAILY_NEXT: &str = "run `anvil` for daily ensure";

const RECEIPT_DIR: &str = ".anvil";
const RECEIPT_FILE: &str = "closing-receipt.json";

/// Isolated save-time fixture bytes (same shape as the ADTRUST-006 recipe).
const SAVE_TIME_FIXTURE: &str = r#"const KEY = "AKIAQRSTUVWXYZ123456";"#;
const SAVE_TIME_FIXTURE_NAME: &str = ".anvil-smoke-test.ts";

/// How a client appears on the closing receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ClientReceipt {
    pub status: ClientReceiptStatus,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ClientReceiptStatus {
    Connected,
    Pending,
    Omitted,
}

impl ClientReceiptStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Pending => "pending",
            Self::Omitted => "omitted",
        }
    }
}

/// Outcome of one proof arm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProofArm {
    pub outcome: ProofKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProofKind {
    Demonstrated,
    HonestSkip,
    Failed,
}

/// Last proof recorded on the receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LastProof {
    pub mcp: ProofArm,
    pub save_time: ProofArm,
    pub at: String,
}

/// Named facts shared by start, status and doctor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ClosingReceipt {
    pub schema_version: String,
    pub project: String,
    pub selected_coverage: String,
    pub client: ClientReceipt,
    pub policy_mode: String,
    pub last_proof: LastProof,
    pub next: String,
}

/// Capture options for assembling a receipt.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CaptureOptions {
    /// Write `.anvil/closing-receipt.json` after assembly.
    pub persist: bool,
    /// Run the isolated save-time fixture (and record MCP evidence now).
    pub run_proofs: bool,
}

impl CaptureOptions {
    pub(crate) const fn mutating() -> Self {
        Self {
            persist: true,
            run_proofs: true,
        }
    }

    pub(crate) const fn verify() -> Self {
        Self {
            persist: false,
            run_proofs: true,
        }
    }

    pub(crate) const fn inspect() -> Self {
        Self {
            persist: false,
            run_proofs: false,
        }
    }
}

impl ClosingReceipt {
    /// Assemble a receipt from live diagnostic evidence.
    pub(crate) fn capture(
        root: &Path,
        diag: &ActivationDiagnostic,
        opts: CaptureOptions,
        next_override: Option<String>,
    ) -> Self {
        let home = crate::util::user_home_dir();
        let intent = IntegrationIntent::infer(root, home.as_deref());
        let (client, mcp_proof) = prove_mcp(diag, &intent);
        let save_time_proof = if opts.run_proofs {
            prove_save_time(root, diag)
        } else {
            load_persisted(root).map_or_else(
                || ProofArm {
                    outcome: ProofKind::HonestSkip,
                    detail: "not observed yet".to_string(),
                },
                |stored| stored.last_proof.save_time,
            )
        };
        let last_proof = LastProof {
            mcp: mcp_proof,
            save_time: save_time_proof,
            at: chrono::Utc::now().to_rfc3339(),
        };
        let next = next_override.unwrap_or_else(|| next_for(diag, &client));
        let receipt = Self {
            schema_version: SCHEMA_VERSION.to_string(),
            project: project_name(root),
            selected_coverage: coverage_line(diag),
            client,
            policy_mode: policy_mode_label(root),
            last_proof,
            next,
        };
        if opts.persist && !crate::install_root::project_writes_gated() {
            let _ = persist(&receipt, root);
        }
        receipt
    }

    /// Plain-text block ending with a newline.
    pub(crate) fn render_human(&self) -> String {
        let mut out = String::from("RECEIPT\n");
        let _ = writeln!(out, "  project: {}", self.project);
        let _ = writeln!(out, "  coverage: {}", self.selected_coverage);
        let _ = writeln!(out, "  client: {}", client_line(&self.client));
        let _ = writeln!(out, "  policy: {}", self.policy_mode);
        let _ = writeln!(out, "  last proof: {}", last_proof_line(&self.last_proof));
        let _ = writeln!(out, "  next: {}", self.next);
        out
    }

    /// Additive JSON object for status / start / doctor documents.
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "schema_version": self.schema_version,
            "project": self.project,
            "selected_coverage": self.selected_coverage,
            "client": {
                "status": self.client.status.label(),
                "names": self.client.names,
            },
            "policy_mode": self.policy_mode,
            "last_proof": {
                "mcp": {
                    "outcome": proof_kind_label(self.last_proof.mcp.outcome),
                    "detail": self.last_proof.mcp.detail,
                },
                "save_time": {
                    "outcome": proof_kind_label(self.last_proof.save_time.outcome),
                    "detail": self.last_proof.save_time.detail,
                },
                "at": self.last_proof.at,
            },
            "next": self.next,
        })
    }

    /// Collapsible TUI rows (same facts as [`Self::render_human`]).
    pub(crate) fn verdict_rows(&self) -> Vec<String> {
        vec![
            format!("project: {}", self.project),
            format!("coverage: {}", self.selected_coverage),
            format!("client: {}", client_line(&self.client)),
            format!("policy: {}", self.policy_mode),
            format!("last proof: {}", last_proof_line(&self.last_proof)),
            format!("next: {}", self.next),
        ]
    }
}

fn proof_kind_label(kind: ProofKind) -> &'static str {
    match kind {
        ProofKind::Demonstrated => "demonstrated",
        ProofKind::HonestSkip => "honest_skip",
        ProofKind::Failed => "failed",
    }
}

fn project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && *name != ".")
        .map_or_else(|| "this project".to_string(), ToOwned::to_owned)
}

fn coverage_line(diag: &ActivationDiagnostic) -> String {
    SharedPostureFacts::from_diagnostic(diag)
        .fact_lines()
        .join("; ")
}

fn client_line(client: &ClientReceipt) -> String {
    if client.names.is_empty() {
        return format!("none ({})", client.status.label());
    }
    format!("{} ({})", client.names.join(", "), client.status.label())
}

fn last_proof_line(proof: &LastProof) -> String {
    format!("{}; {}", proof.mcp.detail, proof.save_time.detail)
}

fn next_for(diag: &ActivationDiagnostic, client: &ClientReceipt) -> String {
    if let Some(hint) = repair_hint_for(diag) {
        return hint.to_string();
    }
    if matches!(diag.config, ConfigStatus::Absent) {
        return "run `anvil start` to activate this repo".to_string();
    }
    if diag.all_languages_unsupported {
        return "no supported languages — anvil has nothing to validate here".to_string();
    }
    if matches!(client.status, ClientReceiptStatus::Pending) {
        if let Some(name) = client.names.first() {
            return format!("restart {name} so MCP pre-write can attach");
        }
        return "restart your editor so MCP pre-write can attach".to_string();
    }
    if matches!(diag.protection_state(), ProtectionState::Error) {
        return "run `anvil doctor` for unresolved faults".to_string();
    }
    DAILY_NEXT.to_string()
}

fn policy_mode_label(root: &Path) -> String {
    use anvil_config::{RuleModes, discover, parse_file};

    match discover(root, ".anvil") {
        Ok(None) => {
            if root.join(".anvilrc").is_file() {
                "custom".to_string()
            } else {
                "defaults".to_string()
            }
        }
        Ok(Some(discovered)) => match parse_file(&discovered.path) {
            Ok(value) => match RuleModes::from_value(&value) {
                Ok(modes) if modes == RuleModes::default() => "defaults".to_string(),
                Ok(modes) => modes.summary(),
                Err(_) => "invalid".to_string(),
            },
            Err(_) => "invalid".to_string(),
        },
        Err(_) => "invalid".to_string(),
    }
}

fn prove_mcp(diag: &ActivationDiagnostic, intent: &IntegrationIntent) -> (ClientReceipt, ProofArm) {
    let mut connected = Vec::new();
    let mut pending = Vec::new();
    for (id, probe) in &diag.mcp {
        match probe.tier {
            McpTier::LiveValidation => connected.push(id.display_name().to_string()),
            McpTier::RestartHandshakeVerified | McpTier::RestartRequired => {
                pending.push(id.display_name().to_string());
            }
            _ => {}
        }
    }
    if !connected.is_empty() {
        return (
            ClientReceipt {
                status: ClientReceiptStatus::Connected,
                names: connected.clone(),
            },
            ProofArm {
                outcome: ProofKind::Demonstrated,
                detail: format!("live validation observed from {}", connected.join(", ")),
            },
        );
    }
    let handshake: Vec<String> = diag
        .mcp
        .iter()
        .filter(|(_, probe)| probe.tier == McpTier::RestartHandshakeVerified)
        .map(|(id, _)| id.display_name().to_string())
        .collect();
    if !handshake.is_empty() {
        return (
            ClientReceipt {
                status: ClientReceiptStatus::Pending,
                names: pending,
            },
            ProofArm {
                outcome: ProofKind::Demonstrated,
                detail: format!(
                    "initialize handshake verified for {} (restart still required)",
                    handshake.join(", ")
                ),
            },
        );
    }
    if !pending.is_empty() {
        return (
            ClientReceipt {
                status: ClientReceiptStatus::Pending,
                names: pending.clone(),
            },
            ProofArm {
                outcome: ProofKind::HonestSkip,
                detail: format!(
                    "MCP wired for {}; restart still required before live validation",
                    pending.join(", ")
                ),
            },
        );
    }
    if intent.mcp_omitted() {
        return (
            ClientReceipt {
                status: ClientReceiptStatus::Omitted,
                names: Vec::new(),
            },
            ProofArm {
                outcome: ProofKind::HonestSkip,
                detail: "MCP omitted — not a failure".to_string(),
            },
        );
    }
    let selected: Vec<String> = intent
        .clients
        .iter()
        .filter(|client| client.choice == ClientChoice::Selected)
        .map(|client| client.id.display_name().to_string())
        .collect();
    (
        ClientReceipt {
            status: ClientReceiptStatus::Pending,
            names: selected.clone(),
        },
        ProofArm {
            outcome: ProofKind::HonestSkip,
            detail: if selected.is_empty() {
                "no MCP client validation observed".to_string()
            } else {
                format!("selected {} not yet validated", selected.join(", "))
            },
        },
    )
}

fn prove_save_time(root: &Path, diag: &ActivationDiagnostic) -> ProofArm {
    if diag.all_languages_unsupported {
        return ProofArm {
            outcome: ProofKind::HonestSkip,
            detail: "no supported languages — save-time fixture would report nothing".to_string(),
        };
    }
    if !secret_detection_enabled(root) {
        return ProofArm {
            outcome: ProofKind::HonestSkip,
            detail: "secret-detection is not enabled in this project".to_string(),
        };
    }
    match run_isolated_save_time_fixture(root) {
        Ok(0) => ProofArm {
            outcome: ProofKind::Failed,
            detail: "isolated fixture: secret-detection reported no finding".to_string(),
        },
        Ok(findings) => ProofArm {
            outcome: ProofKind::Demonstrated,
            detail: format!("isolated fixture: secret-detection caught {findings} finding(s)"),
        },
        Err(error) => ProofArm {
            outcome: ProofKind::Failed,
            detail: format!("isolated fixture failed: {error}"),
        },
    }
}

fn secret_detection_enabled(root: &Path) -> bool {
    match crate::commands::gate::read_anvilrc_checks(root) {
        Ok(None) => true,
        Ok(Some(checks)) => checks.iter().any(|name| {
            crate::commands::check_catalog::canonical_check_name(name) == Some("secret-detection")
                || name == "secret-detection"
        }),
        Err(_) => false,
    }
}

/// Write a throwaway fixture outside the project and scan the saved file.
fn run_isolated_save_time_fixture(project_root: &Path) -> Result<usize, String> {
    use anvil_checks::secret::{SecretCheckConfig, scan_content};

    let dir = tempfile::TempDir::new().map_err(|error| error.to_string())?;
    if dir.path().starts_with(project_root) {
        return Err("demo fixture was not isolated from the project".to_string());
    }
    let path = dir.path().join(SAVE_TIME_FIXTURE_NAME);
    std::fs::write(&path, SAVE_TIME_FIXTURE).map_err(|error| error.to_string())?;
    let saved = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    if saved != SAVE_TIME_FIXTURE {
        return Err("saved fixture did not round-trip".to_string());
    }
    let findings = scan_content(
        &saved,
        SAVE_TIME_FIXTURE_NAME,
        &SecretCheckConfig::default(),
    );
    Ok(findings.len())
}

fn receipt_path(root: &Path) -> PathBuf {
    root.join(RECEIPT_DIR).join(RECEIPT_FILE)
}

fn persist(receipt: &ClosingReceipt, root: &Path) -> Result<(), String> {
    let dir = root.join(RECEIPT_DIR);
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = receipt_path(root);
    let body = serde_json::to_string_pretty(receipt).map_err(|error| error.to_string())?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".closing-receipt-")
        .suffix(".json.tmp")
        .tempfile_in(&dir)
        .map_err(|error| error.to_string())?;
    tmp.write_all(body.as_bytes())
        .map_err(|error| error.to_string())?;
    tmp.flush().map_err(|error| error.to_string())?;
    tmp.persist(&path)
        .map_err(|error| error.error.to_string())?;
    Ok(())
}

fn load_persisted(root: &Path) -> Option<ClosingReceipt> {
    let body = std::fs::read_to_string(receipt_path(root)).ok()?;
    let receipt: ClosingReceipt = serde_json::from_str(&body).ok()?;
    (receipt.schema_version == SCHEMA_VERSION).then_some(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activation::daemon_evidence::DaemonAttestation;
    use crate::activation::diagnostic::{McpClientId, WatchTier};
    use crate::activation::language_profile::RepoLanguageProfile;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn empty_diag() -> ActivationDiagnostic {
        ActivationDiagnostic {
            config: ConfigStatus::Valid,
            mcp: BTreeMap::new(),
            watch: WatchTier::NotRequested,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: RepoLanguageProfile::default(),
            daemon_attestation: DaemonAttestation::NotProbed,
            save_time_driver_attached: false,
        }
    }

    fn live_diag() -> ActivationDiagnostic {
        let mut diag = empty_diag();
        diag.mcp
            .insert(McpClientId::ClaudeCode, McpTier::LiveValidation.into());
        diag.save_time_driver_attached = true;
        diag
    }

    #[test]
    fn receipt_names_required_facts_and_daily_anvil() {
        let root = TempDir::new().unwrap();
        std::fs::write(root.path().join(".anvil.yaml"), "schema: 1\n").unwrap();
        let receipt =
            ClosingReceipt::capture(root.path(), &live_diag(), CaptureOptions::verify(), None);
        let human = receipt.render_human();
        assert!(human.starts_with("RECEIPT\n"), "{human}");
        assert!(human.contains("project:"), "{human}");
        assert!(human.contains("coverage:"), "{human}");
        assert!(human.contains("client:"), "{human}");
        assert!(human.contains("policy:"), "{human}");
        assert!(human.contains("last proof:"), "{human}");
        assert!(human.contains("next:"), "{human}");
        assert!(human.contains("Claude Code (connected)"), "{human}");
        assert!(human.contains(DAILY_NEXT), "{human}");
        assert!(!human.contains("anvil intercept ensure"), "{human}");
        assert!(!human.to_ascii_lowercase().contains("dashboard"), "{human}");
        assert!(!human.to_ascii_lowercase().contains("splash"), "{human}");
    }

    #[test]
    fn save_time_proof_uses_isolated_fixture() {
        let root = TempDir::new().unwrap();
        std::fs::write(root.path().join(".anvil.yaml"), "schema: 1\n").unwrap();
        let proof = prove_save_time(root.path(), &empty_diag());
        assert_eq!(proof.outcome, ProofKind::Demonstrated);
        assert!(
            proof.detail.contains("isolated fixture"),
            "{}",
            proof.detail
        );
        assert!(
            !root.path().join(SAVE_TIME_FIXTURE_NAME).exists(),
            "demo fixture must not land in the project"
        );
    }

    #[test]
    fn unsupported_languages_stay_honest() {
        let root = TempDir::new().unwrap();
        let mut diag = empty_diag();
        diag.all_languages_unsupported = true;
        let proof = prove_save_time(root.path(), &diag);
        assert_eq!(proof.outcome, ProofKind::HonestSkip);
        assert!(
            proof.detail.contains("no supported languages"),
            "{}",
            proof.detail
        );
        let receipt = ClosingReceipt::capture(root.path(), &diag, CaptureOptions::verify(), None);
        assert_eq!(receipt.last_proof.save_time.outcome, ProofKind::HonestSkip);
        let human = receipt.render_human();
        assert_eq!(human.matches("next:").count(), 1, "{human}");
        assert!(
            receipt.next.contains("language") || receipt.next.contains("anvil"),
            "{}",
            receipt.next
        );
    }

    #[test]
    fn pending_client_names_one_owner() {
        let mut diag = empty_diag();
        diag.mcp
            .insert(McpClientId::Cursor, McpTier::RestartRequired.into());
        let root = TempDir::new().unwrap();
        let receipt = ClosingReceipt::capture(root.path(), &diag, CaptureOptions::verify(), None);
        assert_eq!(receipt.client.status, ClientReceiptStatus::Pending);
        assert!(
            receipt.client.names.iter().any(|name| name == "Cursor"),
            "{:?}",
            receipt.client.names
        );
        let human = receipt.render_human();
        assert_eq!(human.matches("next:").count(), 1, "{human}");
        assert!(
            receipt.next.contains("restart") || receipt.next.contains("anvil"),
            "{}",
            receipt.next
        );
    }

    #[test]
    fn handshake_is_a_real_client_validation_action() {
        let mut diag = empty_diag();
        diag.mcp.insert(
            McpClientId::Cursor,
            McpTier::RestartHandshakeVerified.into(),
        );
        let intent = IntegrationIntent::infer(Path::new("."), None);
        let (client, proof) = prove_mcp(&diag, &intent);
        assert_eq!(client.status, ClientReceiptStatus::Pending);
        assert_eq!(proof.outcome, ProofKind::Demonstrated);
        assert!(proof.detail.contains("handshake"), "{}", proof.detail);
    }

    #[test]
    fn omitted_mcp_is_not_a_failure() {
        let root = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let diag = empty_diag();
        let intent = IntegrationIntent::infer(root.path(), Some(home.path()));
        let (client, proof) = prove_mcp(&diag, &intent);
        assert_eq!(client.status, ClientReceiptStatus::Omitted);
        assert_eq!(proof.outcome, ProofKind::HonestSkip);
        assert!(proof.detail.contains("omitted"), "{}", proof.detail);
    }

    #[test]
    fn persist_round_trip_keeps_last_proof_for_status() {
        let root = TempDir::new().unwrap();
        std::fs::write(root.path().join(".anvil.yaml"), "schema: 1\n").unwrap();
        let written =
            ClosingReceipt::capture(root.path(), &live_diag(), CaptureOptions::mutating(), None);
        assert!(receipt_path(root.path()).is_file());
        let loaded =
            ClosingReceipt::capture(root.path(), &live_diag(), CaptureOptions::inspect(), None);
        assert_eq!(
            loaded.last_proof.save_time.detail,
            written.last_proof.save_time.detail
        );
        assert_eq!(loaded.to_json()["project"], written.to_json()["project"]);
    }

    #[test]
    fn start_and_status_share_the_same_human_facts() {
        let root = TempDir::new().unwrap();
        std::fs::write(root.path().join(".anvil.yaml"), "schema: 1\n").unwrap();
        let start =
            ClosingReceipt::capture(root.path(), &live_diag(), CaptureOptions::mutating(), None);
        let status =
            ClosingReceipt::capture(root.path(), &live_diag(), CaptureOptions::inspect(), None);
        let start_human = start.render_human();
        let status_human = status.render_human();
        for needle in [
            "project:",
            "coverage:",
            "client:",
            "policy:",
            "last proof:",
            "next:",
        ] {
            assert!(start_human.contains(needle), "{start_human}");
            assert!(status_human.contains(needle), "{status_human}");
        }
        assert_eq!(start.project, status.project);
        assert_eq!(start.client, status.client);
        assert_eq!(start.selected_coverage, status.selected_coverage);
    }
}
