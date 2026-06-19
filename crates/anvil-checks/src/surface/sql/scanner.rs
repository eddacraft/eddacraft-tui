//! SURFSQL-001 (file detection) + SURFSQL-002 (destructive-pattern
//! catalogue) for the SQL migrations governance surface.
//!
//! Track 3 surfaces are **pattern-catalogue** work, not parser work
//! (`plans/specs/2026-04-08-language-and-coverage-design.md` §8.3): we do
//! comment-aware statement splitting and token matching, not a full SQL
//! grammar. Phase 1 is Postgres-flavoured; dialect quirks are deferred per
//! `plans/modules/surface-sql-migrations.aps.md`.
//!
//! Suppressions reuse the canonical Rust antipattern parser per
//! [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md)
//! via [`super::suppression`]; the SQL `--` comment style is already part of
//! that parser's grammar.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::suppression::resolve_line_suppression;

/// SURFSQL-002 — destructive / irreversible SQL operations in migrations.
pub const SURFSQL_002_RULE_ID: &str = "SURFSQL-002";

/// Migration-directory conventions recognised in addition to the bare
/// `.sql` extension. Matched case-insensitively against any path component.
const MIGRATION_DIRS: &[&str] = &["migrations", "db/migrations", "supabase/migrations"];

/// True when `path` is a SQL file SURFSQL should scan: any `*.sql` file, or
/// an *extensionless* file living under a recognised migration directory
/// (some tools use bare versioned names). A file under a migration directory
/// that carries a non-`.sql` extension (`.py`, `.md`, `.json`, …) is not a
/// SQL migration.
///
/// This is the precise per-file selector. The OPSUP-006 file-presence guard
/// (which keys on coarser globs including `migrations/**`) is only a cheap
/// pre-filter — a repo whose migration directory holds no SQL still pays a
/// near-zero walk and yields no scanned files here.
#[must_use]
pub fn is_sql_migration_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str());
    if ext.is_some_and(|e| e.eq_ignore_ascii_case("sql")) {
        return true;
    }
    // Under a migration directory, treat only *extensionless* files as SQL
    // (some tools use bare versioned names like `001_init`). A file with a
    // non-`.sql` extension is not a SQL migration — this avoids SQL-scanning
    // Django/Alembic `.py` migrations or runbook `.md`/`.json` living under a
    // `migrations/` directory.
    if ext.is_some() {
        return false;
    }
    let normalised = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    MIGRATION_DIRS.iter().any(|dir| {
        normalised.contains(&format!("/{dir}/")) || normalised.starts_with(&format!("{dir}/"))
    })
}

/// Canonical *full-schema definition* basenames. A file with one of these
/// names that does **not** live under a migration directory is a
/// generated or applied-once-to-a-fresh-database schema snapshot (Rails
/// `structure.sql`, sqlx/Ecto output, and hand-rolled `schema.sql`), not an
/// incremental migration. A file literally named `schema.sql` *inside* a
/// migration directory is still a migration step and is scanned normally.
const SCHEMA_DEFINITION_BASENAMES: &[&str] = &["schema.sql", "structure.sql"];

/// Header markers emitted by database dump tools. Their presence anywhere in
/// the file marks tool-generated output regardless of filename — likewise not
/// an authored migration.
const DUMP_CONTENT_MARKERS: &[&str] = &[
    "PostgreSQL database dump",
    "Dumped from database version",
    "MySQL dump",
    "MariaDB dump",
];

/// True when `display_path`/`content` identify a **schema dump or canonical
/// schema-definition file** rather than an incremental migration.
///
/// Such files are generated, or applied once to a fresh database; running the
/// migration-hygiene (SURFSQL-003) or destructive (SURFSQL-002) catalogues
/// against them is a false positive — a fresh-schema `CREATE TABLE`
/// legitimately omits `IF NOT EXISTS` because it is never re-run. The SURFSQL
/// surface governs *migrations*; a non-migration is out of its scope, so
/// [`scan_sql_all`] skips these files entirely. To have such a file scanned,
/// place it under a migration directory or give it a versioned migration name.
#[must_use]
pub fn is_schema_dump(display_path: &str, content: &str) -> bool {
    let normalised = display_path.replace('\\', "/").to_ascii_lowercase();
    let basename = normalised.rsplit('/').next().unwrap_or(normalised.as_str());
    let under_migration_dir = MIGRATION_DIRS.iter().any(|dir| {
        normalised.contains(&format!("/{dir}/")) || normalised.starts_with(&format!("{dir}/"))
    });
    if SCHEMA_DEFINITION_BASENAMES.contains(&basename) && !under_migration_dir {
        return true;
    }
    DUMP_CONTENT_MARKERS.iter().any(|m| content.contains(m))
}

