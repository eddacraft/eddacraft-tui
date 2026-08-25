//! Runtime attestation and evidence trust (SETCON-006 / ADR-132).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{EvidenceMode, SettingKey};

/// Transport that produced the evidence. First release: daemon RPC only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceChannel {
    DaemonRpc,
    /// Rejected by the classifier; cannot produce `active`.
    Untrusted,
}

/// Catalogue trust requirement for accepting evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTrust {
    None,
    DaemonAttested,
}

/// Mutually exclusive runtime state (ADR-132 §2). Not `invalid`/`locked`/
/// `pending activation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeState {
    Unknown,
    Stale,
    Failed,
    Drift,
    Active,
}

/// Structured runtime evidence from a responsible component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    pub component_id: String,
    pub instance_id: String,
    pub component_version: String,
    pub channel: EvidenceChannel,
    pub trust: EvidenceTrust,
    pub keys: Vec<SettingKey>,
    pub active_value: Option<Value>,
    pub classified_digest: Option<String>,
    pub conformance: Option<bool>,
    pub applied_revision: String,
    pub observed_at: String,
    pub valid_until: Option<String>,
    pub restart_required: bool,
    pub failure: Option<String>,
    pub disconnected: bool,
}

/// Inputs the classifier needs besides the attestation itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifyInput<'a> {
    pub key: &'a str,
    pub evidence_mode: EvidenceMode,
    pub required_owner: Option<&'a str>,
    pub required_trust: EvidenceTrust,
    pub resolved_value: Option<&'a Value>,
    pub resolved_revision: &'a str,
    pub now: &'a str,
}

/// Classify one evidence-bearing key. Total: exactly one [`RuntimeState`].
#[must_use]
pub fn classify_runtime_state(
    attestation: Option<&Attestation>,
    input: &ClassifyInput<'_>,
) -> RuntimeState {
    if input.evidence_mode == EvidenceMode::None {
        return RuntimeState::Unknown;
    }
    let Some(att) = attestation else {
        return RuntimeState::Unknown;
    };
    if !evidence_accepted(att, input) {
        return RuntimeState::Unknown;
    }
    if att.disconnected
        || att.applied_revision != input.resolved_revision
        || expired(att.valid_until.as_deref(), input.now)
    {
        return RuntimeState::Stale;
    }
    if att.failure.is_some() || att.conformance == Some(false) {
        return RuntimeState::Failed;
    }
    match input.evidence_mode {
        EvidenceMode::None => RuntimeState::Unknown,
        EvidenceMode::Value => match att.active_value.as_ref() {
            None => RuntimeState::Unknown,
            Some(active) if Some(active) == input.resolved_value => RuntimeState::Active,
            Some(_) => RuntimeState::Drift,
        },
        EvidenceMode::ClassifiedDigest => {
            match (att.classified_digest.as_deref(), input.resolved_value) {
                (Some(digest), Some(resolved)) if digest == digest_of(resolved) => {
                    RuntimeState::Active
                }
                (Some(_), Some(_)) => RuntimeState::Drift,
                _ => RuntimeState::Unknown,
            }
        }
        EvidenceMode::Conformance => match att.conformance {
            Some(true) => RuntimeState::Active,
            _ => RuntimeState::Unknown,
        },
    }
}

fn evidence_accepted(att: &Attestation, input: &ClassifyInput<'_>) -> bool {
    if att.channel != EvidenceChannel::DaemonRpc {
        return false;
    }
    if input.required_trust == EvidenceTrust::DaemonAttested
        && att.trust != EvidenceTrust::DaemonAttested
    {
        return false;
    }
    if let Some(owner) = input.required_owner
        && att.component_id != owner
    {
        return false;
    }
    if !att.keys.iter().any(|k| k.as_str() == input.key) {
        return false;
    }
    true
}

fn expired(valid_until: Option<&str>, now: &str) -> bool {
    use chrono::{DateTime, Utc};
    let Some(until_raw) = valid_until else {
        return false;
    };
    let Ok(until) = DateTime::parse_from_rfc3339(until_raw) else {
        return true;
    };
    let Ok(now) = DateTime::parse_from_rfc3339(now) else {
        return true;
    };
    now.with_timezone(&Utc) >= until.with_timezone(&Utc)
}

