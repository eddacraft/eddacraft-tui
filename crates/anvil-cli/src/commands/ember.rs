//! RCLI3-005: `anvil ember` — port of the historical Node.js Ember CLI.
//!
//! Today this implements `anvil ember list` (RCLI3-005). The command reads
//! Ember proposals from the `.anvil/ember.db` `SQLite` database that the
//! TypeScript `ProposalStore` (`packages/edda-stack/src/ember/proposal-store.ts`)
//! writes, applies `--type` / `--status` filters, sorts by `created_at`
//! descending, and renders either a human-readable table or the JSON envelope
//! existing scripts depend on.
//!
//! The query mirrors `ProposalStore.queryProposals` as `list.ts` calls it:
//! a single requested status, `created_at DESC`, `LIMIT`/`OFFSET 0`. The found
//! JSON envelope keeps the historical shape (`database_found`, `database_path`,
//! `total`, `limit`, `has_more`, `filters`, `proposals`) and the not-found
//! envelope matches the Node five-key shape, so consumers keep working.
//!
//! Note on enums: the proposal `type` and `status` vocabularies are taken
//! from the live `ProposalStore` schema, not the older RCLI3-005 draft (which
//! listed `observation`/`suggestion` types and a `rejected` status that the
//! schema never had).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use clap::{Args, Subcommand};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OpenFlags, params_from_iter};
use serde::Serialize;
use serde_json::{Value, json};

use crate::GlobalArgs;
use crate::output::AlreadyReported;

/// Default number of proposals shown, matching the historical Node.js CLI.
const DEFAULT_LIMIT: usize = 20;

#[derive(Debug, Args)]
pub struct EmberArgs {
    #[command(subcommand)]
    command: EmberCommand,
}

#[derive(Debug, Subcommand)]
enum EmberCommand {
    /// List Ember proposals with filtering.
    #[command(alias = "ls")]
    List(ListArgs),
}

#[derive(Debug, Args)]
struct ListArgs {
    /// Output as JSON.
    #[arg(long)]
    json: bool,
    /// Filter by proposal type (comma-separated for multiple).
    #[arg(long = "type", value_name = "TYPE")]
    types: Option<String>,
    /// Filter by proposal status. Defaults to `active`.
    #[arg(long, default_value = "active")]
    status: String,
    /// Maximum proposals to display.
    #[arg(long, default_value_t = DEFAULT_LIMIT, value_parser = parse_limit)]
    limit: usize,
}

/// Parse `--limit`, rejecting zero to mirror the historical Node.js
/// `coercePositiveInt` (which required a positive integer).
fn parse_limit(raw: &str) -> Result<usize, String> {
    let value: usize = raw
        .parse()
        .map_err(|_| format!("`{raw}` is not a valid number"))?;
    if value == 0 {
        return Err("--limit must be a positive integer".to_owned());
    }
    Ok(value)
}

pub fn run(args: &EmberArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        EmberCommand::List(list_args) => run_list(list_args, global),
    }
}

// ---------------------------------------------------------------------------
// Proposal type / status — mirror the edda-stack ProposalStore schema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProposalType {
    Decision,
    Pattern,
    Warning,
    Lesson,
    Anomaly,
    Constraint,
}

impl ProposalType {
    fn as_str(self) -> &'static str {
        match self {
            ProposalType::Decision => "decision",
            ProposalType::Pattern => "pattern",
            ProposalType::Warning => "warning",
            ProposalType::Lesson => "lesson",
            ProposalType::Anomaly => "anomaly",
            ProposalType::Constraint => "constraint",
        }
    }
}

