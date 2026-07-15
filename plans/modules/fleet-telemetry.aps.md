<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Fleet Telemetry

| ID    | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| FLEET | —     | High     | In Progress |

**Last reviewed:** 2026-07-15 — OQ3 resolved by the operator (investor
evidence is the trigger); design gate
[ADR-107](../decisions/107-fleet-telemetry-consent-posture.md) **Accepted
2026-07-15 (operator)**; module flipped to Ready with FLEET-001..007
drafted against the accepted ADR.

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
telemetry" annotation. That reversal is decided:
[ADR-107](../decisions/107-fleet-telemetry-consent-posture.md) (Accepted
2026-07-15) narrows it to an enumerated payload behind a disclosed opt-out;
FLEET-006 owns rewriting the contract surfaces in the same release as the
beacon.

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

- [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md) — the
  module's design gate, **Accepted 2026-07-15**; its dimension allowlist
  and consent requirements bind every item below.
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

- **OQ1 (consent):** **Resolved 2026-07-15** via
  [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md)
  (Accepted) — disclosed opt-out (notice strictly before the first
  beacon; `anvil telemetry off` / `ANVIL_TELEMETRY=off` /
  `DO_NOT_TRACK=1` all hard offs).
- **OQ2 (identity):** **Resolved 2026-07-15** via
  [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md)
  (Accepted) —
  anonymous random per-install UUID, rotatable via `anvil telemetry
  reset-id`; the salted principal is deliberately NOT reused on the
  wire.
- **OQ3 (trigger):** **Resolved 2026-07-15 (operator):** fleet telemetry
  is needed as evidence for investors — the Draft→Ready flip does NOT
  wait on an EXPORT-style paying-customer/incident gate; it is gated only
  on ADR-107 acceptance and the Ready checklist below.

## Ready Checklist

All satisfied 2026-07-15 — module flipped to **Ready**:

- [x] [ADR-107](../decisions/107-fleet-telemetry-consent-posture.md)
      Accepted by the operator (design gate; Accepted 2026-07-15)
- [x] Dimension allowlist enumerated and reviewed against the privacy
      contract (ADR-107 Decision §3; amendments require a dated ADR note)
- [x] Ingest ownership confirmed (`apps/anvil-api` versioned route; raw
      90d / aggregates kept — ADR-107 Decision §6)
- [x] Work items drafted against the accepted ADR (below)

## Work Items

