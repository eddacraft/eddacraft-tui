//! `anvil telemetry` — the transparency and consent surface for the
//! anonymous fleet beacon.
//!
//! Bare `anvil telemetry` prints the current on/off state, whether the
//! next beacon may send (and why not), the anonymous install id, and
//! the exact dimension allowlist — so the posture is auditable from the
//! binary itself, not just the docs. `on`/`off` persist consent in the
//! user-scoped state directory; `reset-id` rotates the anonymous
//! install id. `ANVIL_TELEMETRY=off` and `DO_NOT_TRACK=1` always win
//! over persisted consent.

use std::path::Path;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use serde_json::json;

use crate::GlobalArgs;
use crate::auth::credentials;
use crate::telemetry::{self, SendGate};

fn status_next_payload_value(payload: &telemetry::BeaconPayload) -> serde_json::Value {
    serde_json::to_value(payload).expect("canonical telemetry payload serialises")
}

#[derive(Debug, Args)]
pub struct TelemetryArgs {
    #[command(subcommand)]
    pub command: Option<TelemetryCommand>,
}

#[derive(Debug, Subcommand)]
pub enum TelemetryCommand {
    /// Turn anonymous usage telemetry on for this user.
    On,
    /// Turn anonymous usage telemetry off for this user (persisted).
    Off,
    /// Rotate the anonymous install id to a fresh random one.
    #[command(name = "reset-id")]
    ResetId,
}

pub fn run(args: &TelemetryArgs, global: &GlobalArgs) -> Result<()> {
    // The consent file lives in the user-scoped state directory, which
    // re-roots under a gated ANVIL_HOME exactly like credentials — a
    // pre-release candidate never touches the production consent state.
    let state_dir = credentials::credentials_dir().context("resolve user state directory")?;
    match &args.command {
        None => status(&state_dir, global),
        Some(TelemetryCommand::On) => set_enabled(&state_dir, true, global.json),
        Some(TelemetryCommand::Off) => set_enabled(&state_dir, false, global.json),
        Some(TelemetryCommand::ResetId) => reset_id(&state_dir, global.json),
    }
}

fn set_enabled(state_dir: &Path, enabled: bool, json_mode: bool) -> Result<()> {
    let update = telemetry::set_enabled_in(state_dir, enabled)?;
    if update.repaired {
        eprintln!(
            "[telemetry] warning: the previous consent state was unreadable; \
             it has been rewritten."
        );
    }
    if enabled {
        // Explicit opt-in: mint the install id now so the transparency
        // surface has something concrete to show.
        let id = telemetry::load_or_create_install_id_in(state_dir)?;
        // Issue #3947: an accepted `--json` means the whole of stdout is
        // one document; the disclosure text rides inside it as a field.
        if json_mode {
            crate::output::json::print(&serde_json::json!({
                "telemetry": "on",
                "install_id": id.to_string(),
                "disclosure": telemetry::disclosure_text(),
            }))?;
        } else {
            println!("Telemetry is on. Anonymous install id: {id}");
            println!();
            println!("{}", telemetry::disclosure_text());
        }
    } else if json_mode {
        crate::output::json::print(&serde_json::json!({
            "telemetry": "off",
            "install_id": serde_json::Value::Null,
        }))?;
    } else {
        println!("Telemetry is off. No beacon will be sent.");
    }
    Ok(())
}

