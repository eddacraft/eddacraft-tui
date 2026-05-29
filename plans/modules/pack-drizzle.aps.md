<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Drizzle Semantic Pack (Track 4)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| PACKDRZ | —     | Draft  |

**Last reviewed:** 2026-04-26

> Note (2026-04-26): "TS substrate" = TS code being analysed. The pack itself
> ships as a Rust crate `crates/anvil-pack-drizzle/` per
> [ADR-027](../decisions/027-pack-architecture.md).

## Purpose

Per [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§8.4 row 2. Catches Drizzle-ORM anti-patterns layered on TS at T3. Demand: 1
(User B — Anvil itself uses raw SQL via NeonClient, not Drizzle, per council
correction C-001). Ranked #2 in Track 4 on **blast radius**: a `.delete()`
without `.where()` ships production data loss in one line.

Phase 2 deliverable (spec §9 step 8).

## In Scope

- Substrate language: TypeScript. Minimum substrate tier: T3.
- Pack activation: detect `drizzle-orm` / `drizzle-orm/*` imports.
- Rule catalogue (per spec §8.4 row 2):
  - `.delete()` without `.where()` (production data loss in one line)
  - `.update()` without `.where()`
  - Raw `sql\`…\`` template interpolation of user-controlled input
  - Missing transactions around multi-statement operations that should be
    atomic
  - Schema drift between `schema.ts` and actual migration history
  - `.execute()` on prepared statements without input validation

## Out of Scope

- Drizzle Kit migration generation analysis (separate concern).
- Schema-level performance recommendations.
- Cross-table referential-integrity inference.
- Non-Drizzle ORMs (Prisma, Kysely, etc. — separate packs only on demand).

## Interfaces

**Depends on:**

- [`lang-ts-audit`](./lang-ts-audit.aps.md) — TS at T3.
- [`pack-pulumi`](./pack-pulumi.aps.md) — first consumer of the pack
  architecture; PACKPUL-001 lands the crate registry this pack registers
  against.
- [ADR-027](../decisions/027-pack-architecture.md) — pack architecture.
- [`surface-sql-migrations`](./surface-sql-migrations.aps.md) — coordinate
  on raw-SQL rules; no duplication of destructive-pattern detection.

**Exposes:**

- Drizzle rule catalogue.

## Prerequisites

- `lang-ts-audit` complete.
- [ADR-027](../decisions/027-pack-architecture.md) Accepted; PACKPUL crate
  skeleton (PACKPUL-001) landed.
- Coordination with `surface-sql-migrations` to avoid rule duplication on
  raw `sql\`…\`` interpolation.

## Ready Checklist

Change status to **Ready** when:

- [ ] LANGTS complete.
- [ ] ADR-027 Accepted; PACKPUL-001 landed.
- [ ] SURFSQL coordination resolved (which side owns raw-SQL interpolation).
- [ ] Validation candidate identified (User B repo, since Anvil itself does
      not use Drizzle).
- [ ] Owner named.

## Work Items

Anticipated:

- PACKDRZ-001: `drizzle-orm` import detection.
- PACKDRZ-002: Where-less delete/update rule.
- PACKDRZ-003: Raw-SQL interpolation rule (coordinated with SURFSQL).
- PACKDRZ-004: Transaction-boundary rule.
- PACKDRZ-005: Schema-vs-migration drift rule.
- PACKDRZ-006: Validation against User B's Drizzle codebase.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| `.delete()` without `.where()` rule trips on intentional table-truncates | Medium | Require explicit `@anvil-ignore` with reason — that is the policy |
| Schema-vs-migration drift requires reading the migration filesystem | Medium | Phase 1 of this rule: only compare in-file; filesystem walk is a follow-up |
| Anvil cannot dogfood (no Drizzle in repo) — validation entirely external | Medium | Make User B validation a hard Ready gate |

## Open Questions

- [ ] Raw-SQL interpolation: PACKDRZ rule, SURFSQL rule, or both with
      different framings?
- [ ] Schema-drift detection scope — in-file only, or include migration
      filesystem walk?
