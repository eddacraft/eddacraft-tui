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
    AffectedFile, AssuranceSummary, AttentionItem, DataGap, DataState, GateCheckSummary,
    GateRunSummary, PROTECTION_OVERVIEW_SCHEMA, ProtectionOverview, SaveTimeSummary,
    WarningSummary,
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

    let warnings = map_gate_warnings(&gate);
    let checks = map_gate_checks(&gate);
    let warning_count = gate.warnings.parse().unwrap_or(warnings.len());
    let latest_run = GateRunSummary {
        id: "latest-gate".to_owned(),
        result: gate.status,
        label: gate.status_label,
        score: Some(gate.score),
        warning_count,
        duration_seconds: gate.duration_seconds.parse().ok(),
        started_at: None,
        new_warning_count: None,
        changed_file_count: None,
        checks,
    };
    let next_attention = warnings.first().map(|warning| AttentionItem {
        title: "Review latest gate attention item".to_owned(),
        detail: warning.message.clone(),
        evidence_id: Some(warning.id.clone()),
    });
    let affected_files = map_affected_files(&warnings);
    // A successfully parsed gate artefact proves these latest-snapshot
    // collections are available, including when a clean run yields no rows.
    // They remain partial because no retained multi-run diagnostics exist.
    let warnings_state = DataState::Partial;
    let affected_files_state = DataState::Partial;
    let mut gaps = history_gaps();
    gaps.retain(|gap| gap.component != "retained-warning-history");
    gaps.push(DataGap {
        component: "retained-warning-history".to_owned(),
        reason: "Only the latest gate snapshot attention items are available; multi-run retained diagnostics are not.".to_owned(),
    });
    gaps.retain(|gap| gap.component != "affected-files");
    gaps.push(DataGap {
        component: "affected-files".to_owned(),
        reason: "Affected files are derived from the latest gate snapshot only.".to_owned(),
    });

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
        latest_run: Some(latest_run.clone()),
        // Single latest run only — no retained multi-run history store.
        recent_runs: vec![latest_run],
        next_attention,
        warnings_state,
        warnings,
        affected_files_state,
        affected_files,
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

fn map_gate_checks(gate: &GateSnapshot) -> Vec<GateCheckSummary> {
    gate.check_rows
        .iter()
        .filter_map(|row| {
            let name = row.first()?.clone();
            let status = row.get(1).cloned().unwrap_or_else(|| "unknown".to_owned());
            let score = row.get(2).cloned().filter(|value| !value.is_empty());
            let message = row.get(3).cloned().unwrap_or_default();
            Some(GateCheckSummary {
                name,
                status,
                score,
                message,
            })
        })
        .collect()
}

fn map_gate_warnings(gate: &GateSnapshot) -> Vec<WarningSummary> {
    let mut warnings = Vec::new();
    for (index, warning) in gate.warning_list.iter().enumerate() {
        warnings.push(warning_from_gate_attention(
            index,
            warning.severity.as_str(),
            &warning.message,
        ));
    }
    // Prefer structured antipattern lines when the gate row embeds them and the
    // attention list is only a short summary.
    if let Some(antipattern_row) = gate
        .check_rows
        .iter()
        .find(|row| row.first().is_some_and(|name| name.contains("antipattern")))
    {
        let detail = antipattern_row.get(3).map_or("", String::as_str);
        let parsed = parse_antipattern_lines(detail);
        if parsed.len() > warnings.len() {
            warnings = parsed;
        }
    }
    warnings
}

fn warning_from_gate_attention(index: usize, severity: &str, message: &str) -> WarningSummary {
    let (file_path, line) = parse_location(message);
    let (rule, category) = split_rule_category(message);
    let pattern = extract_pattern_id(message).unwrap_or_default();
    WarningSummary {
        id: format!("latest-gate-warning-{index}"),
        severity: normalise_severity(severity),
        category,
        message: message.to_owned(),
        file_path,
        age_label: "Latest gate".to_owned(),
        evidence_id: format!("latest-gate-warning-{index}"),
        rule,
        line,
        explanation: message.to_owned(),
        matched_pattern: pattern,
        evidence_excerpt: Vec::new(),
    }
}

