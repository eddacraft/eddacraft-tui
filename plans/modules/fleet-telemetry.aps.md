<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Fleet Telemetry

| ID    | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| FLEET | —     | Medium   | Draft  |

**Last reviewed:** 2026-07-14

> **Provenance:** Filed 2026-07-14 from an operator observability review.
> Today Anvil ships **zero** remote telemetry by deliberate privacy posture:
> the usage pipe (USAGE/KDS) is local-only Kindling, tracing is opt-in local
> sinks, and EXPORT covers only a production sink for the *tracing* pipe.
> The operator has no way to see what version anyone runs, how they installed
> it, or which features are used in the field. This module owns the decision
> and the machinery to change that — deliberately, with explicit user consent, and within
> a tightly controlled dimension set.

## Purpose

Give the operator fleet-level visibility — what version is out there, which
install method delivered it, and which features are actually used — via an
explicitly consented, phone-home telemetry channel.

This is a **posture change**, not an increment: the existing privacy contract
(`docs/observability/usage-analytics.md`) promises observations never leave
the machine, and the INSIGHTS module was shipped with an explicit "No
telemetry" annotation. Nothing in this module is buildable until a consent
ADR reverses that promise for a narrow, enumerated payload.

The dimension contract already exists on paper: the feature-flagging design
(`plans/specs/2026-04-09-feature-flagging-design.md`, restated in
`docs/guides/feature-flag-governance.md`) planned session-start snapshot
telemetry plus one usage stat per feature actually used — no PII, only
low-risk dimensions (feature key, environment, runtime, snapshot version,
coarse tier/channel). That contract was realised locally as the USAGE-002
inline `flag_set`; its remote half was never wired and lands here.

## In Scope

- The consent-posture ADR: opt-in vs opt-out, first-run disclosure surface,
  `DO_NOT_TRACK` / `ANVIL_USAGE_DISABLE` interaction, and what "tightly
  controlled" means as an enumerated allowlist of dimensions.
- The beacon payload: binary version, install method (LAUNCH-013 detection),
  platform triple, and the FLAGS-design feature-usage dimensions — nothing
  free-form, nothing path- or identity-shaped beyond the existing salted
  principal convention (or less).
- The ingest surface (likely an `apps/anvil-api` route) and its retention
  and access story.
- The client emission path: where the beacon fires (session start / daily
  cap), its failure mode (never block or slow a command), and its kill
  switches.
- Privacy-contract and docs updates that make the collected set auditable
  by users (`anvil <something>` should be able to show exactly what would
  be sent).

## Out of Scope

- Local observation collection — USAGE/KDS own the Kindling pipe; CIB-197
  enriches the local envelope with version/install-method independently of
  this module.
- Tracing export — EXPORT owns the tracing-pipe production sink (OQ1).
- Crash reporting or diagnostic dumps — different risk class, separate
  decision if ever wanted.
- Any payload dimension outside the enumerated allowlist.

## Interfaces

**Depends on:**

- A consent-posture ADR (the module's design gate; nothing ships before it
  is Accepted).
- LAUNCH-013 `InstallMethod` detection
  (`crates/anvil-cli/src/commands/version.rs`).
- CIB-197 (local envelope enrichment) — the beacon should reuse the same
  version/install-method fields, not invent parallel ones.
- `apps/anvil-api` — the plausible ingest host.

**Coordinates with:**

- [usage-analytics](../archive/modules/usage-analytics.aps.md) (USAGE,
  archived) — privacy contract, salted-principal convention, opt-out env
  vars.
- [observability-export](./observability-export.aps.md) (EXPORT) — separate
  pipe, but the consent ADR should speak to both so users get one coherent
  telemetry story.
- FLAGS design spec + `docs/guides/feature-flag-governance.md` — the
  feature-usage dimension contract this module inherits.
- Licence-gate / entitlement surface — the beacon and the licence check
  must not become a covert second telemetry channel; whatever the ADR
  decides applies to both.

**Exposes:**

- The consent ADR, the dimension allowlist, the ingest endpoint, and an
  operator-facing view of fleet version/install/feature distribution.

## Open Questions

- **OQ1 (consent):** opt-in (weaker data, cleaner posture) vs
  opt-out-with-first-run-disclosure (better coverage, needs a careful
  first-run UX)? `DO_NOT_TRACK` is already honoured for local collection
  and must remain a hard off.
- **OQ2 (identity):** is the salted per-deployment principal hash reused,
  or does fleet telemetry deliberately drop identity entirely and count
  anonymously?
- **OQ3 (trigger):** does the Draft→Ready flip wait on the same trigger as
  EXPORT (first paying customer / production incident), or does beta
  distribution itself justify it earlier?

## Ready Checklist

Change status to **Ready** when:

- [ ] Consent-posture ADR drafted and Accepted (design gate)
- [ ] Dimension allowlist enumerated and reviewed against the privacy
      contract
- [ ] Ingest ownership confirmed (`apps/anvil-api` route + retention)
- [ ] Work items drafted against the accepted ADR

## Work Items

None yet — per APS rules, work items are drafted when the module goes Ready
(after the consent ADR is Accepted).
