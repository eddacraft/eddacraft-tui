<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# SQL Migrations Governance Surface (Track 3 — Phase 1)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| SURFSQL | —     | Draft  |

**Last reviewed:** 2026-04-26

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
- Suppression syntax: `-- @anvil-ignore <ID>: <reason>`.
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

Change status to **Ready** when:

- [ ] OPSUP slices above landed.
- [ ] ADR-029 Accepted.
- [ ] Anvil's own SQL migrations baselined; FP target N agreed.
- [ ] External codebase validation candidate identified.
- [ ] Owner named.

## Work Items

Tasks will be defined when this module moves to Ready. Anticipated shape:

- SURFSQL-001: SQL file detection + migration-directory heuristics.
- SURFSQL-002: Destructive-pattern catalogue (DROP/TRUNCATE/DELETE without
  WHERE family).
- SURFSQL-003: Schema-hygiene catalogue (missing guards, mixed schema/data
  transactions).
- SURFSQL-004: SQL `--` suppression syntax integration.
- SURFSQL-005: Policy hook wiring.
- SURFSQL-006: Drift baseline default-on for `.sql`.
- SURFSQL-007: Anvil-repo + external validation runs; FP report.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Postgres-bias produces FPs on MySQL/SQLite (council general FP concern) | Medium | Phase 1 ships Postgres-only; document dialect limitation |
| Migration-directory heuristic misses non-standard layouts | Low | Allow opt-in path config; fall back to `.sql` extension |
| Pattern catalogue noisy on Anvil's own pre-existing migrations | High | Pre-baseline before shipping; accept seed FPs as "drift baseline established" |
| LLM Provider pack and SQL surface compete for the same Phase 1 sprint | Medium | Strict Phase 1 line in spec §9 — these are co-required for MVP |

## Open Questions

- [ ] Postgres-only or accept MySQL/SQLite catalogues at Phase 1?
- [ ] Should `pack-drizzle` ship its own SQL-string interpolation rules or
      reuse SURFSQL's pattern catalogue?
- [ ] What counts as the "external codebase validation run" for council
      §16.5 #9 acceptance?