/// The kind of destructive operation a [`SqlFinding`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DestructiveKind {
    /// `DROP TABLE` — irreversible table + data loss (flagged with or
    /// without `IF EXISTS`; the guard only affects idempotency, not the
    /// data loss).
    DropTable,
    /// `ALTER TABLE … DROP COLUMN` without an `IF EXISTS` guard.
    DropColumn,
    /// `TRUNCATE` — removes all rows, not transaction-logged per row.
    Truncate,
    /// `DELETE` with no `WHERE` clause — deletes every row.
    DeleteWithoutWhere,
    /// `UPDATE` with no `WHERE` clause — rewrites every row.
    UpdateWithoutWhere,
    /// `ALTER TABLE … DROP CONSTRAINT` without an `IF EXISTS` guard.
    DropConstraint,
}

impl DestructiveKind {
    /// Human-readable summary used in the finding message.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::DropTable => "DROP TABLE drops a table and all its data irreversibly",
            Self::DropColumn => "DROP COLUMN without IF EXISTS drops column data irreversibly",
            Self::Truncate => "TRUNCATE removes every row in the table",
            Self::DeleteWithoutWhere => "DELETE without a WHERE clause removes every row",
            Self::UpdateWithoutWhere => "UPDATE without a WHERE clause rewrites every row",
            Self::DropConstraint => "DROP CONSTRAINT without IF EXISTS removes a constraint",
        }
    }
}

/// A single SURFSQL-002 finding anchored to the line where the offending
/// statement begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlFinding {
    pub file: String,
    /// 1-indexed line where the offending statement starts.
    pub line: usize,
    pub kind: DestructiveKind,
    /// The offending statement, whitespace-collapsed and truncated for
    /// display — never the whole file.
    pub statement: String,
    /// Move-resistant identity over the **full** (untruncated) normalised
    /// statement plus the rule kind (SURFSQL-006). Used by the drift baseline
    /// so two long statements that share a truncated-display prefix do not
    /// collide; computed here because the untruncated statement is only
    /// available at scan time.
    pub fingerprint: String,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Scan one SQL file's `content` for **both** SURFSQL-002 (destructive) and
/// SURFSQL-003 (schema-hygiene) findings, splitting + normalising statements
/// a single time. This is the path [`super::run_surfsql_check`] uses; the
/// per-rule [`scan_sql_file`] / [`scan_sql_hygiene`] delegate here so the
/// split work is never duplicated as more rules land.
///
/// `display_path` is used only for the `file` field on findings. Findings
/// directly preceded by a matching `-- @anvil-ignore <ID>` directive are
/// returned with `suppressed = true` rather than dropped.
#[must_use]
pub fn scan_sql_all(
    display_path: &str,
    content: &str,
) -> (Vec<SqlFinding>, Vec<SqlHygieneFinding>) {
    // Schema dumps / canonical schema-definition files are not migrations;
    // scanning them produces false positives (SURFSQL-008).
    if is_schema_dump(display_path, content) {
        return (Vec::new(), Vec::new());
    }
    let lines: Vec<&str> = content.lines().collect();
    let mut destructive = Vec::new();
    let mut hygiene = Vec::new();

    for statement in split_statements(content) {
        for kind in classify(&statement.normalised) {
            let (suppressed, reason) =
                resolve_line_suppression(&lines, statement.line, SURFSQL_002_RULE_ID);
            destructive.push(SqlFinding {
                file: display_path.to_string(),
                line: statement.line,
                kind,
                statement: truncate_statement(&statement.normalised),
                fingerprint: statement_fingerprint(&format!("D:{kind:?}"), &statement.normalised),
                suppressed,
                suppression_reason: reason,
            });
        }
        for kind in classify_hygiene(&statement.normalised) {
            let (suppressed, reason) =
                resolve_line_suppression(&lines, statement.line, SURFSQL_003_RULE_ID);
            hygiene.push(SqlHygieneFinding {
                file: display_path.to_string(),
                line: statement.line,
                kind,
                statement: truncate_statement(&statement.normalised),
                fingerprint: statement_fingerprint(&format!("H:{kind:?}"), &statement.normalised),
                suppressed,
                suppression_reason: reason,
            });
        }
    }
    (destructive, hygiene)
}

/// Scan one SQL file's `content` for destructive operations (SURFSQL-002).
#[must_use]
pub fn scan_sql_file(display_path: &str, content: &str) -> Vec<SqlFinding> {
    scan_sql_all(display_path, content).0
}

/// SURFSQL-003 — schema-hygiene rule: idempotent DDL that omits its guard.
pub const SURFSQL_003_RULE_ID: &str = "SURFSQL-003";

