<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Pulumi Semantic Pack (Track 4 — Phase 1)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| PACKPUL | —     | Draft  |

**Last reviewed:** 2026-04-26

> Note (2026-04-26): "TS substrate" = the language being analysed by this pack.
> The pack itself ships as a Rust crate `crates/anvil-pack-pulumi/` per
> [ADR-027](../decisions/027-pack-architecture.md). Anvil's own `infra/`
> directory (Pulumi TS) is the dogfood target.

## Purpose

First semantic pack, layered on TypeScript. Per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§8.4 row 1, this pack catches infrastructure-as-code anti-patterns inside
Pulumi TS programs. Demand: 2 (User B + Anvil's own `infra/`). Blast:
**critical**. Strategic: supports.

This module also **sets the pack architecture pattern** — it is the first
pack to land, so the council §16.5 #4 decision (symbol-graph access vs
content-only, named crate location) must be made and recorded in
[ADR-027](../decisions/027-pack-architecture.md) before tasks are written
for this module. ADR-027 is currently `Proposed`; advancing it to
`Accepted` is part of this module's Ready Checklist.

Phase 1 deliverable (spec §9 step 3). Unblocks immediately after
`lang-ts-audit` completes.

## In Scope

- Substrate language: TypeScript. Minimum substrate tier: T3.
- File-level pack activation: detect Pulumi programs by `@pulumi/*`
  imports (per spec §12.5 — corner cases like conditional imports and
  re-exports are implementation detail).
- Rule catalogue (per spec §8.4 row 1):
  - `acl: "public-read"` on S3 buckets / similar
  - Wide IAM trust policies (`Principal: "*"` on roles)
  - `versioning` disabled on state-holding resources
  - Stack-crossing resource references that bypass `StackReference`
  - Hardcoded secrets in resource definitions
- Pack architecture decision recorded — see
  [ADR-027](../decisions/027-pack-architecture.md) (symbol-graph access,
  per-pack crate location, compiled-in activation). PACKPUL is the first
  consumer of this ADR.
- Test against Anvil's own `infra/` Pulumi program — first acceptance
  validation.

## Out of Scope

- Pulumi state inspection / live-stack analysis.
- Cross-language Pulumi (Python, Go, .NET, YAML) — Phase 2+ extension if
  demand arrives.
- AWS/Azure/GCP-specific compliance frameworks beyond the rules above.
- Drift detection between Pulumi state and live cloud resources.

## Interfaces

**Depends on:**

- [`lang-ts-audit`](../archive/modules/lang-ts-audit.aps.md) — TS substrate at T3.
- Pack architecture (this module sets the pattern).
- Existing OPA pipeline.

**Exposes:**

- Pulumi rule catalogue.
- The pack architecture pattern itself — every other pack module references
  the ADR this module produces.

## Prerequisites

- `lang-ts-audit` complete (TS at audited T3).
- [ADR-027](../decisions/027-pack-architecture.md) reviewed and Accepted
  (it was authored against PACKPUL — review it on the way to Ready).

## Ready Checklist

Change status to **Ready** when:

- [ ] LANGTS complete.
- [ ] ADR-027 advanced from Proposed → Accepted; PACKPUL-001 task replaced
      by reference to the ADR.
- [ ] Owner named.
- [ ] Acceptance bar agreed (FP rate < N% on Anvil's `infra/` AND
      ≥ 1 external Pulumi program validation).

## Work Items

Anticipated:

- PACKPUL-001: Land the `crates/anvil-pack-pulumi/` crate skeleton +
  `crates/anvil-packs/` registry entry per
  [ADR-027](../decisions/027-pack-architecture.md). Activation guard via
  [OPSUP](../archive/modules/operational-supplement.aps.md) check registry.
- PACKPUL-002: Pulumi-program detection (`@pulumi/*` imports).
- PACKPUL-003: IAM / S3 ACL rule set.
- PACKPUL-004: Versioning / stack-crossing rule set.
- PACKPUL-005: Hardcoded-secrets rule set (coordinate with secret scanner —
  no duplication).
- PACKPUL-006: Validation runs (Anvil `infra/` + external Pulumi program).

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Pack architecture decision wrong; expensive to re-do across all packs (council §16.5 #4) | High | Treat ADR as gating; council-review the pack architecture decision before any rule-catalogue work |
| `@pulumi/*` detection false negatives (re-exports, conditional imports) | Medium | Conservative detection; document edge cases; revisit on demand |
| IAM rules too loose / too tight | Medium | Pair with a security-analyst review; allow per-rule opt-out |
| Pack ROI argument depends on this shipping cleanly (council C-007 chain) | High | Phase 1 acceptance bar is hard — no ship until validation passes |

## Open Questions

- [ ] [ADR-027](../decisions/027-pack-architecture.md) is `Proposed`;
      confirm it advances to `Accepted` before PACKPUL tasks start.
      The ADR records: per-pack crate `crates/anvil-pack-{name}/`,
      kernel symbol-graph access, compiled-in activation. Re-open this
      question only if council review of ADR-027 challenges any of those.
- [ ] How is pack activation enabled per project — auto-detect by
      `@pulumi/*` only, or also opt-in via config?
