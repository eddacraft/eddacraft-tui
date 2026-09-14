//! Coarse settings-inspect telemetry (SETINS-009).
//!
//! Only interaction signals. Never values, paths, search strings, policy,
//! diffs, secrets, audit details or attestation payloads.

use serde::Serialize;

/// Signals that may be collected when product telemetry is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsSignal {
    PanelOpened,
    SearchUsed,
    ValidationFailed,
}

/// Coarse event. The payload is the signal name only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SettingsTelemetryEvent {
    pub signal: SettingsSignal,
}

/// Env-level collection gate (SETINS-009). Honour `DO_NOT_TRACK` and
/// `ANVIL_TELEMETRY=off` even when a caller has no CLI consent store.
#[must_use]
pub fn env_allows_collection(anvil_telemetry: Option<&str>, do_not_track: Option<&str>) -> bool {
    if do_not_track.is_some_and(|value| {
        let value = value.trim();
        !value.is_empty() && !matches!(value.to_ascii_lowercase().as_str(), "0" | "false")
    }) {
        return false;
    }
    !anvil_telemetry.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "off" | "0" | "false"
        )
    })
}

/// Emit a coarse signal when env collection is allowed. Failures are
/// swallowed so inspection still works.
pub fn emit(signal: SettingsSignal) {
    let anvil_telemetry = std::env::var("ANVIL_TELEMETRY").ok();
    let do_not_track = std::env::var("DO_NOT_TRACK").ok();
    if !env_allows_collection(anvil_telemetry.as_deref(), do_not_track.as_deref()) {
        return;
    }
    let event = SettingsTelemetryEvent { signal };
    tracing::info!(target: "anvil.settings", signal = ?event.signal, "settings_inspect");
    let _ = event;
}

/// True when a JSON payload contains only the allowed coarse fields.
#[must_use]
pub fn payload_is_coarse(value: &serde_json::Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    obj.len() == 1 && obj.contains_key("signal")
}

#[cfg(test)]
mod settings_telemetry_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn settings_telemetry_payload_is_coarse_only() {
        let event = SettingsTelemetryEvent {
            signal: SettingsSignal::PanelOpened,
        };
        let value = serde_json::to_value(event).expect("json");
        assert!(payload_is_coarse(&value), "{value}");
        let dump = value.to_string();
        for forbidden in [
            "resolved",
            "configured",
            "path",
            "query",
            "policy",
            "diff",
            "secret",
            "attestation",
            "audit",
        ] {
            assert!(
                !dump.contains(forbidden),
                "payload leaked {forbidden}: {dump}"
            );
        }
    }

    #[test]
    fn settings_telemetry_rejects_rich_payloads() {
        assert!(!payload_is_coarse(
            &json!({"signal": "panel_opened", "query": "enforcement"})
        ));
        assert!(!payload_is_coarse(&json!({"value": "block"})));
    }

    #[test]
    fn settings_telemetry_emit_does_not_panic() {
        emit(SettingsSignal::SearchUsed);
        emit(SettingsSignal::ValidationFailed);
    }

    #[test]
    fn settings_telemetry_env_gate_honours_opt_out() {
        assert!(env_allows_collection(None, None));
        assert!(env_allows_collection(Some("on"), None));
        assert!(!env_allows_collection(Some("off"), None));
        assert!(!env_allows_collection(Some("0"), None));
        assert!(!env_allows_collection(None, Some("1")));
        assert!(!env_allows_collection(Some("on"), Some("true")));
        assert!(env_allows_collection(None, Some("0")));
        assert!(env_allows_collection(None, Some("false")));
    }
}
