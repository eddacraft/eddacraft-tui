//! POSBRD-003/004: gather and render the protection posture board.
//!
//! Status renders the SETCON snapshot; it does not become a second resolver.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anvil_intercept_proto::enforcement_config::AnvilConfigFile;
use anvil_intercept_proto::status::{DaemonStatusV1, LastActionV1};
use anvil_l4::OnWarn;
use anvil_settings::{
    AcceptanceSource, AcceptanceVerb, EnforcementSource, GateSource, LastAction, POSTURE_SCALE,
    Posture, PostureInputs, PostureSnapshot, PostureSurface, RuntimeState, SurfaceActive,
    posture_snapshot,
};
use anvil_tui::surfaces::status::{StatusPostureBoard, StatusPostureRow};
use serde::Serialize;

use crate::policy_load;

/// Build the four-row board from workspace sources and optional daemon evidence.
#[must_use]
pub fn gather_posture_snapshot(root: &Path, daemon: Option<&DaemonStatusV1>) -> PostureSnapshot {
    posture_snapshot(&gather_posture_inputs(root, daemon))
}

#[must_use]
pub fn gather_posture_inputs(root: &Path, daemon: Option<&DaemonStatusV1>) -> PostureInputs {
    let declared = declared_enforcement(root);
    let intercept_user = daemon
        .and_then(|snapshot| snapshot.enforcement_mode.as_deref())
        .and_then(Posture::parse)
        .filter(|&mode| declared.is_none_or(|declared| mode > declared));
    PostureInputs {
        enforcement: EnforcementSource {
            declared,
            resolved: declared,
            intercept_user,
        },
        gate: GateSource {
            fail_on_warnings: fail_on_warnings_from_env(),
        },
        acceptance: load_acceptance(root),
        active: [
            unknown_active(),
            intercept_active(daemon),
            unknown_active(),
            unknown_active(),
        ],
        last_action: daemon.and_then(|snapshot| snapshot.last_action.as_ref().map(to_last_action)),
    }
}

fn unknown_active() -> SurfaceActive {
    SurfaceActive {
        value: None,
        state: RuntimeState::Unknown,
    }
}

fn intercept_active(daemon: Option<&DaemonStatusV1>) -> SurfaceActive {
    match daemon.and_then(|snapshot| snapshot.enforcement_mode.as_deref()) {
        Some(mode) => SurfaceActive {
            value: Some(mode.to_owned()),
            state: RuntimeState::Active,
        },
        None => unknown_active(),
    }
}

fn declared_enforcement(root: &Path) -> Option<Posture> {
    let content = std::fs::read_to_string(root.join(".anvil.yaml")).ok()?;
    let config: AnvilConfigFile = serde_yaml::from_str(&content).ok()?;
    config.enforcement.mode.as_deref().and_then(Posture::parse)
}

fn fail_on_warnings_from_env() -> Option<bool> {
    let raw = std::env::var("ANVIL_FAIL_ON_WARNINGS").ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(!matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    ))
}

fn load_acceptance(root: &Path) -> AcceptanceSource {
    match policy_load::load_policy(root) {
        Ok(Some(policy)) => {
            let verb = policy.branches.first().map(|branch| {
                if matches!(branch.on_warn, OnWarn::Reject) {
                    AcceptanceVerb::OnWarn
                } else {
                    AcceptanceVerb::OnBlock
                }
            });
            AcceptanceSource {
                present: true,
                verb,
            }
        }
        Ok(None) | Err(_) => AcceptanceSource {
            present: false,
            verb: None,
        },
    }
}

fn to_last_action(action: &LastActionV1) -> LastAction {
    LastAction {
        decision: action.decision.clone(),
        stage: action.stage.clone(),
        observed_at: unix_to_rfc3339(action.observed_at_unix),
    }
}

fn unix_to_rfc3339(unix: u64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(i64::try_from(unix).unwrap_or(0), 0)
        .map_or_else(
            || unix.to_string(),
            |ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        )
}

