<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# SQL Migrations Governance Surface (Track 3 — Phase 1)

| ID      | Owner      | Status      |
| ------- | ---------- | ----------- |
| SURFSQL | joshuaboys | In Progress |

**Last reviewed:** 2026-06-18

## Purpose

Bring `.sql` migration files to **T2 (Policy)** per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.2, §8.3 — pattern catalogue + suppression syntax + policy hook + drift
baseline. Ranked **#1** in Track 3 by demand × blast radius — User B Postgres
+ Anvil's own migrations, **critical** blast radius (schema/data destruction
cannot be undone), and the pack-unlock bonus that the future `pack-drizzle`
layers directly on this surface's catalogue.

This is the **Phase 1 / MVP** governance surface (spec §9 step 2). Ships
alongside `lang-ts-audit`, `pack-pulumi`, and `pack-llm-provider` to bound
the MVP cut.

## In Scope

- File detection: `*.sql`, plus recognised migration directory conventions
  (`migrations/`, `db/migrations/`, `supabase/migrations/`, etc.).
- Pattern catalogue (per spec §8.3 row 1):
  - `DROP TABLE` (with or without `IF EXISTS`)
  - `DROP COLUMN` without an explicit guard
  - `TRUNCATE`
  - `DELETE` without `WHERE`
  - `UPDATE` without `WHERE`
  - `ALTER TABLE … DROP CONSTRAINT`
  - Unversioned migrations (file naming convention violations)
  - Schema and data changes in the same transaction
  - Missing `IF NOT EXISTS` / `IF EXISTS` guards on idempotent ops
- Suppression syntax: `-- @anvil-ignore <ID> -- <reason>` (the canonical
  ADR-029 directive form; the reason clause is optional).
