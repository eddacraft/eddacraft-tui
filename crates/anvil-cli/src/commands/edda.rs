//! RCLI3-001: `anvil edda` — port of the historical Node.js Edda CLI.
//!
//! Today this implements `anvil edda list` (RCLI3-001). The command
//! reads memory objects from a `.anvil/edda/` store that the
//! TypeScript `MemoryStore` writes, applies filters (`--type`,
//! `--status`, `--confidence`, `--since`, `--limit`), sorts by
//! `created_at` descending, and renders either a human-readable
//! table or the JSON envelope existing scripts depend on.
//!
//! Storage layout (mirrors `packages/edda-stack/src/edda/memory-store.ts`):
//!
//! ```text
//! .anvil/edda/
//!   index.yaml                       — light index (id, type, status, ...)
//!   memories/<type>/<id>.yaml        — full memory objects
//! ```
//!
//! The Rust port renders the table from the index alone (fast path) and
//! loads each full memory file when `--json` is requested, matching the
//! Node implementation's `queryMemories` shape so consumers of the JSON
//! envelope (`storage_found`, `total`, `limit`, `has_more`, `filters`,
//! `memories`) keep working.

use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Utc};
use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct EddaArgs {
    #[command(subcommand)]
    command: EddaCommand,
}

#[derive(Debug, Subcommand)]
enum EddaCommand {
    /// List Edda memories with filtering.
    #[command(alias = "ls")]
    List(ListArgs),
    /// Show a single Edda memory with full metadata.
    Show(ShowArgs),
}

#[derive(Debug, Args)]
struct ListArgs {
    /// Output as JSON.
    #[arg(long)]
    json: bool,
    /// Filter by memory type (comma-separated for multiple).
    #[arg(long = "type", value_name = "TYPE")]
    types: Option<String>,
    /// Filter by memory status. Defaults to `active`.
    #[arg(long, default_value = "active")]
    status: String,
    /// Filter by confidence level(s) (low, medium, high; comma-separated).
    #[arg(long = "confidence", value_name = "LEVEL")]
    confidence: Option<String>,
    /// Filter by age; supports m (minutes), h (hours), d (days).
    /// Example: `30m`, `24h`, `7d`.
    #[arg(long, value_name = "DURATION")]
    since: Option<String>,
    /// Maximum memories to display.
    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Debug, Args)]
struct ShowArgs {
    /// Memory ID to display.
    id: String,
    /// Output as JSON.
    #[arg(long)]
    json: bool,
}

pub fn run(args: &EddaArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        EddaCommand::List(list_args) => run_list(list_args),
        EddaCommand::Show(show_args) => run_show(show_args, global),
    }
}

// ---------------------------------------------------------------------------
// Memory type / status / confidence (mirror the edda-stack zod schemas)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum MemoryType {
    Decision,
    Pattern,
    Constraint,
    Warning,
    Doctrine,
    Lesson,
}

impl MemoryType {
    fn as_str(self) -> &'static str {
        match self {
            MemoryType::Decision => "decision",
            MemoryType::Pattern => "pattern",
            MemoryType::Constraint => "constraint",
            MemoryType::Warning => "warning",
            MemoryType::Doctrine => "doctrine",
            MemoryType::Lesson => "lesson",
        }
    }
}