fn digest_of(value: &Value) -> String {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod runtime_state_tests {
    use super::*;

    fn att() -> Attestation {
        Attestation {
            component_id: "intercept".into(),
            instance_id: "i1".into(),
            component_version: "1".into(),
            channel: EvidenceChannel::DaemonRpc,
            trust: EvidenceTrust::DaemonAttested,
            keys: vec![SettingKey("protection.checks".into())],
            active_value: Some(Value::Bool(true)),
            classified_digest: None,
            conformance: None,
            applied_revision: "rev-1".into(),
            observed_at: "2026-08-25T00:00:00Z".into(),
            valid_until: Some("2026-08-25T01:00:00Z".into()),
            restart_required: false,
            failure: None,
            disconnected: false,
        }
    }

    fn input<'a>(revision: &'a str, now: &'a str, resolved: &'a Value) -> ClassifyInput<'a> {
        ClassifyInput {
            key: "protection.checks",
            evidence_mode: EvidenceMode::Value,
            required_owner: Some("intercept"),
            required_trust: EvidenceTrust::DaemonAttested,
            resolved_value: Some(resolved),
            resolved_revision: revision,
            now,
        }
    }

    #[test]
    fn runtime_state_unknown_without_attestation() {
        let resolved = Value::Bool(true);
        let state =
            classify_runtime_state(None, &input("rev-1", "2026-08-25T00:30:00Z", &resolved));
        assert_eq!(state, RuntimeState::Unknown);
    }

    #[test]
    fn runtime_state_rejects_unregistered_owner() {
        let resolved = Value::Bool(true);
        let mut att = att();
        att.component_id = "not-intercept".into();
        let state = classify_runtime_state(
            Some(&att),
            &input("rev-1", "2026-08-25T00:30:00Z", &resolved),
        );
        assert_eq!(state, RuntimeState::Unknown);
    }

    #[test]
    fn runtime_state_stale_on_revision_change() {
        let resolved = Value::Bool(true);
        let state = classify_runtime_state(
            Some(&att()),
            &input("rev-2", "2026-08-25T00:30:00Z", &resolved),
        );
        assert_eq!(state, RuntimeState::Stale);
    }

    #[test]
    fn runtime_state_failed_on_activation_failure() {
        let resolved = Value::Bool(true);
        let mut att = att();
        att.failure = Some("incompatible".into());
        let state = classify_runtime_state(
            Some(&att),
            &input("rev-1", "2026-08-25T00:30:00Z", &resolved),
        );
        assert_eq!(state, RuntimeState::Failed);
    }

    #[test]
    fn runtime_state_drift_when_active_differs() {
        let resolved = Value::Bool(false);
        let state = classify_runtime_state(
            Some(&att()),
            &input("rev-1", "2026-08-25T00:30:00Z", &resolved),
        );
        assert_eq!(state, RuntimeState::Drift);
    }

    #[test]
    fn runtime_state_active_when_evidence_matches() {
        let resolved = Value::Bool(true);
        let state = classify_runtime_state(
            Some(&att()),
            &input("rev-1", "2026-08-25T00:30:00Z", &resolved),
        );
        assert_eq!(state, RuntimeState::Active);
    }

    #[test]
    fn runtime_state_none_mode_never_active() {
        let resolved = Value::Bool(true);
        let mut inp = input("rev-1", "2026-08-25T00:30:00Z", &resolved);
        inp.evidence_mode = EvidenceMode::None;
        let state = classify_runtime_state(Some(&att()), &inp);
        assert_eq!(state, RuntimeState::Unknown);
    }

    #[test]
    fn runtime_state_rejects_untrusted_channel() {
        let resolved = Value::Bool(true);
        let mut att = att();
        att.channel = EvidenceChannel::Untrusted;
        let state = classify_runtime_state(
            Some(&att),
            &input("rev-1", "2026-08-25T00:30:00Z", &resolved),
        );
        assert_eq!(state, RuntimeState::Unknown);
    }

    #[test]
    fn runtime_state_rejects_trust_mismatch() {
        let resolved = Value::Bool(true);
        let mut att = att();
        att.trust = EvidenceTrust::None;
        let state = classify_runtime_state(
            Some(&att),
            &input("rev-1", "2026-08-25T00:30:00Z", &resolved),
        );
        assert_eq!(state, RuntimeState::Unknown);
    }

    #[test]
    fn runtime_state_stale_on_disconnect_or_expiry() {
        let resolved = Value::Bool(true);
        let mut att = att();
        att.disconnected = true;
        assert_eq!(
            classify_runtime_state(
                Some(&att),
                &input("rev-1", "2026-08-25T00:30:00Z", &resolved)
            ),
            RuntimeState::Stale
        );
        att.disconnected = false;
        assert_eq!(
            classify_runtime_state(
                Some(&att),
                &input("rev-1", "2026-08-25T02:00:00Z", &resolved)
            ),
            RuntimeState::Stale
        );
    }

    #[test]
    fn runtime_state_classified_digest_must_match_resolved() {
        let resolved = Value::Bool(true);
        let mut att = att();
        let mut inp = input("rev-1", "2026-08-25T00:30:00Z", &resolved);
        inp.evidence_mode = EvidenceMode::ClassifiedDigest;
        att.classified_digest = Some("deadbeef".into());
        assert_eq!(
            classify_runtime_state(Some(&att), &inp),
            RuntimeState::Drift
        );
        att.classified_digest = Some(digest_of(&resolved));
        assert_eq!(
            classify_runtime_state(Some(&att), &inp),
            RuntimeState::Active
        );
    }
}
