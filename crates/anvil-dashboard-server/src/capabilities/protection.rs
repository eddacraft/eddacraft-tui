use std::path::Path;

use anvil_intercept::status::build_protection_claim_from_wire;
use anvil_intercept_proto::protocol::{
    ANVIL_STATUS_QUERY, ANVIL_WORKSPACE_STATUS, AssuranceState, StaleReason, WorkspaceAssurance,
    WorkspaceStatusRequest, WorkspaceStatusResponse,
};
use anvil_intercept_proto::status::{DaemonStatusV1, SaveTimeDriverStatusV1};
use anvil_kernel_types::{GateSnapshot, protection_claim::ProtectionClaim};
use serde_json::Value;

use crate::api::{
    AssuranceSummary, AttentionItem, DataGap, DataState, GateRunSummary,
    PROTECTION_OVERVIEW_SCHEMA, ProtectionOverview, SaveTimeSummary,
};
use crate::{Workspace, WorkspaceReadError};

const GATE_ARTEFACT: &str = ".anvil/gates.json";

struct LiveProtectionSnapshot {
    claim: ProtectionClaim,
    assurance: Option<AssuranceSummary>,
    save_time: SaveTimeSummary,
    observed_at_unix: Option<u64>,
    gaps: Vec<DataGap>,
}

pub fn load_protection_overview(workspace: &Workspace) -> ProtectionOverview {
    let mut overview = load_persisted_protection_overview(workspace);
    match query_live_protection(workspace.root()) {
        Ok(snapshot) => merge_live_snapshot(&mut overview, snapshot),
        Err(()) => overview.gaps.push(DataGap {
            component: "live-protection".to_owned(),
            reason: "The local daemon did not return a protection snapshot.".to_owned(),
        }),
    }
    overview
}

pub fn load_persisted_protection_overview(workspace: &Workspace) -> ProtectionOverview {
    let bytes = match workspace.read(Path::new(GATE_ARTEFACT)) {
        Ok(bytes) => bytes,
        Err(WorkspaceReadError::Missing { .. }) => {
            let mut overview = ProtectionOverview::unavailable(
                "No local gate artefact is available yet. Run an Anvil gate to populate this view.",
            );
            overview.gaps.extend(history_gaps());
            return overview;
        }
        Err(error) => {
            let mut overview = ProtectionOverview::unavailable(format!(
                "The local gate artefact could not be read: {error}"
            ));
            overview.gaps.extend(history_gaps());
            return overview;
        }
    };
    let gate: GateSnapshot = if let Ok(gate) = serde_json::from_slice(&bytes) {
        gate
    } else {
        let mut overview =
            ProtectionOverview::unavailable("The local gate artefact has an unsupported shape.");
        overview.gaps.extend(history_gaps());
        return overview;
    };

    let next_attention = gate.warning_list.first().map(|warning| AttentionItem {
        title: "Review latest gate attention item".to_owned(),
        detail: warning.message.clone(),
        evidence_id: None,
    });
    let warning_count = gate.warnings.parse().unwrap_or(gate.warning_list.len());
    let latest_run = Some(GateRunSummary {
        id: "latest-gate".to_owned(),
        result: gate.status,
        label: gate.status_label,
        score: Some(gate.score),
        warning_count,
        duration_seconds: gate.duration_seconds.parse().ok(),
        started_at: None,
        new_warning_count: None,
        changed_file_count: None,
    });
    let gaps = history_gaps();

    ProtectionOverview {
        schema_version: PROTECTION_OVERVIEW_SCHEMA.to_owned(),
        // `.anvil/gates.json` proves a gate result, not current daemon
        // participation. Keep the resource partial until a live claim exists.
        data_state: DataState::Partial,
        source_message: "Latest gate evidence is available; live save-time protection state is not present in this artefact.".to_owned(),
        claim: None,
        assurance: None,
        save_time: None,
        observed_at_unix: None,
        latest_run,
        recent_runs: Vec::new(),
        next_attention,
        warnings_state: DataState::Unavailable,
        warnings: Vec::new(),
        affected_files_state: DataState::Unavailable,
        affected_files: Vec::new(),
        gaps,
    }
}

fn query_live_protection(root: &Path) -> Result<LiveProtectionSnapshot, ()> {
    let status: DaemonStatusV1 =
        anvil_run::ipc::request(ANVIL_STATUS_QUERY, &Value::Null, "dash-status").map_err(|_| ())?;
    let claim = build_protection_claim_from_wire(&status, root);
    let save_time = save_time_summary(&status, root);

    let mut gaps = Vec::new();
    let assurance = serde_json::to_value(WorkspaceStatusRequest {
        workspace_root: root.to_string_lossy().into_owned(),
    })
    .ok()
    .and_then(|params| {
        anvil_run::ipc::request::<WorkspaceStatusResponse>(
            ANVIL_WORKSPACE_STATUS,
            &params,
            "dash-assurance",
        )
        .ok()
    })
    .map(|response| assurance_summary(response.workspace_assurance));
    if assurance.is_none() {
        gaps.push(DataGap {
            component: "workspace-assurance".to_owned(),
            reason: "The daemon did not return a workspace assurance snapshot.".to_owned(),
        });
    }

    Ok(LiveProtectionSnapshot {
        claim,
        assurance,
        save_time,
        observed_at_unix: (status.generated_at_unix != 0).then_some(status.generated_at_unix),
        gaps,
    })
}

