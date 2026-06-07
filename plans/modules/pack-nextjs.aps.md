<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Next.js Semantic Pack (Track 4)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| PACKNXT | —     | Draft  |

**Last reviewed:** 2026-04-26

> Note (2026-04-26): "TS substrate" = TS code being analysed. The pack itself
> ships as a Rust crate `crates/anvil-pack-nextjs/` per
> [ADR-027](../decisions/027-pack-architecture.md). `apps/website` is the
> dogfood target.

## Purpose

Per [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§8.4 row 3. Catches Next.js anti-patterns layered on TS at T3. Demand: 2
(User B + Anvil's `apps/website`). Blast: medium-high. Strategic: supports.

Phase 2 deliverable (spec §9 step 12).

## In Scope

- Substrate language: TypeScript. Minimum substrate tier: T3.
- Pack activation: detect `next` / `next/*` imports + Next.js project shape
  (`apps/*/next.config.*`, `app/` or `pages/` directories).
- Rule catalogue (per spec §8.4 row 3):
  - Raw HTML insertion via React's dangerous prop without explicit
    sanitisation
  - Server Components leaking secrets via props passed to client components
  - `revalidate` misconfigurations (revalidate=0 on cached routes,
    contradictory `dynamic` exports)
  - Middleware matchers reaching root routes unintentionally
  - Client components importing server-only modules
  - Server Actions without Zod (or equivalent) validation

## Out of Scope

- App Router vs Pages Router migration advice.
- Cache Components (Next 16+) policy beyond what's listed above.
- Image-optimisation policy.
- Performance budgets / Lighthouse-style scoring.

## Interfaces

**Depends on:**

- [`lang-ts-audit`](../archive/modules/lang-ts-audit.aps.md) — TS at T3.
- [`pack-pulumi`](./pack-pulumi.aps.md) — first consumer of the pack
  architecture; PACKPUL-001 lands the crate registry.
- [ADR-027](../decisions/027-pack-architecture.md) — pack architecture
  (symbol-graph access required for server/client boundary detection).
- Coordination with the future TS Zod-creep rules from `lang-ts-audit`
  (boundary application of Zod is a pack concern; presence of `z.any()`
  is a language concern).

**Exposes:**

- Next.js rule catalogue.

## Prerequisites

- `lang-ts-audit` complete.
- [ADR-027](../decisions/027-pack-architecture.md) Accepted; PACKPUL crate
  skeleton landed.

## Ready Checklist

Change status to **Ready** when:

- [ ] LANGTS complete.
- [ ] ADR-027 Accepted; PACKPUL-001 landed.
- [ ] Anvil's `apps/website` baselined.
- [ ] External Next.js validation candidate identified.
- [ ] Owner named.

## Work Items

Anticipated:

- PACKNXT-001: `next/*` and project-shape detection.
- PACKNXT-002: Dangerous-HTML-prop rule.
- PACKNXT-003: Server/client boundary rules (secret leaks via props,
  server-only imports in client).
- PACKNXT-004: Caching/`revalidate` rule.
- PACKNXT-005: Middleware-matcher rule.
- PACKNXT-006: Server Actions validation rule (Zod boundary application).
- PACKNXT-007: Validation against `apps/website` + external codebase.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Server/client boundary detection requires symbol-graph + `'use client'` directive awareness | High | Pack architecture must support symbol-graph access (PACKPUL-001 ADR) |
| App Router conventions evolve fast | Medium | Pin rules to documented Next behaviour at a specific version range |
| Zod-boundary rules duplicate the language-level Zod-creep rules | Medium | Coordinate scope with LANGTS — language for `z.any()`, pack for "validator missing at boundary" |

## Open Questions

- [ ] Cache Components (Next 16+) — separate pack or rule expansion here?
- [ ] How to scope "dangerous prop" rule across React-major-version
      changes?
