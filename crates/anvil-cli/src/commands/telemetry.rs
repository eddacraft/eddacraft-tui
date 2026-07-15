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
        Some(TelemetryCommand::On) => set_enabled(&state_dir, true),
        Some(TelemetryCommand::Off) => set_enabled(&state_dir, false),
        Some(TelemetryCommand::ResetId) => reset_id(&state_dir),
    }
}

fn set_enabled(state_dir: &Path, enabled: bool) -> Result<()> {
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
        println!("Telemetry is on. Anonymous install id: {id}");
        println!();
        println!("{}", telemetry::disclosure_text());
    } else {
        println!("Telemetry is off. No beacon will be sent.");
    }
    Ok(())
}

fn reset_id(state_dir: &Path) -> Result<()> {
    let id = telemetry::rotate_install_id_in(state_dir)?;
    println!("Anonymous install id rotated: {id}");
    println!("Previously reported usage can no longer be correlated with this install.");
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn status(state_dir: &Path, global: &GlobalArgs) -> Result<()> {
    let consent = telemetry::load_consent_in(state_dir);
    let gate = telemetry::send_gate();
    let install_id = telemetry::existing_install_id_in(state_dir);
    let env_telemetry = std::env::var(telemetry::TELEMETRY_ENV).ok();
    let do_not_track = std::env::var(telemetry::DO_NOT_TRACK_ENV).ok();
    let install_root_gated = crate::install_root::install_root().is_overridden();

    if global.json {
        // `send_allowed` is the exact predicate the beacon producer will
        // consult, so the JSON surface reports the same answer it gets.
        let send_allowed = telemetry::send_allowed();
        let blocked_reason = match gate {
            SendGate::Allowed => None,
            SendGate::Blocked(reason) => Some(reason.describe()),
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
            "dimensions": telemetry::DISCLOSED_DIMENSIONS,
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
    match gate {
        SendGate::Allowed => println!("Next beacon: allowed"),
        SendGate::Blocked(reason) => println!("Next beacon: blocked — {}", reason.describe()),
    }
    println!();
    println!("Only these dimensions are ever sent:");
    for dimension in telemetry::DISCLOSED_DIMENSIONS {
        println!("  - {dimension}");
    }
    println!("Never: paths, repository names, arguments, hostnames, or emails.");
    Ok(())
}
