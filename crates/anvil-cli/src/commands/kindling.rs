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
//! The command reads the user-scoped sidecar
//! (`<credentials_dir>/kindling/usage.ndjson`) and, under
//! `ANVIL_KINDLING_SINK=daemon` (KDS-004), also the Kindling daemon — the
//! daemon-dispatched JSON-RPC rows live there, not in the sidecar — and unions
//! the two so the views see the full picture. It is local-only and needs no
//! authentication, like `anvil insights`.

use clap::{Args, Subcommand, ValueEnum};
use kindling_client::{
    Client, ClientConfig, ListObservationsRequest, ObservationKind, ScopeIds, Spawner,
};

use crate::usage_views::{self, Period, UsageRow};
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

/// KDS-004: enumerate the daemon's `command.invoked` rows for the current
/// project scope, paginating `list_observations` to completeness and parsing
/// each observation's `content` back into a [`UsageRow`] (the same shape the
/// sidecar lines deserialise to). Async core, so the parity test can drive it
/// against an in-process daemon without a nested `block_on`.
async fn collect_daemon_rows(
    client: &Client,
    repo_id: &str,
    page_limit: Option<u32>,
) -> Result<Vec<UsageRow>, kindling_client::ClientError> {
    // Defensive bound: a correct daemon terminates via `next_cursor = None`
    // (keyset over `(ts, id)`). The cap only guards against a buggy daemon that
    // never terminates the cursor, so the CLI can't hang forever. Generous
    // enough (× the page size) that no real usage log reaches it.
    const MAX_PAGES: usize = 100_000;

    let mut rows = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let page = client
            .list_observations(ListObservationsRequest {
                scope_ids: ScopeIds {
                    repo_id: Some(repo_id.to_string()),
                    ..Default::default()
                },
                kinds: vec![ObservationKind::Command],
                since: None,
                until: None,
                limit: page_limit,
                cursor: cursor.take(),
                // The views only need the (non-secret) command/principal/flag
                // shape; redacted bodies carry none of it. Explicit, not relying
                // on the server default.
                include_redacted: Some(false),
            })
            .await?;
        for obs in page.observations {
            // A daemon Observation's `content` is the serialised anvil
            // `command.invoked` payload; parse it as a UsageRow (subset). Skip a
            // row that does not parse (e.g. a non-anvil Command-kind row) —
            // mirrors `load_rows`'s best-effort tolerance.
            if let Ok(row) = serde_json::from_str::<UsageRow>(&obs.content) {
                rows.push(row);
            }
        }
        cursor = page.next_cursor;
        if cursor.is_none() {
            return Ok(rows);
        }
    }
    tracing::warn!(
        target: "anvil::usage",
        "list_observations exceeded {MAX_PAGES} pages; returning a truncated view",
    );
    Ok(rows)
}

/// KDS-004: read the daemon's `command.invoked` rows, bridging the async core on
/// a current-thread runtime. The views command is sync; this never runs on a hot
/// path (and there is no ambient runtime, so the `block_on` is safe).
fn load_rows_from_daemon() -> anyhow::Result<Vec<UsageRow>> {
    let mut config = ClientConfig::defaults()?;
    // A read-only query must NOT start the daemon: a missing daemon should
    // degrade gracefully (sidecar-only), not exec a long-lived `kindling serve`.
    // Replace the default binary spawner with one that fails fast, so an absent
    // daemon surfaces as `ClientError::Unavailable`.
    config.spawn = Spawner::custom(|| {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "anvil kindling usage does not start the kindling daemon",
        ))
    });
    // Scope to the workspace root (git toplevel) — stable across subdirectory
    // invocations — for BOTH the per-project routing key and the row filter, so
    // the read matches the daemon serving this workspace (rather than the raw
    // CWD, which would silently return zero rows from a subdirectory). A daemon
    // started outside this workspace root is a residual scope gap; fully robust
    // per-call scoping is a follow-up (see the KDS-002 note in `usage.rs`).
    let repo_id = crate::util::workspace_root().map_or_else(
        |_| config.project_root.clone(),
        |root| root.to_string_lossy().into_owned(),
    );
    config.project_root.clone_from(&repo_id);
    let client = Client::with_config(config);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    // `None` page limit → the daemon's default page size (server-clamped).
    let rows = runtime.block_on(collect_daemon_rows(&client, &repo_id, None))?;
    Ok(rows)
}