/// The kind of schema-hygiene issue a [`SqlHygieneFinding`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HygieneKind {
    /// `CREATE TABLE` without `IF NOT EXISTS` — not idempotent; a re-run
    /// (or a partially-applied migration) errors instead of no-op'ing.
    MissingCreateTableGuard,
    /// `CREATE INDEX` without `IF NOT EXISTS`.
    MissingCreateIndexGuard,
}

impl HygieneKind {
    /// Human-readable summary used in the finding message.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::MissingCreateTableGuard => "CREATE TABLE without IF NOT EXISTS is not idempotent",
            Self::MissingCreateIndexGuard => "CREATE INDEX without IF NOT EXISTS is not idempotent",
        }
    }
}

/// A single SURFSQL-003 schema-hygiene finding, anchored to the statement's
/// start line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlHygieneFinding {
    pub file: String,
    pub line: usize,
    pub kind: HygieneKind,
    pub statement: String,
    /// Move-resistant identity over the **full** normalised statement plus the
    /// rule kind (SURFSQL-006); see [`SqlFinding::fingerprint`].
    pub fingerprint: String,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Scan one SQL file's `content` for schema-hygiene issues (SURFSQL-003).
///
/// Only `CREATE TABLE`/`CREATE INDEX` are checked — these support
/// `IF NOT EXISTS`. `CREATE OR REPLACE` constructs (views, functions) use a
/// different idempotency idiom and are out of scope. Suppression uses
/// `-- @anvil-ignore SURFSQL-003`.
#[must_use]
pub fn scan_sql_hygiene(display_path: &str, content: &str) -> Vec<SqlHygieneFinding> {
    scan_sql_all(display_path, content).1
}

/// Modifier keywords that may sit between `CREATE` and the object keyword
/// (`CREATE OR REPLACE`, `CREATE TEMP TABLE`, `CREATE UNIQUE INDEX`, …).
const CREATE_MODIFIERS: &[&str] = &[
    "OR",
    "REPLACE",
    "TEMP",
    "TEMPORARY",
    "UNLOGGED",
    "GLOBAL",
    "LOCAL",
    "UNIQUE",
    "MATERIALIZED",
];

/// Classify a normalised statement into zero or more schema-hygiene kinds.
///
/// The object keyword is matched **positionally** — the first token after
/// `CREATE` that is not a known modifier. This avoids false positives where
/// `TABLE`/`INDEX` appears later as an identifier (a column/parameter/table
/// named `index`, a `CREATE TRIGGER … ON index`, a view alias `AS index`).
fn classify_hygiene(norm: &str) -> Vec<HygieneKind> {
    let tokens: Vec<&str> = norm.split(' ').filter(|t| !t.is_empty()).collect();
    let mut kinds = Vec::new();
    if tokens.first() != Some(&"CREATE") {
        return kinds;
    }
    // `IF NOT EXISTS` anywhere in the (string-elided) statement = guarded.
    if contains_seq(&tokens, &["IF", "NOT", "EXISTS"]) {
        return kinds;
    }
    // The object keyword is the first non-modifier token after CREATE.
    let object = tokens
        .iter()
        .skip(1)
        .find(|t| !CREATE_MODIFIERS.contains(t));
    match object {
        Some(&"TABLE") => kinds.push(HygieneKind::MissingCreateTableGuard),
        Some(&"INDEX") => kinds.push(HygieneKind::MissingCreateIndexGuard),
        _ => {}
    }
    kinds
}

/// Display cap for the echoed statement so a multi-line `CREATE TABLE`
/// doesn't bloat the finding payload.
const STATEMENT_DISPLAY_CAP: usize = 120;

/// Move-resistant identity for a finding (SURFSQL-006): a 16-hex FNV-1a digest
/// of `tag` (a rule discriminator) plus the **full** normalised statement. The
/// statement reaching here is already upper-cased + whitespace-collapsed, so
/// re-indenting or re-casing a flagged statement keeps the same fingerprint,
/// while two distinct long statements never collide (the whole statement is
/// hashed, not the truncated display form). Allocation-light: bytes are hashed
/// in place, with no intermediate buffers. Deterministic across platforms
/// (unlike `DefaultHasher`).
#[must_use]
pub(crate) fn statement_fingerprint(tag: &str, normalised: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a 64-bit offset basis.
    for byte in tag
        .bytes()
        .chain(std::iter::once(b'|'))
        .chain(normalised.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3); // FNV prime.
    }
    format!("{hash:016x}")
}

