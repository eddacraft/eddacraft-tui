<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Hono Semantic Pack (Track 4)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| PACKHON | —     | Draft  |

**Last reviewed:** 2026-04-26

> Note (2026-04-26): "TS substrate" = TS code being analysed. The pack itself
> ships as a Rust crate `crates/anvil-pack-hono/` per
> [ADR-027](../decisions/027-pack-architecture.md). `apps/anvil-api` is the
> dogfood target.

## Purpose

Per [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§8.4 row 5. Catches Hono framework anti-patterns layered on TS at T3.
Demand: 1 (Anvil's own `apps/anvil-api`). Blast: high. Strategic: supports.

Phase 2 deliverable (spec §9 step 13).

## In Scope

- Substrate language: TypeScript. Minimum substrate tier: T3.
- Pack activation: detect `hono` / `@hono/*` imports.
- Rule catalogue (per spec §8.4 row 5):
  - Routes missing auth middleware
  - `c.req.parseBody()` without size limits
  - CORS configured with `origin: '*'`
  - `c.html()` with interpolated values (XSS risk)
  - Error handlers leaking stack traces in responses
  - `c.env.SECRET` access without typed `Bindings` declaration
  - Route order bugs (`app.get('*')` registered before specific routes —
    catch-all shadows real routes)
  - Unvalidated `c.req.param()` / `c.req.query()` consumed into DB queries
  - Missing `@hono/zod-validator` on body-accepting routes (boundary
    application of Zod — coordinates with LANGTS Zod-creep rules)

## Out of Scope

- Hono RPC client analysis.
- Cloudflare-Workers / Bun / Deno runtime-specific rules beyond the above.
- Performance / latency advice.

## Interfaces

**Depends on:**

- [`lang-ts-audit`](../archive/modules/lang-ts-audit.aps.md) — TS at T3.
- [`pack-pulumi`](./pack-pulumi.aps.md) — first consumer of the pack
  architecture; PACKPUL-001 lands the crate registry.
- [ADR-027](../decisions/027-pack-architecture.md) — pack architecture
  (symbol-graph access required for route-order and middleware-presence
  reasoning).
- Coordination with LANGTS on Zod-boundary rules.

**Exposes:**

- Hono rule catalogue. Anvil's own API gets governed by this pack —
  dogfood case.

## Prerequisites

- `lang-ts-audit` complete.
- [ADR-027](../decisions/027-pack-architecture.md) Accepted; PACKPUL crate
  skeleton landed.

## Ready Checklist

Change status to **Ready** when:

- [ ] LANGTS complete.
- [ ] ADR-027 Accepted; PACKPUL-001 landed.
- [ ] `apps/anvil-api` baselined.
- [ ] Owner named.

## Work Items

Anticipated:

- PACKHON-001: `hono` import + project-shape detection.
- PACKHON-002: Auth-middleware-presence rule.
- PACKHON-003: Body-parsing limits + body-validator rule.
- PACKHON-004: CORS / response-header rules.
- PACKHON-005: HTML-interpolation rule (XSS).
- PACKHON-006: Error-handler stack-trace leak rule.
- PACKHON-007: Typed-`Bindings` rule on `c.env` access.
- PACKHON-008: Route-order rule.
- PACKHON-009: Param/query taint-into-DB rule.
- PACKHON-010: Validation against `apps/anvil-api`.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Route-order rule needs cross-route dataflow | Medium | Symbol-graph access from pack architecture (PACKPUL ADR) |
| Auth-middleware-presence rule requires per-project policy on what counts as "auth" | Medium | Per-project config — opt-in named middleware identifier(s) |
| Param/query taint-into-DB rule is a real taint analysis (expensive) | High | Phase 1: name-shape heuristic only; full taint is out of scope |
| Zod-boundary overlap with PACKNXT and LANGTS | Medium | Document scope in this module; one rule, applied at framework boundary |

## Open Questions

- [ ] How is "auth middleware" identified in a project — convention or
      explicit config?
- [ ] Phase-1 taint heuristic vs eventual full taint analysis — separate
      module if/when promoted?