/// Plain sibling of Layers.
#[must_use]
pub fn render_plain_posture(snapshot: &PostureSnapshot, now: SystemTime) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    out.push_str("  Posture:\n");
    out.push_str("    surface         configured     resolved        active\n");
    for row in &snapshot.rows {
        let configured = row
            .configured
            .value
            .as_deref()
            .unwrap_or_else(|| row.id.absent_label());
        let resolved = row
            .resolved
            .value
            .as_deref()
            .unwrap_or_else(|| row.id.absent_label());
        let active = row
            .active
            .value
            .as_deref()
            .unwrap_or_else(|| runtime_state_label(row.active.state));
        let _ = writeln!(
            out,
            "    {:<15} {:<14} {:<15} {active}",
            row.id.label(),
            configured,
            resolved
        );
    }
    if let Some(action) = snapshot
        .rows
        .iter()
        .find(|row| row.id == PostureSurface::Intercept)
        .and_then(|row| row.last_action.as_ref())
    {
        let age = relative_age(&action.observed_at, now);
        match action.stage.as_deref() {
            Some(stage) => {
                let _ = writeln!(
                    out,
                    "    intercept last: {}  {stage}  {age}",
                    action.decision
                );
            }
            None => {
                let _ = writeln!(out, "    intercept last: {}  {age}", action.decision);
            }
        }
    }
    let _ = writeln!(
        out,
        "    scale: {} < {} < {} < {}",
        POSTURE_SCALE[0], POSTURE_SCALE[1], POSTURE_SCALE[2], POSTURE_SCALE[3]
    );
    let _ = writeln!(
        out,
        "           {} ≈ {} ≈ {}",
        POSTURE_SCALE[1],
        snapshot.mapping.warn_equivalents[0],
        snapshot.mapping.warn_equivalents[1]
    );
    let _ = writeln!(
        out,
        "           {} ≈ {} ≈ {}   (block → {})",
        POSTURE_SCALE[3],
        snapshot.mapping.interrupt_equivalents[0],
        snapshot.mapping.interrupt_equivalents[1],
        snapshot.mapping.block_alias
    );
    out
}

fn runtime_state_label(state: RuntimeState) -> &'static str {
    match state {
        RuntimeState::Unknown => "unknown",
        RuntimeState::Stale => "stale",
        RuntimeState::Failed => "failed",
        RuntimeState::Drift => "drift",
        RuntimeState::Active => "active",
    }
}

fn relative_age(observed_at: &str, now: SystemTime) -> String {
    let Some(parsed) = chrono::DateTime::parse_from_rfc3339(observed_at)
        .ok()
        .map(|ts| ts.with_timezone(&chrono::Utc))
    else {
        return observed_at.to_owned();
    };
    let observed = UNIX_EPOCH + Duration::from_secs(u64::try_from(parsed.timestamp()).unwrap_or(0));
    let age = now.duration_since(observed).unwrap_or(Duration::ZERO);
    format!("{} ago", format_age(age))
}

fn format_age(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m", s / 60)
    } else if s < 86400 {
        format!("{}h", s / 3600)
    } else {
        format!("{}d", s / 86400)
    }
}