fn truncate_statement(normalised: &str) -> String {
    if normalised.len() <= STATEMENT_DISPLAY_CAP {
        return normalised.to_string();
    }
    // Walk back to a UTF-8 char boundary so a multi-byte identifier straddling
    // the byte cap can't panic the slice.
    let mut end = STATEMENT_DISPLAY_CAP;
    while end > 0 && !normalised.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &normalised[..end])
}

/// A SQL statement extracted from a file: its normalised (uppercased,
/// whitespace-collapsed, comment-stripped) form plus the 1-indexed line it
/// started on in the source.
struct SqlStatement {
    normalised: String,
    line: usize,
}

/// Split `content` into statements on `;`, stripping `--` line comments,
/// (nested) `/* … */` block comments, and single-quoted string-literal
/// content first. Line numbers of the surviving statement text are preserved
/// so findings anchor to the real source line.
///
/// String content is elided (not just left intact) so a `--`/`;` inside a
/// literal can't split or merge statements and a keyword inside a literal
/// (`'DROP TABLE x'`, `'see WHERE clause'`) can't reach the classifier.
/// Dollar-quoted bodies (`$$ … $$`, function/DO blocks) are a documented
/// Phase-1 gap — their inner statements are scanned as if top-level.
fn split_statements(content: &str) -> Vec<SqlStatement> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut current_line: Option<usize> = None;
    let mut line = 1usize;

    let mut chars = content.chars().peekable();
    // Postgres allows *nested* block comments, so track depth, not a bool.
    let mut block_depth = 0usize;
    let mut in_line_comment = false;
    // Inside a single-quoted string literal: comment/terminator characters
    // there are data, not syntax, and the content must not reach the
    // classifier (else `'DROP TABLE x'` or `'see WHERE clause'` mis-fires).
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if c == '\n' {
            line += 1;
            in_line_comment = false;
            current.push(' ');
            continue;
        }
        if in_line_comment {
            continue;
        }
        if block_depth > 0 {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_depth -= 1;
            } else if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                block_depth += 1;
            }
            continue;
        }
        if in_string {
            // Postgres escapes an embedded quote by doubling it ('').
            if c == '\'' {
                if chars.peek() == Some(&'\'') {
                    chars.next(); // escaped quote — stay in the string
                } else {
                    in_string = false; // closing quote
                }
            }
            // Elide all string content from the statement text.
            continue;
        }
        if c == '\'' {
            if current_line.is_none() {
                current_line = Some(line);
            }
            in_string = true;
            current.push(' '); // placeholder keeps adjoining tokens separate
            continue;
        }
        // Enter comments.
        if c == '-' && chars.peek() == Some(&'-') {
            chars.next();
            in_line_comment = true;
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_depth += 1;
            // Replace the comment with a separator so tokens on either side
            // don't fuse (`IF/*x*/EXISTS` → `IF EXISTS`, not `IFEXISTS`).
            current.push(' ');
            continue;
        }
        if c == ';' {
            flush(&mut statements, &mut current, &mut current_line);
            continue;
        }
        if !c.is_whitespace() && current_line.is_none() {
            current_line = Some(line);
        }
        current.push(c);
    }
    flush(&mut statements, &mut current, &mut current_line);
    statements
}

fn flush(out: &mut Vec<SqlStatement>, current: &mut String, current_line: &mut Option<usize>) {
    let normalised = normalise(current);
    if let (false, Some(start)) = (normalised.is_empty(), *current_line) {
        out.push(SqlStatement {
            normalised,
            line: start,
        });
    }
    current.clear();
    *current_line = None;
}

/// Uppercase + collapse runs of whitespace to a single space.
fn normalise(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}

/// Classify a normalised statement into zero or more destructive kinds.
fn classify(norm: &str) -> Vec<DestructiveKind> {
    let tokens: Vec<&str> = norm.split(' ').filter(|t| !t.is_empty()).collect();
    let mut kinds = Vec::new();

    if contains_seq(&tokens, &["DROP", "TABLE"]) {
        kinds.push(DestructiveKind::DropTable);
    }
    if contains_seq(&tokens, &["TRUNCATE"]) {
        kinds.push(DestructiveKind::Truncate);
    }
    if seq_without_guard(&tokens, &["DROP", "COLUMN"]) {
        kinds.push(DestructiveKind::DropColumn);
    }
    if seq_without_guard(&tokens, &["DROP", "CONSTRAINT"]) {
        kinds.push(DestructiveKind::DropConstraint);
    }
    if contains_seq(&tokens, &["DELETE", "FROM"]) && !tokens.contains(&"WHERE") {
        kinds.push(DestructiveKind::DeleteWithoutWhere);
    }
    // UPDATE must be the *first* token: this deliberately misses a CTE-led
    // `WITH … UPDATE …` (documented Phase-1 false negative) in exchange for not
    // false-positiving on `ON UPDATE SET NULL`/`ON UPDATE CASCADE` foreign-key
    // actions, which are common and safe in real migrations.
    if tokens.first() == Some(&"UPDATE") && tokens.contains(&"SET") && !tokens.contains(&"WHERE") {
        kinds.push(DestructiveKind::UpdateWithoutWhere);
    }
    kinds
}