Sequencing: FLEET-001/-002 are independent leads; FLEET-003 needs both (and
prefers CIB-197's envelope fields); FLEET-004 needs -003's payload builder;
FLEET-005 can proceed in parallel on the API side; FLEET-006 ships in the
same release as -003 (the disclosure/docs and the beacon may not separate);
FLEET-007 follows once ingest has data.

### FLEET-001: Consent state and disclosure surface

- **Status:** In Progress
- **Intent:** No beacon can ever fire before the user has seen an honest
  disclosure, and turning telemetry off is one obvious action.
- **Expected Outcome:** A persisted telemetry consent state (on/off +
  notice-shown) in the user-scoped state dir; `anvil telemetry on|off`
  flips it; the first-run surface (activation/welcome) shows the ADR-107
  disclosure naming the exact dimensions and the off switch, and the
  notice strictly precedes any first send. `ANVIL_TELEMETRY=off` and
  `DO_NOT_TRACK=1` override everything; gated `ANVIL_HOME` and non-TTY
  first runs that never showed the notice do not beacon.
- **Validation:** Tests prove no send path is reachable while
  notice-shown is false or any hard off is set.
- **Dependencies:** ADR-107 (Accepted).
- **Confidence:** high
- **Scope note (recorded narrowing, 2026-07-16):** the disclosure shipped
  on the `welcome` closing surface only; the `anvil start` activation-path
  disclosure is deferred to FLEET-003 as a ship condition (the send gate is
  fail-closed on notice-shown, so the deferral is privacy-safe but leaves
  start-only installs unnoticed — FLEET-003 must close that or the fleet
  under-beacons).

### FLEET-002: Anonymous install identity

- **Status:** In Progress
- **Intent:** Unique-install and retention counts without touching user
  identity.
- **Expected Outcome:** A random UUID v4 `install_id` minted on first use,
  stored beside the per-deployment salt in the credentials dir (0600),
  derived from nothing; `anvil telemetry reset-id` rotates it. The salted
  principal never appears in any telemetry payload.
- **Validation:** Tests cover mint-once, rotation, and permissions;
  payload-shape test asserts no principal field.
- **Dependencies:** ADR-107 (Accepted).
- **Confidence:** high

### FLEET-003: Telemetry beacon producer

- **Status:** Ready
- **Intent:** Ship the ADR-107 allowlist payload — and nothing else — at
  most once per install per 24h, without ever getting in the user's way.
- **Expected Outcome:** A session-start beacon carrying exactly the
  ADR-107 Decision §3 dimensions (schema_version, install_id, version,
  install_method, platform triple, channel, flag snapshot version,
  feature-key usage counts since last beacon), sent asynchronously with a
  short timeout; failure is silent and unspooled; zero blocking latency
  added to any command (ADR-031 discipline). Reuses CIB-197's
  `version`/`install_method` fields rather than a second detector.
- **Validation:** Payload golden test pins the allowlist (a new field
  fails the test); latency assertion on the command path; 24h-cap and
  hard-off tests.
- **Dependencies:** FLEET-001, FLEET-002, CIB-197.
- **Confidence:** medium — the emission point in the session-start path
  needs care around the activation flow.
- **Coordination notes (from the 2026-07-16 FLEET-001/002/005 verification):**
  (a) ship the `anvil start` activation-path disclosure with the beacon
  (see FLEET-001 scope note); (b) the FLEET-005 ingest schema requires
  non-empty `channel` and `flag_snapshot_version` tokens — define concrete
  client values (e.g. `none` / `0`) or beacons will 400; (c) the
  transparency surface (FLEET-004) should list `schema_version` alongside
  the dimensions so the "exactly what is sent" promise is literal.

### FLEET-004: `anvil telemetry` transparency command

- **Status:** Ready
- **Intent:** The allowlist is auditable from the binary itself, not just
  the docs.
- **Expected Outcome:** `anvil telemetry` prints the consent state, the
  install id, and the exact payload the next beacon would send (or that
  none will, and why); `on|off|reset-id` subverbs round out the surface.
- **Validation:** Snapshot test of the rendered payload matches the
  FLEET-003 golden.
- **Dependencies:** FLEET-003 (payload builder).
- **Confidence:** high

### FLEET-005: anvil-api ingest route

- **Status:** In Progress
- **Intent:** A place for beacons to land that honours the retention and
  privacy commitments.
- **Expected Outcome:** A versioned `apps/anvil-api` ingest route
  validating the schema_version'd payload; IPs not retained on the stored
  row; raw rows retained 90 days, aggregates kept; operator-only access.
- **Validation:** Route tests cover schema rejection, IP absence in
  storage, and retention configuration.
- **Dependencies:** ADR-107 (Accepted); payload schema from FLEET-003
  (coordinate on the shape early, don't block).
- **Confidence:** medium — retention mechanics depend on the Neon/storage
  setup.

### FLEET-006: Privacy contract and docs update

- **Status:** Ready
- **Intent:** Every surface that says "nothing leaves the machine" is
  rewritten honestly in the same release that ships the beacon.
- **Expected Outcome:** `docs/observability/usage-analytics.md` (and any
  welcome/docs copy repeating the local-only promise) distinguishes the
  local Kindling pipe (unchanged) from the ADR-107 beacon (dimensions
  enumerated, offs documented); a public docs page lists the allowlist.
- **Validation:** `rg -n "never leaves|local-only" docs/` finds no stale
  absolute claims; docs:check passes.
- **Dependencies:** FLEET-003 (must ship together).
- **Confidence:** high

### FLEET-007: Operator fleet view

- **Status:** Ready
- **Intent:** The investor-evidence read surface — the reason this module
  exists.
- **Expected Outcome:** An operator-only view over the ingested aggregates
  answering: active installs (daily/weekly/monthly), version distribution,
  install-method mix, feature adoption, and retention cohorts.
- **Validation:** View renders correct aggregates against a seeded
  fixture set.
- **Dependencies:** FLEET-005 (ingested data).
- **Confidence:** medium — surface choice (api route + dashboard vs
  ad-hoc queries) can be decided at execution time.
