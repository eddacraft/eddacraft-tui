//! USAGE-003: `anvil kindling usage <view>` — dev-investment query views
//! over the local command-invocation usage log.
//!
//! A first-class surface for the founder's standing questions ("what is
//! being used and what is not") so they need neither ad-hoc `jq` nor SQL.
//! The pure view logic lives in [`crate::usage_views`]; this module is the
//! clap surface and the human/JSON rendering. The runbook in
//! `docs/observability/usage-analytics.md` documents both the commands and
//! the standing caveat: **these views are signal, not evidence.**
//!
//! The command reads only the user-scoped sidecar
//! (`<credentials_dir>/kindling/usage.ndjson`); it is local-only and needs
//! no authentication, like `anvil insights`.

use clap::{Args, Subcommand, ValueEnum};

use crate::usage_views::{self, Period};
use crate::{GlobalArgs, usage};

#[derive(Debug, Args)]
pub struct KindlingArgs {
    #[command(subcommand)]
    command: KindlingCommand,
}

#[derive(Debug, Subcommand)]
enum KindlingCommand {
    /// Dev-investment usage views over the local command-invocation log.
    #[command(subcommand)]
    Usage(UsageView),
}

#[derive(Debug, Subcommand)]
enum UsageView {
    /// Top commands by invocation count.
    Top(TopArgs),
    /// Registered commands that have never been invoked.
    Unused,
    /// Flag-dependent paths exercised in the log (which flags were active).
    Flags,
    /// Anonymised principals by activity level.
    Principals,
}

#[derive(Debug, Args)]
pub struct TopArgs {
    /// Time window for the count.
    #[arg(long, value_enum, default_value_t = PeriodArg::All)]
    period: PeriodArg,
    /// Maximum rows to show (`0` = no limit).
    #[arg(long, default_value_t = 10)]
    limit: usize,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PeriodArg {
    /// Trailing 7 days.
    Week,
    /// Trailing 30 days.
    Month,
    /// Everything since launch.
    All,
}

impl From<PeriodArg> for Period {
    fn from(value: PeriodArg) -> Self {
        match value {
            PeriodArg::Week => Period::Week,
            PeriodArg::Month => Period::Month,
            PeriodArg::All => Period::All,
        }
    }
}

pub fn run(args: &KindlingArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    match &args.command {
        KindlingCommand::Usage(view) => run_usage(view, global),
    }
}

/// KDS-004: the note (if any) to print before a usage view, warning that the
/// sidecar these views read is missing the daemon-routed rows.
///
/// Under `ANVIL_KINDLING_SINK=daemon` the **daemon-dispatched (JSON-RPC)**
/// `command.invoked` rows go to the Kindling daemon (KDS-002), not the
/// `usage.ndjson` sidecar — so these sidecar-sourced views omit them and may be
/// incomplete. (CLI invocations are still recorded to the sidecar by the CLI
/// producer regardless of the sink, so a CLI-only picture stays complete.) A
/// daemon-backed read path that would re-include the daemon rows is blocked on an
/// upstream kindling list/aggregate read API (eddacraft/anvil-001#2910).
///
/// No note for `ndjson` (the default — the sidecar is authoritative) or `off`.
/// And no note when capture is disabled outright (`ANVIL_USAGE_DISABLE` /
/// `DO_NOT_TRACK` / the `ANVIL_INTERCEPT_DISABLE_OBSERVATION` break-glass): the
/// views are then sparse because the operator opted out, not because of the
/// sink, so the daemon-source caveat would only mislead.
fn sidecar_source_warning() -> Option<&'static str> {
    if usage::resolve_kindling_sink() != usage::KindlingSinkSelection::Daemon
        || usage::usage_collection_disabled()
    {
        return None;
    }
    Some(
        "Note: ANVIL_KINDLING_SINK=daemon — daemon-dispatched (JSON-RPC) \
         command.invoked rows are sent to the Kindling daemon under this sink and \
         are not in these sidecar-sourced views (CLI invocations are still \
         recorded locally), so results may be incomplete. A daemon-backed read \
         path is tracked in eddacraft/anvil-001#2910.",
    )
}

