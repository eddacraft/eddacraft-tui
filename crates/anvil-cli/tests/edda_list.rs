//! RCLI3-001: `anvil edda list` integration tests.
//!
//! Pin the public contract the historical Node.js CLI shipped:
//! - reads from `.anvil/edda/` in the workspace root,
//! - filters by `--type` / `--status` / `--confidence` / `--since`,
//! - sorts by `created_at` descending, paginates with `--limit`,
//! - renders either a human table or a JSON envelope (`--json`).
//!
//! Storage layout we read against (written by the TS `MemoryStore`):
//!
//! ```text
//! .anvil/edda/
//!   index.yaml            — index of memory entries
//!   memories/<type>/<id>.yaml  — full memory objects
//! ```

use std::fs;
use std::path::Path;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Write the minimal index + memory pair that the `queryMemories` path
/// expects. The TS `MemoryStore` writes both files atomically; the Rust
/// `edda list` should be able to render against the index alone for
/// the table view and load full memories for JSON.
fn write_memory(
    storage: &Path,
    id: &str,
    memory_type: &str,
    status: &str,
    confidence: &str,
    statement: &str,
    created_at: &str,
) {
    let memories_dir = storage.join("memories").join(memory_type);
    fs::create_dir_all(&memories_dir).expect("create memories dir");

    let memory_yaml = format!(
        "id: {id}
type: {memory_type}
status: {status}
schema_version: 1
statement: {statement:?}
context:
  when: \"unknown\"
  why: \"test\"
  conditions: []
  tags: []
confidence: {confidence}
provenance:
  kindling_observations: []
  ember_source: null
attribution:
  promoted_by: \"tester\"
  promoted_at: \"{created_at}\"
evolution:
  supersedes: []
created_at: \"{created_at}\"
",
    );
    fs::write(memories_dir.join(format!("{id}.yaml")), memory_yaml).expect("write memory");
}

fn write_index(storage: &Path, entries: &[(&str, &str, &str, &str, &str, &str)]) {
    use std::fmt::Write as _;

    // (id, type, status, statement, confidence, created_at)
    let mut content = String::from("memories:\n");
    for (id, ty, status, statement, confidence, created_at) in entries {
        write!(
            content,
            "  - id: {id}\n    type: {ty}\n    status: {status}\n    path: memories/{ty}/{id}.yaml\n    statement: {statement:?}\n    confidence: {confidence}\n    tags: []\n    created_at: \"{created_at}\"\n",
        )
        .expect("write index entry");
    }
    content.push_str("updated_at: \"2026-05-17T00:00:00Z\"\n");
    fs::write(storage.join("index.yaml"), content).expect("write index");
}

/// **Contract:** with no `.anvil/edda/` storage, JSON mode emits the
/// `storage_found: false` envelope the historical Node.js CLI does and
/// exits non-zero. Operators / scripts that rely on the JSON shape to
/// detect missing stores must keep working.
#[test]
fn edda_list_json_with_no_storage_emits_storage_found_false_envelope() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["edda", "list", "--json"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .output()
        .expect("invoke anvil");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!("stdout must be JSON in --json mode; got: {stdout}\nerror: {err}")
    });

    assert_eq!(
        payload["storage_found"], false,
        "missing store must set storage_found=false",
    );
    assert_eq!(payload["total"], 0);
    assert!(
        payload["memories"]
            .as_array()
            .expect("memories array")
            .is_empty(),
        "missing store must return an empty memories array",
    );
    assert!(
        !output.status.success(),
        "missing store must exit non-zero so scripts can detect it",
    );
}

/// **Contract:** the default status filter is `active`. Superseded
/// memories must NOT appear in the default list. Mirrors the
/// `parsedStatus = parseStatus(options.status ?? 'active')` default
/// in the historical Node.js CLI.
#[test]
fn edda_list_json_default_status_filters_to_active() {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = dir.path().join(".anvil").join("edda");
    fs::create_dir_all(&storage).unwrap();

    write_memory(
        &storage,
        "mem-active",
        "decision",
        "active",
        "high",
        "active decision",
        "2026-05-01T00:00:00Z",
    );
    write_memory(
        &storage,
        "mem-super",
        "decision",
        "superseded",
        "high",
        "superseded decision",
        "2026-05-01T00:00:00Z",
    );
    write_index(
        &storage,
        &[
            (
                "mem-active",
                "decision",
                "active",
                "active decision",
                "high",
                "2026-05-01T00:00:00Z",
            ),
            (
                "mem-super",
                "decision",
                "superseded",
                "superseded decision",
                "high",
                "2026-05-01T00:00:00Z",
            ),
        ],
    );

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["edda", "list", "--json"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .output()
        .expect("invoke anvil");

    assert!(
        output.status.success(),
        "list with storage must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["total"], 1, "only the active memory must remain");
    let memories = payload["memories"].as_array().expect("memories");
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0]["id"], "mem-active");
}