/// Display-ready TUI board. Status renders; it does not resolve.
#[must_use]
pub fn tui_board(snapshot: &PostureSnapshot, now: SystemTime) -> StatusPostureBoard {
    let last_action = snapshot
        .rows
        .iter()
        .find(|row| row.id == PostureSurface::Intercept)
        .and_then(|row| row.last_action.as_ref())
        .map(|action| match action.stage.as_deref() {
            Some(stage) => format!(
                "intercept last: {}  {stage}  {}",
                action.decision,
                relative_age(&action.observed_at, now)
            ),
            None => format!(
                "intercept last: {}  {}",
                action.decision,
                relative_age(&action.observed_at, now)
            ),
        });
    StatusPostureBoard {
        rows: snapshot
            .rows
            .iter()
            .map(|row| StatusPostureRow {
                surface: row.id.label().to_owned(),
                configured: row
                    .configured
                    .value
                    .clone()
                    .unwrap_or_else(|| row.id.absent_label().to_owned()),
                resolved: row
                    .resolved
                    .value
                    .clone()
                    .unwrap_or_else(|| row.id.absent_label().to_owned()),
                active: row
                    .active
                    .value
                    .clone()
                    .unwrap_or_else(|| runtime_state_label(row.active.state).to_owned()),
            })
            .collect(),
        last_action,
        scale_lines: vec![
            format!(
                "scale: {} < {} < {} < {}",
                POSTURE_SCALE[0], POSTURE_SCALE[1], POSTURE_SCALE[2], POSTURE_SCALE[3]
            ),
            format!(
                "warn ≈ {} ≈ {}",
                snapshot.mapping.warn_equivalents[0], snapshot.mapping.warn_equivalents[1]
            ),
            format!(
                "interrupt ≈ {} ≈ {}   (block → {})",
                snapshot.mapping.interrupt_equivalents[0],
                snapshot.mapping.interrupt_equivalents[1],
                snapshot.mapping.block_alias
            ),
        ],
    }
}