fn parse_type(value: &str) -> Result<ProposalType> {
    match value.trim() {
        "decision" => Ok(ProposalType::Decision),
        "pattern" => Ok(ProposalType::Pattern),
        "warning" => Ok(ProposalType::Warning),
        "lesson" => Ok(ProposalType::Lesson),
        "anomaly" => Ok(ProposalType::Anomaly),
        "constraint" => Ok(ProposalType::Constraint),
        other => bail!(
            "invalid proposal type: {other}; expected one of decision, pattern, warning, lesson, anomaly, constraint"
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProposalStatus {
    Active,
    Promoted,
    Expired,
    Dismissed,
}

impl ProposalStatus {
    fn as_str(self) -> &'static str {
        match self {
            ProposalStatus::Active => "active",
            ProposalStatus::Promoted => "promoted",
            ProposalStatus::Expired => "expired",
            ProposalStatus::Dismissed => "dismissed",
        }
    }
}

fn parse_status(value: &str) -> Result<ProposalStatus> {
    match value.trim() {
        "active" => Ok(ProposalStatus::Active),
        "promoted" => Ok(ProposalStatus::Promoted),
        "expired" => Ok(ProposalStatus::Expired),
        "dismissed" => Ok(ProposalStatus::Dismissed),
        other => bail!(
            "invalid proposal status: {other}; expected one of active, promoted, expired, dismissed"
        ),
    }
}

fn parse_csv_types(value: Option<&str>) -> Result<Vec<ProposalType>> {
    let Some(raw) = value else {
        return Ok(Vec::new());
    };
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_type)
        .collect()
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

struct Filter {
    types: Vec<ProposalType>,
    status: ProposalStatus,
    limit: usize,
    offset: usize,
}

#[derive(Debug, Clone, Serialize)]
struct Proposal {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    status: String,
    summary: String,
    rationale: String,
    confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
    signals: Value,
    provenance: Value,
    created_at: String,
    expires_at: String,
    ttl_days: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<Value>,
}

struct QueryOutcome {
    total: usize,
    limit: usize,
    has_more: bool,
    proposals: Vec<Proposal>,
}

/// Parse a nullable JSON text column, returning `None` when the column is
/// absent or unparseable. Callers choose the fallback (`Value::Null` or
/// `json!([])`), a tolerant mirror of the Node `JSON.parse(... ?? default)`.
fn parse_json_column(raw: Option<String>) -> Option<Value> {
    raw.and_then(|text| serde_json::from_str(&text).ok())
}

fn query_proposals(conn: &Connection, filter: &Filter) -> Result<QueryOutcome> {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut binds: Vec<SqlValue> = Vec::new();

    if !filter.types.is_empty() {
        let placeholders = vec!["?"; filter.types.len()].join(", ");
        where_clauses.push(format!("type IN ({placeholders})"));
        for t in &filter.types {
            binds.push(SqlValue::Text(t.as_str().to_owned()));
        }
    }

    // A single requested status fully determines the filter, so we bind only
    // `status IN (?)`. The Node `queryProposals` additionally emits a
    // `status != 'expired'` guard when `include_expired` is false, but with a
    // single non-expired status that clause is always redundant, and the
    // `expired` status sets `include_expired` (dropping the guard) anyway — so
    // the net result set is identical without it.
    where_clauses.push("status IN (?)".to_owned());
    binds.push(SqlValue::Text(filter.status.as_str().to_owned()));

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM proposals {where_sql}");
    let total: i64 = conn
        .query_row(&count_sql, params_from_iter(binds.iter()), |row| row.get(0))
        .context("counting Ember proposals")?;
    let total = usize::try_from(total).unwrap_or(0);

    let rows_sql = format!(
        "SELECT id, type, status, summary, rationale, confidence, metadata, \
         signals, provenance, created_at, expires_at, ttl_days, updated_at, resolution \
         FROM proposals {where_sql} ORDER BY created_at DESC LIMIT ? OFFSET ?"
    );
    let mut row_binds = binds.clone();
    row_binds.push(SqlValue::Integer(
        i64::try_from(filter.limit).unwrap_or(i64::MAX),
    ));
    row_binds.push(SqlValue::Integer(i64::try_from(filter.offset).unwrap_or(0)));

    let mut stmt = conn.prepare(&rows_sql).context("preparing Ember query")?;
    let proposals = stmt
        .query_map(params_from_iter(row_binds.iter()), |row| {
            Ok(Proposal {
                id: row.get(0)?,
                kind: row.get(1)?,
                status: row.get(2)?,
                summary: row.get(3)?,
                rationale: row.get(4)?,
                confidence: row.get(5)?,
                metadata: parse_json_column(row.get(6)?),
                signals: parse_json_column(row.get(7)?).unwrap_or_else(|| json!([])),
                provenance: parse_json_column(row.get(8)?).unwrap_or(Value::Null),
                created_at: row.get(9)?,
                expires_at: row.get(10)?,
                ttl_days: row.get(11)?,
                updated_at: row.get(12)?,
                resolution: parse_json_column(row.get(13)?),
            })
        })
        .context("querying Ember proposals")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("reading Ember proposal rows")?;

    let has_more = filter.offset + proposals.len() < total;

    Ok(QueryOutcome {
        total,
        limit: filter.limit,
        has_more,
        proposals,
    })
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

fn workspace_db_path() -> Result<PathBuf> {
    // Match the historical Node.js `getWorkspaceRoot`: resolve the repo root
    // (via `git rev-parse --show-toplevel`, cwd fallback) so `anvil ember list`
    // finds `.anvil/ember.db` even when invoked from a subdirectory.
    Ok(crate::util::workspace_root()?
        .join(".anvil")
        .join("ember.db"))
}

fn run_list(args: &ListArgs, global: &GlobalArgs) -> Result<()> {
    // Honour both the subcommand-local `--json` and the global `anvil --json`
    // flag, matching the sibling `edda` command.
    let json = args.json || global.json;
    let db_path = workspace_db_path()?;
    let status = parse_status(&args.status)?;
    let types = parse_csv_types(args.types.as_deref())?;
    let filter = Filter {
        types,
        status,
        limit: args.limit,
        offset: 0,
    };

    if !db_path.exists() {
        // Mirror the historical Node.js contract: a missing database is an
        // error exit (not an empty success). In `--json` mode the structured
        // not-found envelope goes to stdout for consumers and `AlreadyReported`
        // suppresses the top-level text error so output prints exactly once.
        if json {
            println!("{}", missing_envelope(&db_path));
            return Err(AlreadyReported.into());
        }
        bail!("No Ember database found at {}", db_path.display());
    }

    let conn = match Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => conn,
        Err(err) => {
            return report_query_error(
                json,
                &format!("opening Ember database at {}: {err}", db_path.display()),
            );
        }
    };
    let outcome = match query_proposals(&conn, &filter) {
        Ok(outcome) => outcome,
        Err(err) => return report_query_error(json, &format!("{err:#}")),
    };

    // Use the canonical parsed status (not the raw arg) so trimmed/odd input
    // like `--status "active "` is reported in its canonical form.
    let status_label = filter.status.as_str();

    if json {
        println!(
            "{}",
            found_envelope(&db_path, status_label, &filter.types, &outcome)
        );
        return Ok(());
    }

    render_table(&outcome, status_label, &filter.types);
    Ok(())
}

/// Report a database/query failure once. In `--json` mode the error goes to
/// stdout as `{ "error": ... }` (mirroring the Node.js catch block) and
/// `AlreadyReported` stops `main` reprinting it; otherwise it propagates as a
/// normal error for the top-level `Error:` renderer.
fn report_query_error(json: bool, message: &str) -> Result<()> {
    if json {
        println!("{}", json!({ "error": message }));
        return Err(AlreadyReported.into());
    }
    bail!("{message}")
}

// ---------------------------------------------------------------------------
// JSON envelopes (historical shape)
// ---------------------------------------------------------------------------

fn filters_payload(status: &str, types: &[ProposalType]) -> Value {
    let type_value = if types.is_empty() {
        Value::Null
    } else {
        Value::Array(
            types
                .iter()
                .map(|t| Value::String(t.as_str().to_owned()))
                .collect(),
        )
    };
    json!({ "status": status, "type": type_value })
}

fn missing_envelope(db_path: &Path) -> Value {
    // Exactly the historical Node.js not-found shape (`list.ts`): five keys,
    // no `limit`/`has_more`/`filters`.
    json!({
        "error": format!("No Ember database found at {}", db_path.display()),
        "database_found": false,
        "database_path": db_path.display().to_string(),
        "total": 0,
        "proposals": [],
    })
}

fn found_envelope(
    db_path: &Path,
    status: &str,
    types: &[ProposalType],
    outcome: &QueryOutcome,
) -> Value {
    json!({
        "database_found": true,
        "database_path": db_path.display().to_string(),
        "total": outcome.total,
        "limit": outcome.limit,
        "has_more": outcome.has_more,
        "filters": filters_payload(status, types),
        "proposals": outcome.proposals,
    })
}

// ---------------------------------------------------------------------------
// Human rendering
// ---------------------------------------------------------------------------

fn render_table(outcome: &QueryOutcome, status: &str, types: &[ProposalType]) {
    let type_filter = if types.is_empty() {
        "all".to_owned()
    } else {
        types
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    println!();
    println!("Ember Proposals");
    println!(
        "{} found  |  status: {status}  |  type: {type_filter}",
        outcome.total
    );
    println!(
        "  {:<14} {:<11} {:<10} {:<12} {:<34} {:<16} {:<16}",
        "ID", "Type", "Status", "Confidence", "Summary", "Created", "Expires",
    );

    if outcome.proposals.is_empty() {
        println!("  No proposals match the current filters.");
        println!();
        return;
    }

    for proposal in &outcome.proposals {
        println!(
            "  {:<14} {:<11} {:<10} {:<12.2} {:<34} {:<16} {:<16}",
            truncate(&proposal.id, 12),
            proposal.kind,
            proposal.status,
            proposal.confidence,
            truncate(&proposal.summary, 32),
            format_relative_time(&proposal.created_at),
            format_relative_time(&proposal.expires_at),
        );
    }
    println!();
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_owned();
    }
    let kept: String = value.chars().take(width.saturating_sub(2)).collect();
    format!("{kept}..")
}

/// Relative time mirroring the Node.js `formatRelativeTime`: `just now`,
/// `Nm ago` / `in Nm`, `Nh ago` / `in Nh`, `Nd ago` / `in Nd`. Falls back to
/// the raw string when it is not a parseable RFC3339 timestamp.
fn format_relative_time(raw: &str) -> String {
    let Ok(parsed) = DateTime::parse_from_rfc3339(raw) else {
        return raw.to_owned();
    };
    let diff = Utc::now().signed_duration_since(parsed.with_timezone(&Utc));
    let secs = diff.num_seconds();
    let abs = secs.abs();
    let forward = secs >= 0;

    if abs < 60 {
        return if forward {
            "just now".to_owned()
        } else {
            "soon".to_owned()
        };
    }
    if abs < 3_600 {
        let n = abs / 60;
        return if forward {
            format!("{n}m ago")
        } else {
            format!("in {n}m")
        };
    }
    if abs < 86_400 {
        let n = abs / 3_600;
        return if forward {
            format!("{n}h ago")
        } else {
            format!("in {n}h")
        };
    }
    let n = abs / 86_400;
    if forward {
        format!("{n}d ago")
    } else {
        format!("in {n}d")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE proposals (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                summary TEXT NOT NULL,
                rationale TEXT NOT NULL,
                confidence REAL NOT NULL,
                metadata TEXT,
                signals TEXT,
                provenance TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                ttl_days INTEGER NOT NULL,
                updated_at TEXT,
                resolution TEXT
            );",
        )
        .expect("create schema");
        conn
    }

    #[allow(clippy::too_many_arguments)]
    fn insert(
        conn: &Connection,
        id: &str,
        type_: &str,
        status: &str,
        confidence: f64,
        created_at: &str,
        expires_at: &str,
    ) {
        conn.execute(
            "INSERT INTO proposals
                (id, type, status, summary, rationale, confidence, metadata, signals,
                 provenance, created_at, expires_at, ttl_days, updated_at, resolution)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, '[]', '{\"session_ids\":[]}', ?7, ?8, 30, NULL, NULL)",
            rusqlite::params![
                id,
                type_,
                status,
                format!("summary for {id}"),
                "rationale",
                confidence,
                created_at,
                expires_at,
            ],
        )
        .expect("insert proposal");
    }

    fn filter(status: ProposalStatus, types: Vec<ProposalType>, limit: usize) -> Filter {
        Filter {
            types,
            status,
            limit,
            offset: 0,
        }
    }

    #[test]
    fn lists_active_only_sorted_created_desc() {
        let conn = seed_conn();
        insert(
            &conn,
            "old",
            "pattern",
            "active",
            0.9,
            "2026-06-10T00:00:00Z",
            "2026-07-10T00:00:00Z",
        );
        insert(
            &conn,
            "new",
            "pattern",
            "active",
            0.8,
            "2026-06-16T00:00:00Z",
            "2026-07-16T00:00:00Z",
        );
        insert(
            &conn,
            "prom",
            "pattern",
            "promoted",
            0.7,
            "2026-06-15T00:00:00Z",
            "2026-07-15T00:00:00Z",
        );
        insert(
            &conn,
            "exp",
            "pattern",
            "expired",
            0.6,
            "2026-06-14T00:00:00Z",
            "2026-06-15T00:00:00Z",
        );

        let outcome = query_proposals(&conn, &filter(ProposalStatus::Active, vec![], 20)).unwrap();

        assert_eq!(outcome.total, 2, "only active proposals counted");
        let ids: Vec<&str> = outcome.proposals.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["new", "old"], "sorted by created_at desc");
        assert!(!outcome.has_more);
    }

    #[test]
    fn status_filter_selects_promoted() {
        let conn = seed_conn();
        insert(
            &conn,
            "a",
            "pattern",
            "active",
            0.9,
            "2026-06-10T00:00:00Z",
            "2026-07-10T00:00:00Z",
        );
        insert(
            &conn,
            "p",
            "pattern",
            "promoted",
            0.8,
            "2026-06-11T00:00:00Z",
            "2026-07-11T00:00:00Z",
        );

        let outcome =
            query_proposals(&conn, &filter(ProposalStatus::Promoted, vec![], 20)).unwrap();

        assert_eq!(outcome.total, 1);
        assert_eq!(outcome.proposals[0].id, "p");
    }

    #[test]
    fn expired_status_includes_expired_rows() {
        let conn = seed_conn();
        insert(
            &conn,
            "a",
            "pattern",
            "active",
            0.9,
            "2026-06-10T00:00:00Z",
            "2026-07-10T00:00:00Z",
        );
        insert(
            &conn,
            "e",
            "pattern",
            "expired",
            0.5,
            "2026-06-09T00:00:00Z",
            "2026-06-10T00:00:00Z",
        );

        let outcome = query_proposals(&conn, &filter(ProposalStatus::Expired, vec![], 20)).unwrap();

        assert_eq!(outcome.total, 1, "expired status surfaces expired rows");
        assert_eq!(outcome.proposals[0].id, "e");
    }

    #[test]
    fn type_filter_narrows_results() {
        let conn = seed_conn();
        insert(
            &conn,
            "pat",
            "pattern",
            "active",
            0.9,
            "2026-06-10T00:00:00Z",
            "2026-07-10T00:00:00Z",
        );
        insert(
            &conn,
            "dec",
            "decision",
            "active",
            0.8,
            "2026-06-11T00:00:00Z",
            "2026-07-11T00:00:00Z",
        );
        insert(
            &conn,
            "ano",
            "anomaly",
            "active",
            0.7,
            "2026-06-12T00:00:00Z",
            "2026-07-12T00:00:00Z",
        );

        let outcome = query_proposals(
            &conn,
            &filter(
                ProposalStatus::Active,
                vec![ProposalType::Pattern, ProposalType::Anomaly],
                20,
            ),
        )
        .unwrap();

        assert_eq!(outcome.total, 2);
        let mut ids: Vec<&str> = outcome.proposals.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["ano", "pat"]);
    }

    #[test]
    fn limit_truncates_and_sets_has_more() {
        let conn = seed_conn();
        for i in 0..3 {
            insert(
                &conn,
                &format!("p{i}"),
                "pattern",
                "active",
                0.9,
                &format!("2026-06-1{i}T00:00:00Z"),
                "2026-07-10T00:00:00Z",
            );
        }

        let limited = query_proposals(&conn, &filter(ProposalStatus::Active, vec![], 2)).unwrap();
        assert_eq!(
            limited.total, 3,
            "total reflects the full match set, not the page"
        );
        assert_eq!(limited.proposals.len(), 2);
        assert!(limited.has_more);

        let full = query_proposals(&conn, &filter(ProposalStatus::Active, vec![], 5)).unwrap();
        assert!(!full.has_more);
    }

    #[test]
    fn found_envelope_has_historical_shape() {
        let conn = seed_conn();
        insert(
            &conn,
            "p1",
            "pattern",
            "active",
            0.9,
            "2026-06-10T00:00:00Z",
            "2026-07-10T00:00:00Z",
        );
        let outcome = query_proposals(
            &conn,
            &filter(ProposalStatus::Active, vec![ProposalType::Pattern], 20),
        )
        .unwrap();

        // Pass a non-canonical status label to prove the envelope emits the
        // canonical parsed status the caller supplies, not raw user input.
        let env = found_envelope(
            Path::new(".anvil/ember.db"),
            ProposalStatus::Active.as_str(),
            &[ProposalType::Pattern],
            &outcome,
        );

        assert_eq!(env["database_found"], json!(true));
        assert_eq!(env["total"], json!(1));
        assert_eq!(env["limit"], json!(20));
        assert_eq!(env["has_more"], json!(false));
        assert_eq!(env["filters"]["status"], json!("active"));
        assert_eq!(env["filters"]["type"], json!(["pattern"]));
        assert_eq!(env["proposals"][0]["id"], json!("p1"));
        assert_eq!(env["proposals"][0]["type"], json!("pattern"));
        assert_eq!(env["proposals"][0]["confidence"], json!(0.9));
        // Null columns are omitted (mirroring `deserialiseRow`'s undefined fields).
        assert!(env["proposals"][0].get("metadata").is_none());
        assert_eq!(env["proposals"][0]["signals"], json!([]));
    }

    #[test]
    fn missing_envelope_matches_node_five_key_shape() {
        let env = missing_envelope(Path::new(".anvil/ember.db"));
        // Exactly the historical Node.js keys — no limit/has_more/filters.
        assert_eq!(env["database_found"], json!(false));
        assert_eq!(env["total"], json!(0));
        assert_eq!(env["proposals"], json!([]));
        assert!(env["database_path"].is_string());
        assert!(env["error"].is_string());
        let keys: Vec<&str> = env
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys.len(), 5, "exactly 5 keys, got {keys:?}");
        assert!(env.get("limit").is_none());
        assert!(env.get("has_more").is_none());
        assert!(env.get("filters").is_none());
    }

    #[test]
    fn parse_status_and_type_reject_unknown() {
        assert!(
            parse_status("rejected").is_err(),
            "`rejected` is not a real status"
        );
        assert!(parse_status("active").is_ok());
        assert!(
            parse_type("observation").is_err(),
            "`observation` is not a real type"
        );
        assert!(parse_type("decision").is_ok());
    }

    #[test]
    fn parse_csv_types_trims_and_skips_blanks() {
        let parsed = parse_csv_types(Some(" pattern , decision ,")).unwrap();
        assert_eq!(parsed, vec![ProposalType::Pattern, ProposalType::Decision]);
        assert!(parse_csv_types(None).unwrap().is_empty());
        // A mix of valid and invalid short-circuits to an error.
        assert!(
            parse_csv_types(Some("pattern,bad_type")).is_err(),
            "mixed valid+invalid types must fail"
        );
    }

    #[test]
    fn parse_limit_rejects_zero() {
        assert_eq!(parse_limit("5").unwrap(), 5);
        assert!(parse_limit("0").is_err(), "zero is not a positive limit");
        assert!(parse_limit("notanumber").is_err());
    }

    #[test]
    fn truncate_caps_width() {
        assert_eq!(truncate("short", 12), "short");
        assert_eq!(truncate("an-extremely-long-id", 12), "an-extreme..");
    }
}