fn parse_memory_type(value: &str) -> Result<MemoryType> {
    match value.trim() {
        "decision" => Ok(MemoryType::Decision),
        "pattern" => Ok(MemoryType::Pattern),
        "constraint" => Ok(MemoryType::Constraint),
        "warning" => Ok(MemoryType::Warning),
        "doctrine" => Ok(MemoryType::Doctrine),
        "lesson" => Ok(MemoryType::Lesson),
        other => bail!(
            "invalid memory type: {other}; expected one of decision, pattern, constraint, warning, doctrine, lesson"
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum MemoryStatus {
    Active,
    Superseded,
    Retired,
}

impl MemoryStatus {
    fn as_str(self) -> &'static str {
        match self {
            MemoryStatus::Active => "active",
            MemoryStatus::Superseded => "superseded",
            MemoryStatus::Retired => "retired",
        }
    }
}

fn parse_memory_status(value: &str) -> Result<MemoryStatus> {
    match value.trim() {
        "active" => Ok(MemoryStatus::Active),
        "superseded" => Ok(MemoryStatus::Superseded),
        "retired" => Ok(MemoryStatus::Retired),
        other => {
            bail!("invalid memory status: {other}; expected one of active, superseded, retired")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ConfidenceLevel {
    Low,
    Medium,
    High,
}

fn parse_confidence(value: &str) -> Result<ConfidenceLevel> {
    match value.trim() {
        "low" => Ok(ConfidenceLevel::Low),
        "medium" => Ok(ConfidenceLevel::Medium),
        "high" => Ok(ConfidenceLevel::High),
        other => bail!("invalid confidence level: {other}; expected one of low, medium, high"),
    }
}

// ---------------------------------------------------------------------------
// Storage shapes — minimal subset of the YAML files. We deliberately do
// NOT pin every field because the Node writer may add fields in future
// (schema_version is already on the wire); preserving forward-compat is
// cheaper than re-deriving every change.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MemoryIndex {
    memories: Vec<MemoryIndexEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MemoryIndexEntry {
    id: String,
    #[serde(rename = "type")]
    memory_type: String,
    status: String,
    path: String,
    #[serde(default)]
    statement: Option<String>,
    #[serde(default)]
    confidence: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    created_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MemoryRecord {
    #[serde(flatten)]
    rest: serde_yaml::Value,
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

fn run_list(args: &ListArgs) -> Result<()> {
    let storage_path = workspace_storage_path();
    let parsed_status = parse_memory_status(&args.status)?;
    let parsed_types = parse_csv(args.types.as_deref(), parse_memory_type)?;
    let parsed_confidence = parse_csv(args.confidence.as_deref(), parse_confidence)?;
    let created_after = args.since.as_deref().map(parse_since).transpose()?;

    if !storage_path.exists() {
        let envelope = missing_storage_envelope(
            &storage_path,
            &args.status,
            parsed_types.as_slice(),
            parsed_confidence.as_slice(),
            args.since.as_deref(),
            args.limit,
        );
        if args.json {
            println!("{envelope}");
        } else {
            println!("No Edda storage found at {}", storage_path.display());
        }
        // Match the historical Node.js CLI: missing storage is a
        // CliError, so callers (scripts, CI) can distinguish "no
        // memories yet" from "no store at all".
        bail!("No Edda storage found at {}", storage_path.display());
    }

    let index = load_index(&storage_path)?;
    let mut filtered: Vec<&MemoryIndexEntry> = index
        .memories
        .iter()
        .filter(|entry| {
            matches_filters(
                entry,
                parsed_status,
                &parsed_types,
                &parsed_confidence,
                created_after.as_ref(),
            )
        })
        .collect();
    // Sort by created_at descending; stable tiebreak on id ascending.
    filtered.sort_by(|a, b| {
        let by_created = b
            .created_at
            .as_deref()
            .unwrap_or("")
            .cmp(a.created_at.as_deref().unwrap_or(""));
        by_created.then_with(|| a.id.cmp(&b.id))
    });

    let total = filtered.len();
    let limit = args.limit;
    let displayed_entries: Vec<&MemoryIndexEntry> = filtered.iter().take(limit).copied().collect();
    let has_more = total > displayed_entries.len();

    if args.json {
        let memories: Vec<Value> = displayed_entries
            .iter()
            .map(|entry| match read_memory(&storage_path, entry) {
                Ok(record) => serde_json::to_value(&record.rest).unwrap_or(Value::Null),
                Err(_) => Value::Null,
            })
            .collect();
        let envelope = json!({
            "storage_found": true,
            "storage_path": storage_path.display().to_string(),
            "total": total,
            "limit": limit,
            "has_more": has_more,
            "filters": filters_payload(&args.status, parsed_types.as_slice(), parsed_confidence.as_slice(), args.since.as_deref()),
            "memories": memories,
        });
        println!("{envelope}");
        return Ok(());
    }

    render_table(
        &displayed_entries,
        total,
        &args.status,
        &parsed_types,
        &parsed_confidence,
        args.since.as_deref(),
    );
    Ok(())
}

fn run_show(args: &ShowArgs, global: &GlobalArgs) -> Result<()> {
    let storage_path = workspace_storage_path();
    let json_output = args.json || global.json;
    if !storage_path.exists() {
        if json_output {
            println!(
                "{}",
                show_error_envelope(
                    &format!("No Edda storage found at {}", storage_path.display()),
                    Some(false),
                )
            );
        }
        bail!("No Edda storage found at {}", storage_path.display());
    }

    let payload = match show_memory_payload(&storage_path, &args.id) {
        Ok(payload) => payload,
        Err(err) => {
            if json_output {
                println!("{}", show_error_envelope(&err.to_string(), None));
            }
            return Err(err);
        }
    };
    if json_output {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        render_show(&payload);
    }
    Ok(())
}

fn workspace_storage_path() -> PathBuf {
    // Match the historical Node.js CLI: anchor to the working
    // directory (the operator's project root). Walking up to find a
    // workspace marker would be more clever, but the demo runbook
    // explicitly calls `anvil edda list` from the repo root so the
    // simple anchor is the contract.
    PathBuf::from(".anvil").join("edda")
}

fn parse_csv<T>(value: Option<&str>, parse: impl Fn(&str) -> Result<T>) -> Result<Vec<T>> {
    let Some(raw) = value else {
        return Ok(Vec::new());
    };
    raw.split(',').map(|part| parse(part.trim())).collect()
}

/// `--since 7d` / `24h` / `30m` -> the lower bound timestamp.
/// Matches `parseSince` in the historical Node.js CLI list.ts.
fn parse_since(raw: &str) -> Result<DateTime<Utc>> {
    let trimmed = raw.trim();
    let (amount_str, unit) = trimmed.split_at(
        trimmed
            .char_indices()
            .find(|(_, c)| !c.is_ascii_digit())
            .map_or(trimmed.len(), |(i, _)| i),
    );
    let amount: i64 = amount_str
        .parse()
        .with_context(|| format!("invalid --since format: {raw}; expected e.g. 7d, 24h, 30m"))?;
    let duration = match unit {
        "d" => Duration::days(amount),
        "h" => Duration::hours(amount),
        "m" => Duration::minutes(amount),
        other => {
            bail!("invalid --since unit: {other}; expected d (days), h (hours), or m (minutes)")
        }
    };
    Ok(Utc::now() - duration)
}

fn matches_filters(
    entry: &MemoryIndexEntry,
    status: MemoryStatus,
    types: &[MemoryType],
    confidences: &[ConfidenceLevel],
    created_after: Option<&DateTime<Utc>>,
) -> bool {
    if entry.status.as_str() != status.as_str() {
        return false;
    }
    if !types.is_empty() && !types.iter().any(|t| t.as_str() == entry.memory_type) {
        return false;
    }
    if !confidences.is_empty() {
        let Some(entry_conf) = entry.confidence.as_deref() else {
            return false;
        };
        if !confidences.iter().any(|c| match c {
            ConfidenceLevel::Low => entry_conf == "low",
            ConfidenceLevel::Medium => entry_conf == "medium",
            ConfidenceLevel::High => entry_conf == "high",
        }) {
            return false;
        }
    }
    if let Some(threshold) = created_after {
        let Some(created_raw) = entry.created_at.as_deref() else {
            return false;
        };
        let Ok(parsed_created) = DateTime::parse_from_rfc3339(created_raw) else {
            return false;
        };
        if parsed_created.with_timezone(&Utc) <= *threshold {
            return false;
        }
    }
    true
}

fn load_index(storage_path: &Path) -> Result<MemoryIndex> {
    let index_path = storage_path.join("index.yaml");
    let raw = fs::read_to_string(&index_path)
        .with_context(|| format!("failed to read {}", index_path.display()))?;
    serde_yaml::from_str::<MemoryIndex>(&raw)
        .with_context(|| format!("failed to parse {}", index_path.display()))
}

/// Resolve an index entry path under `storage_path` without allowing escape.
///
/// Index YAML is trusted only as a catalogue of relative memory files. Absolute
/// paths, `..` components, and symlink targets that leave the Edda storage
/// directory are rejected so `edda list` / `edda show` cannot be used as a
/// confused deputy to read arbitrary YAML on disk.
fn resolve_memory_path(storage_path: &Path, entry_path: &str) -> Result<PathBuf> {
    let rel = Path::new(entry_path);
    if rel.as_os_str().is_empty() {
        bail!(
            "Edda index path must be relative and stay under the storage directory: {entry_path:?}"
        );
    }
    if rel.is_absolute() {
        bail!(
            "Edda index path must be relative and stay under the storage directory: {entry_path}"
        );
    }

    let mut has_normal = false;
    for component in rel.components() {
        match component {
            // Strict normal-only segments: reject `.`, `..`, roots, and prefixes.
            Component::Normal(_) => has_normal = true,
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                bail!(
                    "Edda index path must be relative and stay under the storage directory: {entry_path}"
                );
            }
        }
    }
    if !has_normal {
        bail!(
            "Edda index path must be relative and stay under the storage directory: {entry_path}"
        );
    }

    let candidate = storage_path.join(rel);
    let storage_canon = dunce::canonicalize(storage_path).with_context(|| {
        format!(
            "failed to resolve Edda storage directory {}",
            storage_path.display()
        )
    })?;

    // When the target exists (including via symlink), re-check after resolution
    // so a symlink under storage cannot point outside the Edda root.
    if candidate.exists() {
        let target_canon = dunce::canonicalize(&candidate).with_context(|| {
            format!("failed to resolve Edda memory path {}", candidate.display())
        })?;
        if target_canon.strip_prefix(&storage_canon).is_err() {
            bail!(
                "Edda index path must be relative and stay under the storage directory: {entry_path}"
            );
        }
        return Ok(target_canon);
    }

    Ok(candidate)
}

fn read_memory(storage_path: &Path, entry: &MemoryIndexEntry) -> Result<MemoryRecord> {
    let path = resolve_memory_path(storage_path, &entry.path)?;
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_yaml::from_str::<MemoryRecord>(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))
}

fn show_memory_payload(storage_path: &Path, id: &str) -> Result<Value> {
    let index = load_index(storage_path)?;
    let entry = index
        .memories
        .iter()
        .find(|entry| entry.id == id)
        .with_context(|| format!("Edda memory not found: {id}"))?;
    let record = read_memory(storage_path, entry)?;
    serde_json::to_value(record.rest).context("failed to serialise Edda memory")
}

fn show_error_envelope(message: &str, storage_found: Option<bool>) -> Value {
    let mut envelope = serde_json::Map::new();
    envelope.insert("error".to_string(), Value::String(message.to_string()));
    if let Some(storage_found) = storage_found {
        envelope.insert("storage_found".to_string(), Value::Bool(storage_found));
    }
    Value::Object(envelope)
}

fn filters_payload(
    status: &str,
    types: &[MemoryType],
    confidences: &[ConfidenceLevel],
    since: Option<&str>,
) -> Value {
    let type_value: Value = if types.is_empty() {
        Value::Null
    } else {
        Value::Array(
            types
                .iter()
                .map(|t| Value::String(t.as_str().to_owned()))
                .collect(),
        )
    };
    let confidence_value: Value = if confidences.is_empty() {
        Value::Null
    } else {
        Value::Array(
            confidences
                .iter()
                .map(|c| {
                    Value::String(
                        match c {
                            ConfidenceLevel::Low => "low",
                            ConfidenceLevel::Medium => "medium",
                            ConfidenceLevel::High => "high",
                        }
                        .to_owned(),
                    )
                })
                .collect(),
        )
    };
    json!({
        "status": status,
        "type": type_value,
        "confidence": confidence_value,
        "since": since,
    })
}

fn missing_storage_envelope(
    storage_path: &Path,
    status: &str,
    types: &[MemoryType],
    confidences: &[ConfidenceLevel],
    since: Option<&str>,
    limit: usize,
) -> Value {
    json!({
        "error": format!("No Edda storage found at {}", storage_path.display()),
        "storage_found": false,
        "storage_path": storage_path.display().to_string(),
        "total": 0,
        "limit": limit,
        "has_more": false,
        "filters": filters_payload(status, types, confidences, since),
        "memories": [],
    })
}

fn render_table(
    entries: &[&MemoryIndexEntry],
    total: usize,
    status: &str,
    types: &[MemoryType],
    confidences: &[ConfidenceLevel],
    since: Option<&str>,
) {
    println!();
    println!("Edda Memories");
    let mut filter_parts: Vec<String> = vec![
        format!("status: {status}"),
        format!(
            "type: {}",
            if types.is_empty() {
                "all".to_owned()
            } else {
                types
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ),
    ];
    if !confidences.is_empty() {
        filter_parts.push(format!(
            "confidence: {}",
            confidences
                .iter()
                .map(|c| match c {
                    ConfidenceLevel::Low => "low",
                    ConfidenceLevel::Medium => "medium",
                    ConfidenceLevel::High => "high",
                })
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(since) = since {
        filter_parts.push(format!("since: {since}"));
    }
    println!("{} found  |  {}", total, filter_parts.join("  |  "));
    println!(
        "  {:<14} {:<11} {:<12} {:<12} {:<48} {:<16}",
        "ID", "Type", "Status", "Confidence", "Statement", "Created",
    );
    if entries.is_empty() {
        println!("  No memories match the current filters.");
        println!();
        return;
    }
    for entry in entries {
        let id = truncate(&entry.id, 12);
        let statement = truncate(entry.statement.as_deref().unwrap_or(""), 46);
        let created = entry
            .created_at
            .as_deref()
            .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
            .map_or_else(
                || "—".to_owned(),
                |dt| format_relative_time(&dt.with_timezone(&Utc)),
            );
        println!(
            "  {:<14} {:<11} {:<12} {:<12} {:<48} {:<16}",
            id,
            entry.memory_type,
            entry.status,
            entry.confidence.as_deref().unwrap_or(""),
            statement,
            created,
        );
    }
    println!();
}

fn render_show(payload: &Value) {
    println!();
    println!("Memory: {}", field_string(payload, "id").unwrap_or("—"));
    println!("Type:   {}", field_string(payload, "type").unwrap_or("—"));
    println!("Status: {}", field_string(payload, "status").unwrap_or("—"));

    print_show_section("Statement", payload.get("statement"));
    print_show_section("Context", payload.get("context"));
    print_confidence_section(payload.get("confidence"));
    print_show_section("Attribution", payload.get("attribution"));
    print_show_section("Provenance", payload.get("provenance"));
    print_show_section("Evolution", payload.get("evolution"));
    println!();
}

fn field_string<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(Value::as_str)
}

fn print_confidence_section(value: Option<&Value>) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() {
        return;
    }
    println!();
    println!("Confidence");
    match value {
        Value::String(level) => println!("  Level: {level}"),
        other => print_section_value(other),
    }
}

fn print_show_section(label: &str, value: Option<&Value>) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() {
        return;
    }
    println!();
    println!("{label}");
    print_section_value(value);
}

fn print_section_value(value: &Value) {
    match value {
        Value::String(value) => println!("  {value}"),
        Value::Object(fields) => {
            for (key, value) in fields {
                println!(
                    "  {:<12} {}",
                    format!("{}:", human_label(key)),
                    display_value(value)
                );
            }
        }
        other => println!("  {}", display_value(other)),
    }
}

fn human_label(key: &str) -> String {
    key.replace('_', " ")
}

fn display_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(display_value)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| format!("{}: {}", human_label(key), display_value(value)))
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(width.saturating_sub(2)).collect();
    out.push_str("..");
    out
}

fn format_relative_time(value: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(*value);
    let abs = diff.num_seconds().abs();
    if abs < 60 {
        return if diff.num_seconds() >= 0 {
            "just now".to_owned()
        } else {
            "soon".to_owned()
        };
    }
    if abs < 3600 {
        let minutes = abs / 60;
        return if diff.num_seconds() >= 0 {
            format!("{minutes}m ago")
        } else {
            format!("in {minutes}m")
        };
    }
    if abs < 86_400 {
        let hours = abs / 3600;
        return if diff.num_seconds() >= 0 {
            format!("{hours}h ago")
        } else {
            format!("in {hours}h")
        };
    }
    let days = abs / 86_400;
    if diff.num_seconds() >= 0 {
        format!("{days}d ago")
    } else {
        format!("in {days}d")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_since_accepts_canonical_units() {
        // Just confirm each unit parses without panicking; the
        // absolute timestamp depends on Utc::now() so we don't assert
        // a value, only that the parser shape pins the contract.
        for input in ["30m", "24h", "7d", "1d"] {
            parse_since(input).unwrap_or_else(|err| panic!("`{input}` must parse: {err}"));
        }
    }

    #[test]
    fn parse_since_rejects_bad_units_and_bad_amounts() {
        assert!(parse_since("seven days").is_err());
        assert!(parse_since("7w").is_err(), "weeks are not supported");
        assert!(parse_since("d7").is_err(), "amount must precede unit");
    }

    #[test]
    fn matches_filters_default_status_active_excludes_superseded() {
        let entry = MemoryIndexEntry {
            id: "m1".into(),
            memory_type: "decision".into(),
            status: "superseded".into(),
            path: "memories/decision/m1.yaml".into(),
            statement: Some("x".into()),
            confidence: Some("high".into()),
            tags: None,
            created_at: Some("2026-01-01T00:00:00Z".into()),
        };
        assert!(
            !matches_filters(&entry, MemoryStatus::Active, &[], &[], None),
            "default --status active must hide superseded memories",
        );
    }

    #[test]
    fn matches_filters_type_csv_matches_any_of() {
        let entry = MemoryIndexEntry {
            id: "m1".into(),
            memory_type: "lesson".into(),
            status: "active".into(),
            path: "memories/lesson/m1.yaml".into(),
            statement: Some("x".into()),
            confidence: Some("low".into()),
            tags: None,
            created_at: Some("2026-01-01T00:00:00Z".into()),
        };
        assert!(matches_filters(
            &entry,
            MemoryStatus::Active,
            &[MemoryType::Decision, MemoryType::Lesson],
            &[],
            None,
        ));
        assert!(!matches_filters(
            &entry,
            MemoryStatus::Active,
            &[MemoryType::Decision],
            &[],
            None,
        ));
    }

    #[test]
    fn truncate_with_ellipsis_at_max_width() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghijkl", 6), "abcd..");
    }

    #[test]
    fn show_memory_payload_loads_full_record_by_id() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = tmp.path().join(".anvil").join("edda");
        fs::create_dir_all(storage.join("memories/decision")).unwrap();
        fs::write(
            storage.join("index.yaml"),
            r#"memories:
  - id: edda-demo
    type: decision
    status: active
    path: memories/decision/edda-demo.yaml
    statement: Prefer boring tests
    confidence: high
    tags: [testing, rust]
    created_at: "2026-05-01T00:00:00Z"
"#,
        )
        .unwrap();
        fs::write(
            storage.join("memories/decision/edda-demo.yaml"),
            r"id: edda-demo
type: decision
status: active
statement: Prefer boring tests
context: Keeps command ports small and verifiable.
confidence: high
tags:
  - testing
  - rust
provenance:
  ember_id: emb-123
evolution:
  supersedes: []
  superseded_by: []
",
        )
        .unwrap();

        let payload = show_memory_payload(&storage, "edda-demo").unwrap();

        assert_eq!(payload["id"], "edda-demo");
        assert_eq!(payload["statement"], "Prefer boring tests");
        assert_eq!(
            payload["context"],
            "Keeps command ports small and verifiable."
        );
        assert_eq!(payload["provenance"]["ember_id"], "emb-123");
    }

    #[test]
    fn show_memory_payload_reports_unknown_id() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = tmp.path().join(".anvil").join("edda");
        fs::create_dir_all(&storage).unwrap();
        fs::write(storage.join("index.yaml"), "memories: []\n").unwrap();

        let err = show_memory_payload(&storage, "missing")
            .unwrap_err()
            .to_string();

        assert!(err.contains("Edda memory not found: missing"));
    }

    fn seed_escape_index(storage: &Path, id: &str, path: &str) {
        fs::create_dir_all(storage).unwrap();
        fs::write(
            storage.join("index.yaml"),
            format!(
                r#"memories:
  - id: {id}
    type: decision
    status: active
    path: {path}
    statement: should-not-load
    confidence: high
    tags: []
    created_at: "2026-05-01T00:00:00Z"
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn show_memory_payload_rejects_parent_dir_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = tmp.path().join(".anvil").join("edda");
        let outside = tmp.path().join("secret.yaml");
        fs::write(&outside, "id: leaked\nstatement: TOP_SECRET_PARENT\n").unwrap();
        seed_escape_index(&storage, "escape-parent", "../../secret.yaml");

        let err = show_memory_payload(&storage, "escape-parent")
            .expect_err("parent-dir index path must be refused")
            .to_string();

        assert!(
            err.contains("must be relative and stay under the storage directory"),
            "unexpected error: {err}"
        );
        assert!(
            !err.contains("TOP_SECRET_PARENT"),
            "error must not include outside file contents"
        );
    }

    #[test]
    fn show_memory_payload_rejects_absolute_path_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = tmp.path().join(".anvil").join("edda");
        let outside = tmp.path().join("secret.yaml");
        fs::write(&outside, "id: leaked\nstatement: TOP_SECRET_ABS\n").unwrap();
        seed_escape_index(&storage, "escape-abs", &outside.to_string_lossy());

        let err = show_memory_payload(&storage, "escape-abs")
            .expect_err("absolute index path must be refused")
            .to_string();

        assert!(
            err.contains("must be relative and stay under the storage directory"),
            "unexpected error: {err}"
        );
        assert!(!err.contains("TOP_SECRET_ABS"));
    }

    #[cfg(unix)]
    #[test]
    fn show_memory_payload_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let storage = tmp.path().join(".anvil").join("edda");
        fs::create_dir_all(storage.join("memories/decision")).unwrap();

        let outside = tmp.path().join("secret.yaml");
        fs::write(&outside, "id: leaked\nstatement: TOP_SECRET_SYMLINK\n").unwrap();

        let link = storage.join("memories/decision/escape.yaml");
        symlink(&outside, &link).unwrap();
        seed_escape_index(&storage, "escape-link", "memories/decision/escape.yaml");

        let err = show_memory_payload(&storage, "escape-link")
            .expect_err("symlink escape must be refused")
            .to_string();

        assert!(
            err.contains("must be relative and stay under the storage directory"),
            "unexpected error: {err}"
        );
        assert!(!err.contains("TOP_SECRET_SYMLINK"));
    }

    #[test]
    fn resolve_memory_path_accepts_normal_relative_path() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = tmp.path().join(".anvil").join("edda");
        fs::create_dir_all(storage.join("memories/decision")).unwrap();
        let memory = storage.join("memories/decision/ok.yaml");
        fs::write(&memory, "id: ok\n").unwrap();

        let resolved =
            resolve_memory_path(&storage, "memories/decision/ok.yaml").expect("valid path");
        assert_eq!(
            dunce::canonicalize(&resolved).unwrap(),
            dunce::canonicalize(&memory).unwrap()
        );
    }

    #[test]
    fn resolve_memory_path_rejects_curdir_component() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = tmp.path().join(".anvil").join("edda");
        fs::create_dir_all(&storage).unwrap();

        let err = resolve_memory_path(&storage, "./memories/decision/ok.yaml")
            .expect_err("`.` components must be refused")
            .to_string();
        assert!(
            err.contains("must be relative and stay under the storage directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn show_error_envelope_is_machine_readable() {
        let envelope = show_error_envelope("Edda memory not found: missing", None);

        assert_eq!(envelope["error"], "Edda memory not found: missing");
        assert!(envelope.get("storage_found").is_none());
        assert!(envelope.get("memories").is_none());

        let missing_store =
            show_error_envelope("No Edda storage found at .anvil/edda", Some(false));
        assert_eq!(missing_store["storage_found"], false);
    }
}