fn run_usage(view: &UsageView, global: &GlobalArgs) -> anyhow::Result<()> {
    let path = usage::default_usage_log_path()?;
    let mut rows = usage_views::load_rows(&path)?;

    // KDS-004: under the daemon sink the JSON-RPC-dispatched `command.invoked`
    // rows live in the daemon (not the sidecar), so union them in for the full
    // picture. In steady state the two sources are disjoint — the CLI producer
    // writes only the sidecar, the daemon producer only the daemon, and they
    // record different invocations — so the union is a plain concat. (There is
    // no shared id to dedup across the two sources, so flipping the sink between
    // runs, or a daemon-sink build-failure fallback to the sidecar, could
    // transiently double-count an invocation. Acceptable for these
    // "signal, not evidence" views and an opt-in / default-off sink; a stable
    // cross-source identity is a follow-up.) Degrade gracefully (sidecar-only,
    // with a stderr note that keeps `--json` stdout clean) if the daemon can't
    // be read. No daemon read when capture is disabled or under ndjson/off.
    if usage::resolve_kindling_sink() == usage::KindlingSinkSelection::Daemon
        && !usage::usage_collection_disabled()
    {
        match load_rows_from_daemon() {
            Ok(daemon_rows) => rows.extend(daemon_rows),
            Err(err) => {
                eprintln!(
                    "Note: ANVIL_KINDLING_SINK=daemon but the Kindling daemon could not be \
                     read ({err}); showing locally-recorded rows only — daemon-dispatched \
                     rows are omitted.",
                );
            }
        }
    }

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

// KDS-004: the daemon read path, proven against a real in-process
// `kindling-server` on a temp Unix domain socket (same pattern as the
// `KindlingDaemonSink` parity tests). Gated `unix` for the UDS bind.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use kindling_client::{ClientConfig, ObservationInput, Spawner, Transport};
    use kindling_server::{ServerConfig, serve};
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::TempDir;

    const REPO_ID: &str = "/repo/anvil";

    /// Long idle timeout so the test daemon never self-shuts mid-test. Routed
    /// through a named const (literal × const) to sidestep
    /// `clippy::duration_suboptimal_units`.
    const TEST_IDLE_TIMEOUT: Duration = {
        const MINUTE_SECS: u64 = 60;
        Duration::from_secs(60 * MINUTE_SECS)
    };

    fn schema_version_u32() -> u32 {
        u32::try_from(kindling_store::schema_version().version).expect("schema version fits u32")
    }

    struct TestDaemon {
        socket_path: PathBuf,
        _home: TempDir,
        _handle: tokio::task::JoinHandle<Result<(), kindling_server::ServerError>>,
    }

    impl TestDaemon {
        async fn start() -> Self {
            let home = tempfile::tempdir().expect("temp kindling home");
            let home_path = home.path().to_path_buf();
            let socket_path = home_path.join("k.sock");
            let config = ServerConfig {
                socket_path: socket_path.clone(),
                kindling_home: home_path.clone(),
                pid_path: home_path.join("k.pid"),
                port_path: home_path.join("k.port"),
                idle_timeout: TEST_IDLE_TIMEOUT,
                transport: kindling_server::Transport::default(),
            };
            let handle = tokio::spawn(async move { serve(config).await });
            let mut ready = false;
            for _ in 0..400 {
                if socket_path.exists() {
                    ready = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert!(
                ready,
                "test kindling daemon socket never appeared: {}",
                socket_path.display(),
            );
            Self {
                socket_path,
                _home: home,
                _handle: handle,
            }
        }

        fn client(&self) -> Client {
            Client::with_config(ClientConfig {
                socket_path: self.socket_path.clone(),
                port_path: PathBuf::from("unused.port"),
                project_root: REPO_ID.to_string(),
                expected_schema_version: schema_version_u32(),
                connect_timeout: Duration::from_secs(2),
                poll_interval: Duration::from_millis(10),
                spawn: Spawner::custom(|| {
                    panic!("spawner must not be called when the daemon is up")
                }),
                transport: Transport::Uds,
                spawn_log_path: None,
            })
        }
    }

    async fn append_command(client: &Client, command: &str, principal: &str, ts: &str) {
        let content = format!(
            r#"{{"kind":"command.invoked","session_id":"s","timestamp":"{ts}","command":"{command}","principal":"{principal}","args":[],"flag_set":[]}}"#
        );
        client
            .append_observation(
                ObservationInput {
                    id: None,
                    kind: ObservationKind::Command,
                    content,
                    provenance: None,
                    ts: None,
                    scope_ids: ScopeIds {
                        repo_id: Some(REPO_ID.to_string()),
                        ..Default::default()
                    },
                    redacted: None,
                },
                None,
                Some(true),
            )
            .await
            .expect("append command.invoked");
    }

    /// `collect_daemon_rows` enumerates **every** matching row across pages
    /// (forced multi-page via a small page limit) and parses each back to a
    /// `UsageRow` — the completeness the exact-count / set-difference views need.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn collect_daemon_rows_enumerates_all_across_pages() {
        let daemon = TestDaemon::start().await;
        let client = daemon.client();
        // Seed 5 command.invoked rows; `check` ×3, `status` ×2.
        append_command(&client, "check", "p1", "2026-06-26T10:00:00Z").await;
        append_command(&client, "check", "p1", "2026-06-26T10:01:00Z").await;
        append_command(&client, "check", "p2", "2026-06-26T10:02:00Z").await;
        append_command(&client, "status", "p2", "2026-06-26T10:03:00Z").await;
        append_command(&client, "status", "p1", "2026-06-26T10:04:00Z").await;

        // page_limit = 2 forces 3 pages, exercising the keyset cursor loop.
        let rows = collect_daemon_rows(&client, REPO_ID, Some(2))
            .await
            .expect("collect succeeds");

        assert_eq!(rows.len(), 5, "all rows across all pages are returned");
        assert_eq!(rows.iter().filter(|r| r.command == "check").count(), 3);
        assert_eq!(rows.iter().filter(|r| r.command == "status").count(), 2);
        assert!(rows.iter().any(|r| r.principal == "p2"));
    }

    /// A non-anvil `Command`-kind row (content that isn't a `command.invoked`
    /// payload) is skipped, not surfaced as a bogus view row.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn collect_daemon_rows_skips_unparseable_content() {
        let daemon = TestDaemon::start().await;
        let client = daemon.client();
        append_command(&client, "check", "p1", "2026-06-26T10:00:00Z").await;
        // A Command-kind row whose content is not an anvil command.invoked payload.
        client
            .append_observation(
                ObservationInput {
                    id: None,
                    kind: ObservationKind::Command,
                    content: "not-a-command-invoked-json".to_string(),
                    provenance: None,
                    ts: None,
                    scope_ids: ScopeIds {
                        repo_id: Some(REPO_ID.to_string()),
                        ..Default::default()
                    },
                    redacted: None,
                },
                None,
                Some(true),
            )
            .await
            .expect("append");

        let rows = collect_daemon_rows(&client, REPO_ID, None)
            .await
            .expect("collect succeeds");
        assert_eq!(
            rows.len(),
            1,
            "only the parseable command.invoked row is kept"
        );
        assert_eq!(rows[0].command, "check");
    }
}