fn merge_live_snapshot(overview: &mut ProtectionOverview, snapshot: LiveProtectionSnapshot) {
    overview.claim = Some(snapshot.claim);
    overview.assurance = snapshot.assurance;
    overview.save_time = Some(snapshot.save_time);
    overview.observed_at_unix = snapshot.observed_at_unix;
    overview
        .gaps
        .retain(|gap| gap.component != "live-protection");
    overview.gaps.extend(snapshot.gaps);
    // Retained diagnostics and affected-file history remain explicitly partial,
    // even when the current daemon state is available.
    overview.data_state = DataState::Partial;
    overview.source_message = if overview.latest_run.is_some() {
        "Live protection and latest gate evidence are available; retained diagnostics remain partial."
            .to_owned()
    } else {
        "Live protection evidence is available; no local gate snapshot is present.".to_owned()
    };
}

fn save_time_summary(status: &DaemonStatusV1, root: &Path) -> SaveTimeSummary {
    let driver_states = status
        .worktrees
        .iter()
        .filter(|worktree| worktree.worktree == root)
        .map(|worktree| worktree.save_time_driver)
        .collect::<Vec<_>>();
    let active = driver_states.contains(&SaveTimeDriverStatusV1::Attached);
    let failure_count = driver_states
        .iter()
        .filter(|state| **state == SaveTimeDriverStatusV1::Failed)
        .count();
    let state = if active {
        "attached"
    } else if failure_count > 0 {
        "failed"
    } else if driver_states.contains(&SaveTimeDriverStatusV1::Unknown) {
        "unknown"
    } else {
        "absent"
    };
    SaveTimeSummary {
        state: state.to_owned(),
        active,
        failure_count,
    }
}

fn assurance_summary(assurance: WorkspaceAssurance) -> AssuranceSummary {
    let (scanned_files, total_files) = assurance.scan_coverage.map_or((None, None), |coverage| {
        (Some(coverage.scanned_files), Some(coverage.total_files))
    });
    AssuranceSummary {
        state: assurance_state(assurance.state).to_owned(),
        reason: assurance.reason.map(stale_reason).map(str::to_owned),
        generation: assurance.generation,
        last_full_scan: assurance.last_full_scan,
        scanned_files,
        total_files,
    }
}

fn assurance_state(state: AssuranceState) -> &'static str {
    match state {
        AssuranceState::Clean => "clean",
        AssuranceState::Stale => "stale",
        AssuranceState::Pending => "pending",
        AssuranceState::Running => "running",
        AssuranceState::Bounded => "bounded",
        AssuranceState::Unavailable => "unavailable",
        AssuranceState::Unknown => "unknown",
    }
}

fn stale_reason(reason: StaleReason) -> &'static str {
    match reason {
        StaleReason::CrossFileResolutionNeeded => "cross-file-resolution-needed",
        StaleReason::Deleted => "deleted",
        StaleReason::Renamed => "renamed",
        StaleReason::SymlinkRetarget => "symlink-retarget",
        StaleReason::ConfigBoundaryPolicyEdit => "config-boundary-policy-edit",
        StaleReason::GitignoreScopeChange => "gitignore-scope-change",
        StaleReason::ImpactSetOverflow => "impact-set-overflow",
        StaleReason::WarmStateEvicted => "warm-state-evicted",
        StaleReason::ScanTimeout => "scan-timeout",
        StaleReason::DaemonAbsent => "daemon-absent",
        StaleReason::UnknownClass => "unknown-class",
        StaleReason::Unknown => "unknown",
    }
}

fn history_gaps() -> Vec<DataGap> {
    vec![
        DataGap {
            component: "retained-warning-history".to_owned(),
            reason: "The canonical gate snapshot records only summary attention items; retained diagnostic details are unavailable.".to_owned(),
        },
        DataGap {
            component: "affected-files".to_owned(),
            reason: "Affected files are unavailable without retained diagnostics.".to_owned(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::protection_claim::WorktreeClaimState;

    #[test]
    fn live_snapshot_replaces_unavailable_claim_without_overstating_history() {
        let mut overview = ProtectionOverview::unavailable("no persisted evidence");
        overview.gaps.push(DataGap {
            component: "live-protection".to_owned(),
            reason: "offline".to_owned(),
        });
        let snapshot = LiveProtectionSnapshot {
            claim: ProtectionClaim::new(WorktreeClaimState::PreWriteDaemon, vec![]),
            assurance: Some(AssuranceSummary {
                state: "clean".to_owned(),
                reason: None,
                generation: 3,
                last_full_scan: None,
                scanned_files: None,
                total_files: None,
            }),
            save_time: SaveTimeSummary {
                state: "attached".to_owned(),
                active: true,
                failure_count: 0,
            },
            observed_at_unix: Some(42),
            gaps: history_gaps(),
        };

        merge_live_snapshot(&mut overview, snapshot);

        assert!(overview.claim.is_some());
        assert!(overview.save_time.is_some_and(|save_time| save_time.active));
        assert_eq!(overview.observed_at_unix, Some(42));
        assert_eq!(overview.data_state, DataState::Partial);
        assert!(
            overview
                .gaps
                .iter()
                .all(|gap| gap.component != "live-protection")
        );
    }
}