fn parse_antipattern_lines(detail: &str) -> Vec<WarningSummary> {
    detail
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.contains('[') {
                return None;
            }
            let pattern = extract_pattern_id(trimmed)?;
            let (file_path, line_no) = parse_location(trimmed);
            let severity = "medium".to_owned();
            let id = format!(
                "latest-gate-pattern-{}-{}",
                pattern,
                line_no.map_or_else(|| format!("entry-{}", index + 1), |n| format!("line-{n}"))
            );
            Some(WarningSummary {
                id: id.clone(),
                severity,
                category: "anti-pattern".to_owned(),
                message: trimmed.to_owned(),
                file_path,
                age_label: "Latest gate".to_owned(),
                evidence_id: id,
                rule: pattern.clone(),
                line: line_no,
                explanation: format!("Anti-pattern {pattern} matched in the latest gate scan."),
                matched_pattern: pattern,
                evidence_excerpt: Vec::new(),
            })
        })
        .collect()
}

fn map_affected_files(warnings: &[WarningSummary]) -> Vec<AffectedFile> {
    use std::collections::BTreeMap;
    let mut by_path: BTreeMap<String, AffectedFile> = BTreeMap::new();
    for warning in warnings {
        let Some(path) = warning.file_path.clone() else {
            continue;
        };
        let entry = by_path.entry(path.clone()).or_insert_with(|| AffectedFile {
            path,
            highest_severity: warning.severity.clone(),
            warning_count: 0,
            first_seen: warning.age_label.clone(),
            last_seen: warning.age_label.clone(),
            warning_id: warning.id.clone(),
        });
        entry.warning_count += 1;
        if severity_rank(&warning.severity) > severity_rank(&entry.highest_severity) {
            entry.highest_severity.clone_from(&warning.severity);
            entry.warning_id.clone_from(&warning.id);
        }
    }
    by_path.into_values().collect()
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "high" | "error" => 3,
        "medium" | "warn" | "warning" => 2,
        "low" | "info" => 1,
        _ => 0,
    }
}

fn normalise_severity(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "error" | "high" | "fail" | "failed" => "high".to_owned(),
        "warn" | "warning" | "medium" => "medium".to_owned(),
        "info" | "low" | "note" => "low".to_owned(),
        other => other.to_owned(),
    }
}

fn split_rule_category(message: &str) -> (String, String) {
    if let Some((head, _)) = message.split_once(':') {
        let rule = head.trim().to_owned();
        if !rule.is_empty() && !rule.contains('/') && rule.len() < 64 {
            return (rule.clone(), rule);
        }
    }
    if let Some(pattern) = extract_pattern_id(message) {
        return (pattern.clone(), "anti-pattern".to_owned());
    }
    ("gate-warning".to_owned(), "gate".to_owned())
}

fn parse_location(message: &str) -> (Option<String>, Option<usize>) {
    // Match path:line tokens such as src/config.ts:18
    for token in message.split_whitespace() {
        let token = token.trim_matches(|c: char| c == ',' || c == ';' || c == ')');
        if let Some((path, line)) = token.rsplit_once(':')
            && (path.contains('/') || path.contains('.'))
            && let Ok(line_no) = line.parse::<usize>()
        {
            return (Some(path.to_owned()), Some(line_no));
        }
    }
    (None, None)
}

fn extract_pattern_id(message: &str) -> Option<String> {
    let start = message.find('[')?;
    let end = message[start + 1..].find(']')? + start + 1;
    let candidate = &message[start + 1..end];
    if candidate
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && candidate.contains('-')
    {
        Some(candidate.to_owned())
    } else {
        None
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

    #[test]
    fn parsed_antipattern_ids_are_short_unique_and_location_based() {
        let warnings = parse_antipattern_lines(
            "[PAT-001] src/very/long/path/that/does/not/belong/in/an/id.rs:12 first\n\
             [PAT-001] src/very/long/path/that/does/not/belong/in/an/id.rs:24 second",
        );

        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].id, "latest-gate-pattern-PAT-001-line-12");
        assert_eq!(warnings[1].id, "latest-gate-pattern-PAT-001-line-24");
        assert_eq!(warnings[0].evidence_id, warnings[0].id);
        assert_ne!(warnings[0].id, warnings[1].id);
        assert!(warnings.iter().all(|warning| warning.id.len() <= 128));
    }
}