/// True when `tokens` contains `needle` as a consecutive subsequence.
fn contains_seq(tokens: &[&str], needle: &[&str]) -> bool {
    seq_index(tokens, needle).is_some()
}

/// True when `tokens` contains *any* occurrence of `needle` that is **not**
/// immediately followed by an `IF EXISTS` guard. Checking every occurrence
/// matters for comma-chained operations like
/// `DROP COLUMN IF EXISTS a, DROP COLUMN b` — the second drop is unguarded.
fn seq_without_guard(tokens: &[&str], needle: &[&str]) -> bool {
    let mut start = 0;
    while let Some(rel) = seq_index(&tokens[start..], needle) {
        let idx = start + rel;
        let after = idx + needle.len();
        let guarded = tokens.get(after) == Some(&"IF") && tokens.get(after + 1) == Some(&"EXISTS");
        if !guarded {
            return true;
        }
        start = idx + 1;
    }
    false
}

fn seq_index(tokens: &[&str], needle: &[&str]) -> Option<usize> {
    if needle.is_empty() || needle.len() > tokens.len() {
        return None;
    }
    tokens
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(content: &str) -> Vec<DestructiveKind> {
        scan_sql_file("m.sql", content)
            .into_iter()
            .map(|f| f.kind)
            .collect()
    }

    #[test]
    fn finding_fingerprint_is_move_resistant() {
        // Same statement, different whitespace/case → same fingerprint, so
        // re-indenting a flagged statement is not a new edge (SURFSQL-006).
        let a = scan_sql_file("m.sql", "DROP TABLE users;");
        let b = scan_sql_file("m.sql", "drop   table\n   users;");
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].fingerprint, b[0].fingerprint);
        assert_eq!(a[0].fingerprint.len(), 16, "16-hex digest");
        // A different table → a different fingerprint.
        let c = scan_sql_file("m.sql", "DROP TABLE accounts;");
        assert_ne!(a[0].fingerprint, c[0].fingerprint);
    }

    #[test]
    fn finding_fingerprint_does_not_collide_on_shared_display_prefix() {
        // Two long CREATE TABLEs that share the truncated (120-char) display
        // prefix but differ in their tail must not collide — the fingerprint
        // hashes the FULL statement, not the truncated display form.
        let prefix = format!("CREATE TABLE t ({}", "col_aaaaaaaa INT, ".repeat(9));
        let (_, h1) = scan_sql_all("m.sql", &format!("{prefix} unique_alpha INT);"));
        let (_, h2) = scan_sql_all("m.sql", &format!("{prefix} unique_omega INT);"));
        assert_eq!(h1.len(), 1);
        assert_eq!(h2.len(), 1);
        // Display statements collide (shared 120-char prefix) ...
        assert_eq!(
            h1[0].statement, h2[0].statement,
            "display forms share the prefix"
        );
        // ... but the full-statement fingerprints must not.
        assert_ne!(
            h1[0].fingerprint, h2[0].fingerprint,
            "distinct long statements must not share a baseline identity"
        );
    }

    #[test]
    fn detects_sql_files_by_extension_and_migration_dir() {
        assert!(is_sql_migration_file(Path::new("foo/bar.sql")));
        assert!(is_sql_migration_file(Path::new("FOO/BAR.SQL")));
        assert!(is_sql_migration_file(Path::new("db/migrations/001-init")));
        assert!(is_sql_migration_file(Path::new(
            "supabase/migrations/20260101_init"
        )));
        assert!(is_sql_migration_file(Path::new("migrations/001")));
        // Windows separators normalise.
        assert!(is_sql_migration_file(Path::new(
            "app\\db\\migrations\\001-init"
        )));
        // Not a SQL file and not under a migration dir.
        assert!(!is_sql_migration_file(Path::new("src/main.rs")));
        assert!(!is_sql_migration_file(Path::new("docs/migrations.md")));
        // Non-`.sql` files *inside* a migration dir are NOT SQL — Django/
        // Alembic Python migrations and runbook docs must not be SQL-scanned.
        assert!(!is_sql_migration_file(Path::new(
            "migrations/0001_initial.py"
        )));
        assert!(!is_sql_migration_file(Path::new("db/migrations/README.md")));
        assert!(!is_sql_migration_file(Path::new(
            "supabase/migrations/seed.json"
        )));
    }

    #[test]
    fn flags_drop_table_with_or_without_guard() {
        assert_eq!(kinds("DROP TABLE users;"), vec![DestructiveKind::DropTable]);
        assert_eq!(
            kinds("DROP TABLE IF EXISTS users;"),
            vec![DestructiveKind::DropTable]
        );
    }

    #[test]
    fn flags_truncate() {
        assert_eq!(kinds("TRUNCATE events;"), vec![DestructiveKind::Truncate]);
        assert_eq!(
            kinds("TRUNCATE TABLE events;"),
            vec![DestructiveKind::Truncate]
        );
    }

    #[test]
    fn drop_column_guard_awareness() {
        assert_eq!(
            kinds("ALTER TABLE users DROP COLUMN email;"),
            vec![DestructiveKind::DropColumn]
        );
        // Guarded — Anvil's own migrations use this; must NOT flag.
        assert!(kinds("ALTER TABLE IF EXISTS users DROP COLUMN IF EXISTS email;").is_empty());
    }

    #[test]
    fn drop_constraint_guard_awareness() {
        assert_eq!(
            kinds("ALTER TABLE users DROP CONSTRAINT users_pkey;"),
            vec![DestructiveKind::DropConstraint]
        );
        // Guarded form Anvil uses intentionally (002/013 migrations).
        assert!(
            kinds("ALTER TABLE beta_users DROP CONSTRAINT IF EXISTS beta_users_status_check;")
                .is_empty()
        );
    }

    #[test]
    fn delete_and_update_need_a_where_clause() {
        assert_eq!(
            kinds("DELETE FROM audit_log;"),
            vec![DestructiveKind::DeleteWithoutWhere]
        );
        assert!(kinds("DELETE FROM audit_log WHERE created_at < now();").is_empty());
        assert_eq!(
            kinds("UPDATE users SET active = true;"),
            vec![DestructiveKind::UpdateWithoutWhere]
        );
        assert!(kinds("UPDATE users SET active = true WHERE id = 1;").is_empty());
    }

    #[test]
    fn ignores_destructive_keywords_inside_comments() {
        // Line comment.
        assert!(kinds("-- DROP TABLE users;\nCREATE TABLE users (id int);").is_empty());
        // Block comment spanning lines.
        assert!(kinds("/* historical:\n   DROP TABLE users;\n*/\nSELECT 1;").is_empty());
    }

    #[test]
    fn does_not_false_match_substrings_or_identifiers() {
        // A column named like a keyword must not trip the catalogue.
        assert!(kinds("CREATE TABLE t (dropped boolean, update_count int);").is_empty());
        // "UPDATED_AT" column in a CREATE is not an UPDATE statement.
        assert!(kinds("CREATE TABLE t (updated_at timestamptz);").is_empty());
    }

    #[test]
    fn anchors_finding_to_statement_start_line() {
        let content = "CREATE TABLE ok (id int);\n\n\nDROP TABLE legacy;\n";
        let findings = scan_sql_file("m.sql", content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 4);
        assert_eq!(findings[0].kind, DestructiveKind::DropTable);
    }

    #[test]
    fn suppression_directive_marks_finding_suppressed() {
        let content =
            "-- @anvil-ignore SURFSQL-002 -- table already archived\nDROP TABLE legacy;\n";
        let findings = scan_sql_file("m.sql", content);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].suppressed);
        assert_eq!(
            findings[0].suppression_reason.as_deref(),
            Some("table already archived")
        );
    }

    #[test]
    fn clean_guarded_migration_produces_no_findings() {
        // Mirrors the shape of Anvil's real migrations (all guarded) — the
        // FP-avoidance contract that lets SURFSQL hit the <1% bar on the
        // dogfood corpus.
        let content = "CREATE TABLE IF NOT EXISTS device_codes (id uuid PRIMARY KEY);\n\
                       ALTER TABLE beta_users DROP CONSTRAINT IF EXISTS beta_users_status_check;\n\
                       ALTER TABLE IF EXISTS snaps DROP COLUMN IF EXISTS source;\n";
        assert!(
            scan_sql_file("003-auth.sql", content).is_empty(),
            "guarded migration must be clean"
        );
    }

    #[test]
    fn semicolon_inside_quoted_default_is_not_a_terminator() {
        // The `;` lives inside a string literal and must not split the
        // statement (string-literal awareness); no destructive op → clean.
        let content = "INSERT INTO settings (k, v) VALUES ('greeting', 'hi; there');\n";
        assert!(scan_sql_file("m.sql", content).is_empty());
    }

    #[test]
    fn long_multibyte_statement_does_not_panic() {
        // truncate_statement slices at a byte cap; a multi-byte identifier
        // straddling the cap must not panic (council CRITICAL).
        let filler = "a".repeat(115);
        let content = format!("DROP TABLE {filler}中中中中中;\n");
        let findings = scan_sql_file("m.sql", &content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, DestructiveKind::DropTable);
        assert!(findings[0].statement.ends_with('…'));
    }

    #[test]
    fn dash_inside_string_literal_does_not_merge_statements() {
        // `--` inside a literal must not start a line comment and swallow the
        // terminating `;` (council HIGH). Both statements are safe (have
        // WHERE) → no findings.
        let content = "UPDATE t SET note = 'a--b' WHERE id = 1;\nDELETE FROM t2 WHERE id = 2;\n";
        assert!(
            scan_sql_file("m.sql", content).is_empty(),
            "string-embedded -- must not corrupt splitting"
        );
    }

    #[test]
    fn keyword_inside_string_literal_is_not_flagged() {
        // 'DROP TABLE ...' inside an INSERT value is data, not a statement.
        let content = "INSERT INTO audit_log (msg) VALUES ('text DROP TABLE old_users text');\n";
        assert!(scan_sql_file("m.sql", content).is_empty());
    }

    #[test]
    fn where_inside_string_literal_does_not_mask_missing_where() {
        // A genuinely unguarded UPDATE whose value merely mentions "where"
        // must still be flagged (council false-negative).
        let content = "UPDATE config SET note = 'see WHERE clause for details';\n";
        assert_eq!(kinds(content), vec![DestructiveKind::UpdateWithoutWhere]);
    }

    #[test]
    fn second_unguarded_drop_column_in_chain_is_flagged() {
        // First drop is guarded, second is not — must still flag (council).
        let content = "ALTER TABLE t DROP COLUMN IF EXISTS a, DROP COLUMN b;\n";
        assert_eq!(kinds(content), vec![DestructiveKind::DropColumn]);
    }

    #[test]
    fn inline_block_comment_does_not_fuse_tokens() {
        // `IF/*x*/EXISTS` must read as a guard (IF EXISTS), not `IFEXISTS`
        // (council: stripping a comment without a separator broke detection).
        assert!(
            kinds("ALTER TABLE t DROP COLUMN IF/*c*/EXISTS email;").is_empty(),
            "inline comment must not break the IF EXISTS guard"
        );
        // And the inverse: a comment splitting a keyword must not hide it.
        assert_eq!(
            kinds("DROP/*c*/TABLE users;"),
            vec![DestructiveKind::DropTable]
        );
    }

    #[test]
    fn nested_block_comment_is_fully_ignored() {
        // Postgres allows nested block comments; the inner close must not
        // leak the trailing DROP TABLE back into live SQL (council).
        let content = "/* outer /* inner */ DROP TABLE users; */\nSELECT 1;\n";
        assert!(scan_sql_file("m.sql", content).is_empty());
    }

    // ── SURFSQL-003: schema hygiene (missing idempotency guards) ────

    fn hygiene_kinds(content: &str) -> Vec<HygieneKind> {
        scan_sql_hygiene("m.sql", content)
            .into_iter()
            .map(|f| f.kind)
            .collect()
    }

    #[test]
    fn flags_create_table_without_guard() {
        assert_eq!(
            hygiene_kinds("CREATE TABLE users (id int);"),
            vec![HygieneKind::MissingCreateTableGuard]
        );
        // Guarded — Anvil's own migrations use this; must be clean.
        assert!(hygiene_kinds("CREATE TABLE IF NOT EXISTS users (id int);").is_empty());
    }

    #[test]
    fn flags_create_index_without_guard() {
        assert_eq!(
            hygiene_kinds("CREATE UNIQUE INDEX idx ON users (email);"),
            vec![HygieneKind::MissingCreateIndexGuard]
        );
        assert!(hygiene_kinds("CREATE INDEX IF NOT EXISTS idx ON users (email);").is_empty());
    }

    #[test]
    fn hygiene_ignores_non_table_index_creates() {
        // CREATE OR REPLACE VIEW/FUNCTION use a different idempotency idiom.
        assert!(hygiene_kinds("CREATE OR REPLACE VIEW v AS SELECT 1;").is_empty());
        assert!(hygiene_kinds("CREATE SCHEMA app;").is_empty());
        assert!(hygiene_kinds("CREATE EXTENSION IF NOT EXISTS pgcrypto;").is_empty());
        assert!(hygiene_kinds("CREATE MATERIALIZED VIEW mv AS SELECT 1;").is_empty());
        // ALTER / DROP are not CREATE statements.
        assert!(hygiene_kinds("ALTER TABLE users ADD COLUMN x int;").is_empty());
    }

    #[test]
    fn hygiene_object_keyword_is_positional_not_anywhere() {
        // The keyword TABLE/INDEX appearing later as an identifier must NOT
        // trigger a finding (council false-positives).
        assert!(
            hygiene_kinds(
                "CREATE TRIGGER t AFTER INSERT ON index FOR EACH ROW EXECUTE FUNCTION f();"
            )
            .is_empty(),
            "CREATE TRIGGER ... ON index must not fire the index rule"
        );
        assert!(
            hygiene_kinds(
                "CREATE OR REPLACE VIEW summary AS SELECT count(*) AS index FROM events;"
            )
            .is_empty(),
            "a view column aliased `index` must not fire the index rule"
        );
        assert!(
            hygiene_kinds("CREATE FUNCTION check_status( index integer ) RETURNS void LANGUAGE sql AS $$ SELECT 1 $$;").is_empty(),
            "a function parameter named `index` must not fire the index rule"
        );
    }

    #[test]
    fn hygiene_covers_create_modifiers() {
        // Modifiers between CREATE and the object keyword still resolve.
        assert_eq!(
            hygiene_kinds("CREATE TEMP TABLE scratch (id int);"),
            vec![HygieneKind::MissingCreateTableGuard]
        );
        assert_eq!(
            hygiene_kinds("CREATE UNIQUE INDEX CONCURRENTLY idx ON t (a);"),
            vec![HygieneKind::MissingCreateIndexGuard]
        );
        assert_eq!(
            hygiene_kinds("CREATE TABLE archive AS SELECT * FROM events;"),
            vec![HygieneKind::MissingCreateTableGuard]
        );
    }

    #[test]
    fn hygiene_suppression_marks_finding() {
        let content = "-- @anvil-ignore SURFSQL-003 -- one-shot bootstrap table\nCREATE TABLE seed (id int);\n";
        let findings = scan_sql_hygiene("m.sql", content);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].suppressed);
        assert_eq!(
            findings[0].suppression_reason.as_deref(),
            Some("one-shot bootstrap table")
        );
    }

    // ── SURFSQL-008: schema-dump / canonical-schema scoping ─────────

    #[test]
    fn schema_definition_file_outside_migrations_is_skipped() {
        // The real Anvil case: a hand-authored full-schema file with bare
        // CREATE TABLEs at `db/schema.sql` is not an incremental migration,
        // so SURFSQL-003 must not fire (these were the 29 SURFSQL-007 FPs).
        let content = "-- Beta Access System Schema\nCREATE EXTENSION IF NOT EXISTS citext;\nCREATE TABLE beta_users (id uuid PRIMARY KEY);\nCREATE TABLE access_tokens (id uuid PRIMARY KEY);\n";
        let path = "apps/anvil-api/src/db/schema.sql";
        assert!(is_schema_dump(path, content));
        let (destructive, hygiene) = scan_sql_all(path, content);
        assert!(destructive.is_empty());
        assert!(
            hygiene.is_empty(),
            "a canonical schema.sql must not fire SURFSQL-003"
        );
    }

    #[test]
    fn structure_sql_is_treated_as_schema_definition() {
        // Rails / Ecto generated `structure.sql`.
        assert!(is_schema_dump(
            "db/structure.sql",
            "CREATE TABLE t (id int);"
        ));
    }

    #[test]
    fn schema_sql_under_migration_dir_is_still_scanned() {
        // A migration step that happens to be named schema.sql is a real
        // migration — scoping must not skip it.
        let content = "CREATE TABLE users (id int);";
        assert!(!is_schema_dump("db/migrations/schema.sql", content));
        let (_d, hygiene) = scan_sql_all("db/migrations/schema.sql", content);
        assert_eq!(
            hygiene.len(),
            1,
            "schema.sql inside a migration dir is still governed"
        );
    }

    #[test]
    fn pg_dump_header_marks_dump_regardless_of_filename() {
        // Tool-generated dump detected by header even when oddly named.
        let content = "--\n-- PostgreSQL database dump\n--\nCREATE TABLE t (id int);\n";
        assert!(is_schema_dump("backups/2026-06-18.sql", content));
        let (_d, hygiene) = scan_sql_all("backups/2026-06-18.sql", content);
        assert!(hygiene.is_empty());
    }

    #[test]
    fn ordinary_migration_is_unaffected_by_dump_scoping() {
        let content = "CREATE TABLE users (id int);";
        assert!(!is_schema_dump("migrations/001_init.sql", content));
        let (_d, hygiene) = scan_sql_all("migrations/001_init.sql", content);
        assert_eq!(hygiene.len(), 1);
    }
}