#[derive(Serialize)]
pub struct PostureJson {
    pub scale: [&'static str; 4],
    pub rows: Vec<PostureRowJson>,
    pub mapping: PostureMappingJson,
}

#[derive(Serialize)]
pub struct PostureRowJson {
    pub id: &'static str,
    pub verbs: &'static str,
    pub configured: PostureCellJson,
    pub resolved: PostureCellJson,
    pub active: PostureActiveJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action: Option<LastActionJson>,
}

#[derive(Serialize)]
pub struct PostureCellJson {
    pub value: Option<String>,
    pub source: &'static str,
}

#[derive(Serialize)]
pub struct PostureActiveJson {
    pub value: Option<String>,
    pub state: RuntimeState,
}

#[derive(Serialize)]
pub struct LastActionJson {
    pub decision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    pub observed_at: String,
}

#[derive(Serialize)]
pub struct PostureMappingJson {
    pub scale: [&'static str; 4],
    pub warn: [&'static str; 2],
    pub interrupt: [&'static str; 2],
    pub block_alias: &'static str,
}

#[must_use]
pub fn posture_json(snapshot: &PostureSnapshot) -> PostureJson {
    PostureJson {
        scale: snapshot.scale,
        rows: snapshot
            .rows
            .iter()
            .map(|row| PostureRowJson {
                id: row.id.id(),
                verbs: match row.verbs {
                    anvil_settings::VerbFamily::Ladder => "ladder",
                    anvil_settings::VerbFamily::Native => "native",
                },
                configured: PostureCellJson {
                    value: row.configured.value.clone(),
                    source: row.configured.source,
                },
                resolved: PostureCellJson {
                    value: row.resolved.value.clone(),
                    source: row.resolved.source,
                },
                active: PostureActiveJson {
                    value: row.active.value.clone(),
                    state: row.active.state,
                },
                last_action: row.last_action.as_ref().map(|action| LastActionJson {
                    decision: action.decision.clone(),
                    stage: action.stage.clone(),
                    observed_at: action.observed_at.clone(),
                }),
            })
            .collect(),
        mapping: PostureMappingJson {
            scale: snapshot.mapping.scale,
            warn: snapshot.mapping.warn_equivalents,
            interrupt: snapshot.mapping.interrupt_equivalents,
            block_alias: snapshot.mapping.block_alias,
        },
    }
}

#[cfg(test)]
mod status_posture_tests {
    use super::*;
    use anvil_intercept_proto::status::{HealthStateV1, IpcStateV1, LatencyMidEditMapV1};

    fn empty_daemon() -> DaemonStatusV1 {
        DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 1,
                version: "test".into(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 0,
            enforcement_mode: None,
            last_action: None,
        }
    }

    #[test]
    fn status_posture_absent_key_splits_mcp_interrupt_and_intercept_warn() {
        let dir = tempfile::tempdir().expect("tempdir");
        let snapshot = gather_posture_snapshot(dir.path(), None);
        let rendered = render_plain_posture(&snapshot, SystemTime::now());
        assert!(
            rendered.contains("Posture:"),
            "sibling section must be named:\n{rendered}"
        );
        assert!(
            rendered.contains("mcp pre-write") && rendered.contains("interrupt"),
            "absent key resolves MCP to interrupt:\n{rendered}"
        );
        assert!(
            rendered.contains("intercept") && rendered.contains("warn"),
            "absent key resolves intercept to warn:\n{rendered}"
        );
        assert!(
            rendered.contains("unknown"),
            "active is unknown without evidence:\n{rendered}"
        );
        assert!(
            !rendered.contains("anvil settings"),
            "must not claim anvil settings:\n{rendered}"
        );
    }

    #[test]
    fn status_posture_json_documents_four_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let snapshot = gather_posture_snapshot(dir.path(), None);
        let json = serde_json::to_value(posture_json(&snapshot)).expect("json");
        assert_eq!(json["scale"][0], "off");
        assert_eq!(json["rows"][0]["id"], "mcp_pre_write");
        assert_eq!(json["rows"][1]["id"], "intercept");
        assert_eq!(json["rows"][2]["id"], "gate");
        assert_eq!(json["rows"][3]["id"], "acceptance");
        assert!(
            json["rows"][1]["last_action"].is_null()
                || json["rows"][1].get("last_action").is_none()
        );
    }

    #[test]
    fn status_posture_daemon_mode_is_intercept_active_not_configured() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut daemon = empty_daemon();
        daemon.enforcement_mode = Some("warn".into());
        let snapshot = gather_posture_snapshot(dir.path(), Some(&daemon));
        let intercept = snapshot
            .rows
            .iter()
            .find(|row| row.id == PostureSurface::Intercept)
            .expect("intercept row");
        assert!(intercept.configured.value.is_none());
        assert_eq!(intercept.resolved.value.as_deref(), Some("warn"));
        assert_eq!(intercept.active.value.as_deref(), Some("warn"));
        assert_eq!(intercept.active.state, RuntimeState::Active);
    }

    #[test]
    fn status_posture_last_action_omitted_when_unattested_even_with_fences() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut daemon = empty_daemon();
        daemon.fences = vec![anvil_intercept_proto::status::FenceStateV1 {
            worktree: dir.path().to_path_buf(),
            reason: "secret-detection".into(),
            fenced_at_unix: 1_700_000_000,
        }];
        let snapshot = gather_posture_snapshot(dir.path(), Some(&daemon));
        assert!(
            snapshot.rows.iter().all(|row| row.last_action.is_none()),
            "fences alone must not synthesise last-action"
        );
        let rendered = render_plain_posture(&snapshot, SystemTime::now());
        assert!(
            !rendered.contains("intercept last:"),
            "unattested last-action must omit:\n{rendered}"
        );
    }

    #[test]
    fn status_posture_last_action_renders_when_attested() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut daemon = empty_daemon();
        daemon.last_action = Some(LastActionV1 {
            decision: "interrupt".into(),
            stage: Some("sigkill".into()),
            observed_at_unix: 1_700_000_000,
        });
        let snapshot = gather_posture_snapshot(dir.path(), Some(&daemon));
        let rendered =
            render_plain_posture(&snapshot, UNIX_EPOCH + Duration::from_secs(1_700_000_012));
        assert!(
            rendered.contains("intercept last:")
                && rendered.contains("interrupt")
                && rendered.contains("sigkill"),
            "attested last-action must render:\n{rendered}"
        );
    }
}
