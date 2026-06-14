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

fn run_usage(view: &UsageView, global: &GlobalArgs) -> anyhow::Result<()> {
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