- Policy hook integration with the existing OPA pipeline.
- Drift baseline default-on for `.sql` files (per the schema-versioning
  story owned by [OPSUP](./operational-supplement.aps.md), council §16.5 #7).
- Acceptance bar per council §16.5 #9: FP rate < N% on Anvil's own repo
  AND ≥ 1 external codebase validation run.

## Out of Scope

- ORM-level patterns (`pack-drizzle` covers Drizzle-shaped patterns layered
  on this surface).
- SQL schema graph / dependency analysis (that is T3 "Resource-aware" — out
  of scope for Phase 1).
- Stored procedure body analysis.
- Database performance / query-plan analysis.
- Non-Postgres dialect quirks (start Postgres-flavoured; revisit when
  demand arrives).

## Interfaces

**Depends on:**

- Existing OPA pipeline.
- [`operational-supplement`](./operational-supplement.aps.md) — check
  registry IDs, drift schema versioning, per-track feature flag,
  file-presence guard, FP reporting channel.
- Rust suppression parser per
  [ADR-029](../decisions/029-suppression-parser-authority.md) — adds the
  `--` SQL comment style.

**Exposes:**

- SQL pattern catalogue — substrate for the future `pack-drizzle`.
- First reference implementation of "governance surface T2" — sets the
  pattern other Track 3 surfaces follow.

## Prerequisites

- [OPSUP](./operational-supplement.aps.md) check-registry slice + drift
  schema-versioning slice + per-track feature-flag slice landed (full
  OPSUP delivery not required — surfaces can move to Ready against the
  slices they need; see OPSUP §Risks).
- [ADR-029](../decisions/029-suppression-parser-authority.md) Accepted;
  Rust suppression parser teaches the `--` style.

## Ready Checklist

Promoted Draft → In Progress 2026-06-18. Checklist satisfied:

- [x] OPSUP slices above landed — OPSUP-001 (check-ID registry, Done), OPSUP-003
      (drift schema versioning, Merged #2694), OPSUP-005 (per-track flag
      taxonomy, Merged #2755 — `track.surface` umbrella available for gating).
- [x] ADR-029 Accepted — `--` SQL comment style already in the canonical
      antipattern suppression parser.
- [x] Anvil's own SQL migrations baselined; FP target N agreed — corpus is
      `apps/anvil-api/src/db/migrations/` (16 migrations + `schema.sql`), all
      destructive ops intentionally `IF EXISTS`-guarded; **FP target N = 1%**
      (per the PYLAN-009 precedent, operator-ratifiable). The guard-aware
      catalogue produces zero findings on this corpus (covered by
      `clean_guarded_migration_produces_no_findings`).
- [x] External codebase validation candidate identified — a Postgres-flavoured
      OSS project with `migrations/`/`supabase/migrations/` (final pick recorded
      in SURFSQL-007); SURFSQL-007 runs both.
- [x] Owner named — joshuaboys.

## Work Items

Delivered as a sequence of slices. PR1 lands the `anvil-checks` library surface
(detection + destructive catalogue + suppression); later slices add schema
hygiene, gate/catalogue registration, drift, and validation.

### SURFSQL-001 — SQL file detection + migration-directory heuristics

- **Status:** In Progress
- **Intent:** Identify the files SURFSQL governs without forcing config.
- **Expected Outcome:** `*.sql` files and files under recognised migration
  directories (`migrations/`, `db/migrations/`, `supabase/migrations/`) are
  detected; non-SQL files are not.
- **Files:** `crates/anvil-checks/src/surface/sql/scanner.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::sql::scanner::tests::detects_sql_files_by_extension_and_migration_dir`
- **Confidence:** high

### SURFSQL-002 — Destructive-pattern catalogue

- **Status:** In Progress
- **Intent:** Warn on irreversible / unguarded destructive operations.
- **Expected Outcome:** `DROP TABLE`, `TRUNCATE`, unguarded `DROP COLUMN`,
  unguarded `ALTER … DROP CONSTRAINT`, and `DELETE`/`UPDATE` without a `WHERE`
  clause are flagged; comment-embedded and `IF EXISTS`-guarded forms are not.
  Findings anchor to the offending statement's start line.
- **Files:** `crates/anvil-checks/src/surface/sql/scanner.rs`,
  `crates/anvil-checks/src/surface/sql/check.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::sql`
- **Confidence:** high

### SURFSQL-003 — Schema-hygiene catalogue

- **Status:** Ready
- **Intent:** Flag missing idempotency guards and mixed schema/data
  transactions.
- **Expected Outcome:** Missing `IF NOT EXISTS`/`IF EXISTS` on idempotent ops
  and schema+data changes in one transaction surface as findings, with `--`
  suppression.
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::sql` (new schema-hygiene cases)
- **Dependencies:** SURFSQL-001, SURFSQL-002
- **Confidence:** medium

### SURFSQL-004 — SQL `--` suppression syntax integration

- **Status:** In Progress
- **Intent:** Honour `-- @anvil-ignore <ID> -- <reason>` for SQL findings.
- **Expected Outcome:** A directive on the line above a finding (or in the file
  header) marks it suppressed with its reason, reusing the ADR-029 canonical
  parser; cross-rule leakage is rejected.
- **Files:** `crates/anvil-checks/src/surface/sql/suppression.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::sql::suppression`
- **Confidence:** high

### SURFSQL-005 — Gate/catalogue registration + policy hook wiring

- **Status:** Ready
- **Intent:** Surface SURFSQL in the gate behind `track.surface.sql`.
- **Expected Outcome:** A `ANV-SURF-SQL-001` check is registered in
  `check_catalog.rs` (relaxing the `ANV-CORE-`-only ID-validation test to
  accept the `ANV-SURF-<SURFACE>-NNN` scheme), wired into the gate dispatcher
  with file-presence guards, and gated behind the `track.surface.sql` leaf flag
  (createdFor SURFSQL-005) inheriting the OPSUP-005 `track.surface` umbrella.
- **Validation:** `cargo test -p eddacraft-anvil commands::check_catalog` + gate dispatch test asserting SURFSQL runs only when `track.surface.sql` resolves enabled
- **Dependencies:** SURFSQL-002, OPSUP-005 (Merged)
- **Confidence:** medium

### SURFSQL-006 — Drift baseline default-on for `.sql`

- **Status:** Ready
- **Intent:** Baseline existing destructive ops; warn only on new edges.
- **Expected Outcome:** SURFSQL declares its drift baseline fields via the
  OPSUP-003 schema-versioned model; pre-existing findings are baselined so only
  newly introduced destructive ops warn.
- **Validation:** `cargo test -p eddacraft-anvil drift` (SURFSQL baseline round-trip + new-edge-only warn)
- **Dependencies:** SURFSQL-002, OPSUP-003 (Merged)
- **Confidence:** medium

### SURFSQL-007 — Anvil-repo + external validation runs; FP report

- **Status:** Ready
- **Intent:** Prove the acceptance bar (FP < 1% on Anvil + ≥1 external repo).
- **Expected Outcome:** A validation run over Anvil's own migrations and one
  external Postgres codebase, with a recorded FP report meeting N = 1%.
- **Validation:** FP report committed under `plans/reviews/` showing < 1% on Anvil + ≥1 external repo
- **Dependencies:** SURFSQL-002, SURFSQL-005, SURFSQL-006
- **Confidence:** medium

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Postgres-bias produces FPs on MySQL/SQLite (council general FP concern) | Medium | Phase 1 ships Postgres-only; document dialect limitation |
| Migration-directory heuristic misses non-standard layouts | Low | Allow opt-in path config; fall back to `.sql` extension |
| Pattern catalogue noisy on Anvil's own pre-existing migrations | High | Pre-baseline before shipping; accept seed FPs as "drift baseline established" |
| LLM Provider pack and SQL surface compete for the same Phase 1 sprint | Medium | Strict Phase 1 line in spec §9 — these are co-required for MVP |

## Known limitations (Phase 1)

The detector is a comment- and string-aware token matcher, not a SQL grammar
(per spec §8.3). Documented gaps, surfaced in the PR1 council review:

- **CTE-led `UPDATE`** (`WITH … UPDATE t SET …` with no `WHERE`) is a false
  negative — `UPDATE` detection anchors on the statement's first token to avoid
  false-positives on `ON UPDATE` foreign-key actions (`ON UPDATE SET NULL`),
  which are common in real migrations. The FK-FP avoidance is the deliberate
  trade.
- **Dollar-quoted bodies** (`$$ … $$`, `DO`/function blocks) are scanned as if
  their inner statements were top-level; not yet body-aware.
- **Suppression** must sit on the line *immediately* above the statement (a
  blank line between directive and statement is not honoured) — consistent with
  SURFENV.

These are revisited if dogfood/external validation (SURFSQL-007) shows real
impact.

## Open Questions

- [ ] Postgres-only or accept MySQL/SQLite catalogues at Phase 1?
- [ ] Should `pack-drizzle` ship its own SQL-string interpolation rules or
      reuse SURFSQL's pattern catalogue?
- [ ] What counts as the "external codebase validation run" for council
      §16.5 #9 acceptance?