fn reset_id(state_dir: &Path, json_mode: bool) -> Result<()> {
    let id = telemetry::rotate_install_id_in(state_dir)?;
    if json_mode {
        crate::output::json::print(&serde_json::json!({
            "telemetry_id": "rotated",
            "install_id": id.to_string(),
        }))?;
    } else {
        println!("Anonymous install id rotated: {id}");
        println!("Previously reported usage can no longer be correlated with this install.");
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn status(state_dir: &Path, global: &GlobalArgs) -> Result<()> {
    let consent = telemetry::load_consent_in(state_dir);
    let gate = telemetry::send_gate();
    let next_payload = telemetry::next_payload_for_gate_in(state_dir, gate);
    let payload_error = next_payload
        .as_ref()
        .err()
        .map(|error| format!("{error:#}"));
    let next_payload = next_payload.ok().flatten();
    let install_id = telemetry::existing_install_id_in(state_dir);
    let delivery_reason = if gate == SendGate::Allowed && next_payload.is_none() {
        install_id.and_then(|id| {
            telemetry::next_delivery_block_reason_in(state_dir, id, chrono::Utc::now())
                .ok()
                .flatten()
        })
    } else {
        None
    };
    let env_telemetry = std::env::var(telemetry::TELEMETRY_ENV).ok();
    let do_not_track = std::env::var(telemetry::DO_NOT_TRACK_ENV).ok();
    let install_root_gated = crate::install_root::install_root().is_overridden();

    if global.json {
        let send_allowed = gate == SendGate::Allowed && next_payload.is_some();
        let blocked_reason = match (gate, payload_error.as_deref(), delivery_reason) {
            (SendGate::Allowed, None, None) => None,
            (SendGate::Allowed, Some(error), _) => Some(error),
            (SendGate::Allowed, None, Some(reason)) => Some(reason),
            (SendGate::Blocked(reason), _, _) => Some(reason.describe()),
        };
        let payload = json!({
            "enabled": consent.as_ref().map(|c| c.enabled).ok(),
            "noticeShown": consent.as_ref().map(|c| c.notice_shown).ok(),
            "consentReadable": consent.is_ok(),
            "anvilTelemetryEnv": env_telemetry,
            "doNotTrackEnv": do_not_track,
            "installRootGated": install_root_gated,
            "installId": install_id.map(|id| id.to_string()),
            "sendAllowed": send_allowed,
            "blockedReason": blocked_reason,
            "nextPayload": next_payload.as_ref().map(status_next_payload_value),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if let Ok(state) = &consent {
        println!("Telemetry: {}", if state.enabled { "on" } else { "off" });
        println!(
            "Disclosure notice shown: {}",
            if state.notice_shown { "yes" } else { "not yet" }
        );
    } else {
        println!(
            "Telemetry: unknown — the consent state is unreadable and is \
             treated as off (fail-safe)."
        );
        println!("Run `anvil telemetry on` or `anvil telemetry off` to rewrite it.");
    }
    if let Some(value) = &env_telemetry {
        println!("ANVIL_TELEMETRY={value} (environment override)");
    }
    if let Some(value) = &do_not_track {
        println!("DO_NOT_TRACK={value} (environment override)");
    }
    if install_root_gated {
        println!("Install root: non-default ANVIL_HOME — this environment never beacons.");
    }
    match install_id {
        Some(id) => println!("Anonymous install id: {id}"),
        None => println!("Anonymous install id: not yet created"),
    }
    match (
        gate,
        next_payload.as_ref(),
        payload_error.as_deref(),
        delivery_reason,
    ) {
        (SendGate::Allowed, Some(payload), _, _) => {
            println!("Next beacon: allowed");
            println!("Exact next payload:");
            println!("{}", serde_json::to_string_pretty(payload)?);
        }
        (SendGate::Allowed, None, Some(error), _) => {
            println!("Next beacon: blocked — payload unavailable: {error}");
        }
        (SendGate::Allowed, None, None, Some(reason)) => {
            println!("Next beacon: blocked — {reason}");
        }
        (SendGate::Allowed, None, None, None) => {
            println!("Next beacon: blocked — canonical payload unavailable");
        }
        (SendGate::Blocked(reason), _, _, _) => {
            println!("Next beacon: blocked — {}", reason.describe());
        }
    }
    println!();
    println!("Never: paths, repository names, arguments, hostnames, or emails.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_embeds_the_canonical_payload_without_reshaping_it() {
        let payload = telemetry::BeaconPayload::new(
            uuid::Uuid::parse_str("018f78e4-49b5-7f23-a33f-7db9ad9a2f45").unwrap(),
            "0.9.0-beta",
            "cargo_dist",
            "x86_64-unknown-linux-gnu",
            "beta",
            "0",
            vec![],
        );
        assert_eq!(
            status_next_payload_value(&payload),
            serde_json::to_value(&payload).unwrap()
        );
    }
}