fn run_usage(view: &UsageView, global: &GlobalArgs) -> anyhow::Result<()> {
    // KDS-004: warn (stderr, so `--json` stdout stays clean) when the sidecar
    // these views read is not the authoritative store under the daemon sink.
    if let Some(note) = sidecar_source_warning() {
        eprintln!("{note}");
    }

    let path = usage::default_usage_log_path()?;
    let rows = usage_views::load_rows(&path)?;

    match view {
        UsageView::Top(args) => {
            let result = usage_views::top_commands(
                &rows,
                args.period.into(),
                chrono::Utc::now(),
                args.limit,
            );
            if global.json {
                print_json(&result)?;
            } else {
                render_top(&result);
            }
        }
        UsageView::Unused => {
            let registered = crate::registered_command_names();
            let result = usage_views::never_invoked(&rows, &registered);
            if global.json {
                print_json(&result)?;
            } else {
                render_list("Commands never invoked", &result);
            }
        }
        UsageView::Flags => {
            let result = usage_views::flag_usage(&rows);
            if global.json {
                print_json(&result)?;
            } else {
                render_flags(&result);
            }
        }
        UsageView::Principals => {
            let result = usage_views::principals_by_activity(&rows);
            if global.json {
                print_json(&result)?;
            } else {
                render_principals(&result);
            }
        }
    }
    Ok(())
}

fn print_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// One-line caveat printed under every human view: these are direction
/// signals, not decision evidence (USAGE-003 / OQ3).
fn print_signal_footer() {
    eprintln!("\nNote: usage views are signal, not evidence — small populations, flag bias, and");
    eprintln!("survivorship effects mean they inform direction, not decisions in isolation.");
}

fn render_top(result: &[usage_views::CommandCount]) {
    if result.is_empty() {
        println!("No command invocations recorded yet.");
        return;
    }
    println!("Top commands by invocation count:");
    for entry in result {
        println!("  {:>6}  {}", entry.count, entry.command);
    }
    print_signal_footer();
}

fn render_list(title: &str, items: &[String]) {
    if items.is_empty() {
        println!("{title}: none.");
        return;
    }
    println!("{title}:");
    for item in items {
        println!("  {item}");
    }
    print_signal_footer();
}

fn render_flags(result: &[usage_views::FlagUsage]) {
    if result.is_empty() {
        println!("No flag-dependent paths recorded yet.");
        return;
    }
    println!("Flag-dependent paths exercised:");
    for flag in result {
        let gate = if flag.gate_affecting { " [gate]" } else { "" };
        let plural = if flag.invocations == 1 { "" } else { "s" };
        println!(
            "  {} ({} invocation{plural}){gate}",
            flag.key, flag.invocations
        );
        for variant in &flag.variants {
            println!("      {:>6}  {}", variant.count, variant.variant);
        }
    }
    print_signal_footer();
}

fn render_principals(result: &[usage_views::PrincipalActivity]) {
    if result.is_empty() {
        println!("No principals recorded yet.");
        return;
    }
    println!("Principals by activity level (anonymised):");
    for entry in result {
        println!("  {:>6}  {}", entry.invocations, entry.principal);
    }
    print_signal_footer();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KDS-004: the source-aware note fires ONLY under the daemon sink with
    /// capture on — the one case where the sidecar omits the daemon-routed rows.
    #[test]
    fn sidecar_warning_only_under_daemon_sink() {
        // Daemon sink + capture on → warns and points at the tracking issue.
        // Clear the opt-out vars so an ambient one can't suppress the note.
        temp_env::with_vars(
            [
                ("ANVIL_KINDLING_SINK", Some("daemon")),
                ("ANVIL_USAGE_DISABLE", None),
                ("DO_NOT_TRACK", None),
                ("ANVIL_INTERCEPT_DISABLE_OBSERVATION", None),
            ],
            || {
                let note = sidecar_source_warning().expect("daemon sink warns");
                assert!(
                    note.contains("anvil-001#2910"),
                    "note points at the tracking issue"
                );
                assert!(note.contains("may be incomplete"));
            },
        );

        // ndjson / off / an unrecognised value (falls back to ndjson) / unset →
        // the sidecar is the right source, so no note. Clear the opt-out vars too
        // so the `None` is attributable to the SINK, not to an ambient
        // capture-disable making `usage_collection_disabled()` true (which would
        // pass these assertions vacuously).
        for v in [Some("ndjson"), Some("off"), Some("DAEMON_TYPO"), None] {
            temp_env::with_vars(
                [
                    ("ANVIL_KINDLING_SINK", v),
                    ("ANVIL_USAGE_DISABLE", None::<&str>),
                    ("DO_NOT_TRACK", None::<&str>),
                    ("ANVIL_INTERCEPT_DISABLE_OBSERVATION", None::<&str>),
                ],
                || {
                    assert!(sidecar_source_warning().is_none(), "no note for {v:?}");
                },
            );
        }

        // Daemon sink BUT capture disabled → the views are sparse because the
        // operator opted out, not because of the sink, so no daemon-source note.
        temp_env::with_vars(
            [
                ("ANVIL_KINDLING_SINK", Some("daemon")),
                ("ANVIL_USAGE_DISABLE", Some("1")),
            ],
            || {
                assert!(
                    sidecar_source_warning().is_none(),
                    "no daemon-source note when capture is disabled",
                );
            },
        );
    }
}