/// **Contract:** `--type` accepts a comma-separated list and filters
/// the result set. The historical Node.js CLI mirrored
/// `MemoryQuery.types`; the Rust port must keep the same shape.
#[test]
fn edda_list_json_filters_by_comma_separated_type() {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = dir.path().join(".anvil").join("edda");
    fs::create_dir_all(&storage).unwrap();

    write_memory(
        &storage,
        "mem-d",
        "decision",
        "active",
        "high",
        "a decision",
        "2026-05-01T00:00:00Z",
    );
    write_memory(
        &storage,
        "mem-p",
        "pattern",
        "active",
        "medium",
        "a pattern",
        "2026-05-02T00:00:00Z",
    );
    write_memory(
        &storage,
        "mem-l",
        "lesson",
        "active",
        "low",
        "a lesson",
        "2026-05-03T00:00:00Z",
    );
    write_index(
        &storage,
        &[
            (
                "mem-d",
                "decision",
                "active",
                "a decision",
                "high",
                "2026-05-01T00:00:00Z",
            ),
            (
                "mem-p",
                "pattern",
                "active",
                "a pattern",
                "medium",
                "2026-05-02T00:00:00Z",
            ),
            (
                "mem-l",
                "lesson",
                "active",
                "a lesson",
                "low",
                "2026-05-03T00:00:00Z",
            ),
        ],
    );

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["edda", "list", "--json", "--type", "decision,lesson"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .output()
        .expect("invoke anvil");

    assert!(output.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["total"], 2);
    let ids: Vec<&str> = payload["memories"]
        .as_array()
        .expect("memories")
        .iter()
        .map(|m| m["id"].as_str().expect("id"))
        .collect();
    // Sort order is created_at desc, so the lesson comes first.
    assert_eq!(ids, vec!["mem-l", "mem-d"]);
}

/// **Contract:** `--limit` caps the rendered set; `total` still
/// reflects the unfiltered match count and `has_more` flips when the
/// limit is hit. Pinned because operators rely on `has_more` to drive
/// follow-up requests.
#[test]
fn edda_list_json_limit_sets_has_more_true_when_truncating() {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = dir.path().join(".anvil").join("edda");
    fs::create_dir_all(&storage).unwrap();

    for i in 0..3 {
        let id = format!("mem-{i:02}");
        let created = format!("2026-05-{:02}T00:00:00Z", i + 1);
        write_memory(&storage, &id, "decision", "active", "high", "x", &created);
    }
    write_index(
        &storage,
        &[
            (
                "mem-00",
                "decision",
                "active",
                "x",
                "high",
                "2026-05-01T00:00:00Z",
            ),
            (
                "mem-01",
                "decision",
                "active",
                "x",
                "high",
                "2026-05-02T00:00:00Z",
            ),
            (
                "mem-02",
                "decision",
                "active",
                "x",
                "high",
                "2026-05-03T00:00:00Z",
            ),
        ],
    );

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["edda", "list", "--json", "--limit", "2"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .output()
        .expect("invoke anvil");

    assert!(output.status.success());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["total"], 3, "total reflects full matches");
    assert_eq!(payload["limit"], 2);
    assert_eq!(payload["has_more"], true);
    assert_eq!(payload["memories"].as_array().unwrap().len(), 2);
}

/// **Contract:** the human-readable (non-JSON) table contains every
/// shipped column header — ID / Type / Status / Confidence / Statement
/// / Created — and at least one row per matching memory. Keeps the
/// operator-facing output stable against accidental column drops.
#[test]
fn edda_list_table_contains_expected_columns_and_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = dir.path().join(".anvil").join("edda");
    fs::create_dir_all(&storage).unwrap();
    write_memory(
        &storage,
        "mem-x",
        "decision",
        "active",
        "high",
        "hello world",
        "2026-05-01T00:00:00Z",
    );
    write_index(
        &storage,
        &[(
            "mem-x",
            "decision",
            "active",
            "hello world",
            "high",
            "2026-05-01T00:00:00Z",
        )],
    );

    let output = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .args(["edda", "list"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .output()
        .expect("invoke anvil");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for column in ["ID", "Type", "Status", "Confidence", "Statement", "Created"] {
        assert!(
            stdout.contains(column),
            "column {column:?} must appear in the table; got:\n{stdout}",
        );
    }
    assert!(stdout.contains("mem-x"), "row must appear; got:\n{stdout}");
    assert!(stdout.contains("hello world"), "statement must appear");
}
