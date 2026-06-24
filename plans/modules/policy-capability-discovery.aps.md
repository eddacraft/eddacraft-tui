# Policy Capability Discovery (POLCAP)

| ID     | Owner | Status   |
| ------ | ----- | -------- |
| POLCAP | —     | Proposed |

**Created:** 2026-05-24
**Promotion gate:** Planning Council required before any task is marked
  Ready. Cross-boundary surface (policy + intercept daemon + witness chain
  + agent runtime + driver-client) — design needs multi-persona review.

> **Policy-solution validation (2026-06-24):** corrected the ADR placeholder
> from ADR-051 to ADR-092 because ADR-051 is already the accepted
> CLI-panic-unwind decision. POLCAP composes with the ADR-040 regorus policy
> runtime and ACTAX/AGOV/IORISK signal producers; it must not create a parallel
> policy evaluator. The capability view is advisory until gate/witness bindings
> make `cap_id` audit rows load-bearing.

## Purpose

Give a governed agent a deterministic, signed, machine-readable view of the
action families it may attempt in the current session — before it commits
to a tool call. The view is advisory; `gate` remains the sole enforcement
authority. Every `cap_id` returned by the view is load-bearing for
subsequent gate decisions and witness-chain audit rows.

This module owns the new `anvil policy capabilities` CLI subcommand, the
`capabilities/describe` daemon IPC method, the signed-envelope shape, the
skill-recipe vocabulary, the structured error-code taxonomy, the
fail-closed semantics for unknown / stale cap-IDs, and the witness-chain
audit binding. Detailed design lives in
[`plans/specs/2026-05-24-policy-capability-discovery.md`](../specs/2026-05-24-policy-capability-discovery.md).

## Boundaries

**In scope**

- `anvil policy capabilities` CLI subcommand (advisory, planless-first
  compatible).
- Daemon IPC method `capabilities/describe` with embedded
  correctness-equivalent fallback.
- Signed view envelope (HMAC-SHA256, session-keyed); wire-shape carries
  `alg` so Ed25519 is an additive v2 upgrade.
- Skill recipes for the beta narrow set: `file.write`, `shell.run`,
  `repo.change`, `network.request`, `secret.read`, plus selected MCP
  tools.
- Structured error-code taxonomy (`stale_epoch`, `unknown_cap_id`, …) in
  `crates/anvil-kernel-types`.
- Witness-chain row extension: additive optional `cap_id` +
  `cap_id_status` fields under `serde(default, skip_serializing_if)`.
- "Agent never does" negative-space contract landed in `AGENTS.md`.
- Reconciliation with AGOV-007 (agent-declared capability manifest) — two
  surfaces, distinct ownership, joined in policy evaluation.

**Out of scope**

- Credential brokering or upstream proxying (the Warden surface POLCAP is
  *not* copying).
- Asymmetric signing of capability views (reserved for v2).
- Provider catalogue beyond the named beta families (ACTAX governance
  owns family expansion).
- Capability views for non-governed sessions (no daemon → no view).
- Cross-host federation (single host, single daemon).
- Web-dashboard rendering (post-launch DASHOPS).

## Dependencies and coordination

- **Coordinates with:** AGOV-007 (capability-manifest schema reconciliation),
  ACTAX-001 (action taxonomy `domain.verb` vocabulary), IORISK (risk
  dimensions as evidence requirements), POLENG-001 (engine facade as the
  view's source of truth), SKOBS (skill-inventory hashing pattern reused
  for recipes), DRVR (per-driver manifest negotiation already exists —
  POLCAP is the agent-side surface; DRVR is the driver-side surface).
- **Blocks on:** Planning Council acceptance of this module + ADR-092.
- **Cites:** ADR-001 (planless-first), ADR-002 (warnings over blocks),
  ADR-037 (witness chain + L4 policy), ADR-040 (regorus engine), ADR-024
  (weave agent harness), ADR-045 (minisign — signing-scheme prior art for
  the eventual v2 asymmetric upgrade).

## Release metadata

- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** Planning Council accepts ADR-092; AGOV-007 manifest
  shape stable; ACTAX-001 action taxonomy vocabulary frozen for v1.
- **releaseScope:** minor (new agent-facing protocol surface, additive
  witness-chain fields)
- **releaseNote.audience:** developer
- **releaseNote.type:** added
- **releaseNote.text:** `anvil policy capabilities` returns a signed,
  scoped view of allowed action families for the current agent session;
  gate and witness chain bind each decision to its cap_id.

## Validation

- `cargo test -p eddacraft-anvil-policy -p eddacraft-anvil-kernel-types`
  covers view shape, envelope round-trip, fail-closed on unknown cap_id,
  fail-closed on stale epoch.
- Cross-language parity test (Rust ↔ TS driver-client) for the signed-view
  JSON shape against a captured fixture.
- End-to-end test through the daemon: allowed call (records cap_id on
  witness row), denied call (gate refuses with structured code), unknown
  cap_id (gate refuses with code 10, witness row records
  `cap_id_status: "unknown"`).
- `pnpm docs:check && pnpm docs:index:check` green after recipe files
  land.

## Work Items

| ID | Title | Status | Confidence |
|---|---|---|---|
| POLCAP-001 | Author ADR-092 and convene Planning Council | Proposed | medium |
| POLCAP-002 | Capability-view JSON schema + signing envelope (kernel-types) | Proposed | medium |
| POLCAP-003 | Skill-recipe vocabulary and the seven beta recipes | Proposed | medium |
| POLCAP-004 | Structured error-code taxonomy + agent recovery contract | Proposed | high |
| POLCAP-005 | `anvil policy capabilities` CLI subcommand | Proposed | medium |
| POLCAP-006 | Daemon IPC method `capabilities/describe` with embedded fallback | Proposed | medium |
| POLCAP-007 | Witness-chain row extension: optional `cap_id` + `cap_id_status` | Proposed | medium |
| POLCAP-008 | Fail-closed semantics: unknown cap_id, stale signing_epoch | Proposed | high |
| POLCAP-009 | Reconciliation with AGOV-007, ACTAX-001, IORISK risk dimensions | Proposed | low |
| POLCAP-010 | Operator docs: recipe authoring guide + epoch-rotation runbook | Proposed | high |

### POLCAP-001 — Author ADR-092 and convene Planning Council

- **Intent:** Land the durable architectural decision for the
  capability-discovery surface before any implementation begins.
- **Expected Outcome:** ADR-092 file at
  `plans/decisions/092-policy-capability-discovery.md` with status
  Proposed; entry added to
  `plans/decisions/DECISION-LOG.md` under "Policy and Governance"; Planning
  Council session run with kernel, ops, adversarial, pragmatic, and
  policy-engine personas; council verdict recorded inline in the ADR.
- **Validation:** `pnpm adr:check` green; council session minutes linked
  from the ADR.
- **Files:** `plans/decisions/092-policy-capability-discovery.md`,
  `plans/decisions/DECISION-LOG.md`, this module file.
- **Confidence:** medium — depends on council outcome.
- **changeType:** docs
- **releaseIntent:** never

### POLCAP-002 — Capability-view JSON schema + signing envelope

- **Intent:** Pin the on-wire shape of `SignedCapabilityView` and its
  HMAC-SHA256 envelope as a typed kernel artefact so every consumer
  (daemon, CLI, MCP shim, TS driver-client) reads one source of truth.
- **Expected Outcome:** `crates/anvil-kernel-types::policy::capabilities`
  module exposing `SignedCapabilityView`, `CapabilityRow`, `RoleRow`,
  `EscalationRow`, `Envelope`, with `serde(deny_unknown_fields = false)`
  on the optional-additive fields per MLP2-052; matching `schemas/anvil-policy-capabilities.v1.json`;
  one byte-equal round-trip test.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types`; new schema
  picked up by `pnpm test:schema-contracts`.
- **Dependencies:** POLCAP-001.
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** ADR-092 Accepted.

### POLCAP-003 — Skill-recipe vocabulary and the seven beta recipes

- **Intent:** Author the markdown recipes the agent reads to learn how to
  invoke each action family.
- **Expected Outcome:** `docs/agent/skills/` directory with one recipe per
  beta family (`file.write.v1.md`, `shell.run.v1.md`, `repo.change.v1.md`,
  `network.request.v1.md`, `secret.read.v1.md`, plus `mcp.discovery.v1.md`
  and `troubleshooting.v1.md`); recipe front-matter pins the `version`
  integer; `docs/indexes/` rebuilt.
- **Validation:** `pnpm docs:check && pnpm docs:index:check`; recipe
  fixture test that the daemon registry can load every shipped recipe.
- **Dependencies:** POLCAP-002 (envelope must exist before recipes can
  reference recipe versions on the wire).
- **changeType:** docs
- **releaseIntent:** hold
- **holdCondition:** POLCAP-002 Done.

### POLCAP-004 — Structured error-code taxonomy

- **Intent:** Pin the typed error codes the agent branches on so recovery
  is deterministic.
- **Expected Outcome:** `crates/anvil-kernel-types::policy::error::CapErrorCode`
  enum covering the 10 codes in spec §9; constants exported to TS
  driver-client; matching documentation lane in the recipes
  (`troubleshooting.v1.md`).
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types`;
  cross-language parity test pinning each variant's wire string.
- **Dependencies:** POLCAP-002.
- **changeType:** feature
- **releaseIntent:** hold

### POLCAP-005 — `anvil policy capabilities` CLI subcommand

- **Intent:** Ship the operator-facing diagnosis surface.
- **Expected Outcome:** `anvil policy capabilities [--scope] [--format
  json|yaml] [--raw]` returns a signed view from the daemon when
  available, falls back to a correctness-equivalent embedded computation
  when not; exit codes map to the §9 taxonomy.
- **Validation:** `cargo test -p eddacraft-anvil-cli`; integration test
  exercising both daemon-backed and embedded paths.
- **Dependencies:** POLCAP-002, POLCAP-004, POLCAP-006.
- **changeType:** feature
- **releaseIntent:** hold

### POLCAP-006 — Daemon IPC method `capabilities/describe`

- **Intent:** Wire the authoritative surface in the intercept daemon.
- **Expected Outcome:** New `capabilities/describe` JSON-RPC method in
  `crates/anvil-intercept`; signs the response with the per-session HMAC
  key derived during owner-only IPC handshake; rate-limited by the
  existing session registry primitives; emits `tracing::` instrumentation
  per ADR-035.
- **Validation:** `cargo test -p eddacraft-anvil-intercept`; latency under the
  500 ms activation budget pinned by INTD-014's rubric.
- **Dependencies:** POLCAP-002, POLCAP-004; coordinates with MLP2-051f
  (activation diagnostic) for the signing-key derivation path.
- **changeType:** feature
- **releaseIntent:** hold

### POLCAP-007 — Witness-chain row extension

- **Intent:** Bind the advisory view to the authoritative record.
- **Expected Outcome:** `WitnessLine` gains optional `cap_id: Option<String>`
  and `cap_id_status: Option<CapIdStatus>` fields under
  `serde(default, skip_serializing_if = "Option::is_none")`; daemon writes
  them on gate decisions; manifest event stream
  (`anvil/witness/manifest/chain.ndjson`) carries the same fields.
- **Validation:** `cargo test -p eddacraft-anvil-witness`; round-trip
  parity test pins that pre-POLCAP-007 lines still deserialise (MLP2-052
  additive-optional contract); `cargo test -p eddacraft-anvil-l4`
  exercises the unknown-cap-id refusal path.
- **Dependencies:** POLCAP-002, POLCAP-006.
- **changeType:** feature
- **releaseIntent:** hold

### POLCAP-008 — Fail-closed semantics

- **Intent:** Make unknown / stale cap-IDs refuse, never silently
  downgrade.
- **Expected Outcome:** Gate refuses any call carrying a `cap_id` unknown
  to the daemon's registry with `CapErrorCode::UnknownCapId`; refuses any
  call referencing a `signing_epoch` below the daemon's current epoch
  with `CapErrorCode::StaleEpoch`; both refusals record on the witness
  row.
- **Validation:** `cargo test -p eddacraft-anvil-intercept` covering both
  refusal paths; adversarial test injects a cap_id from a different
  session and confirms refusal.
- **Dependencies:** POLCAP-006, POLCAP-007.
- **changeType:** feature
- **releaseIntent:** hold

### POLCAP-009 — Reconciliation with AGOV-007 / ACTAX-001 / IORISK

- **Intent:** Make sure POLCAP, AGOV's capability-manifest, ACTAX's
  taxonomy, and IORISK's risk dimensions form one coherent surface, not
  three overlapping vocabularies.
- **Expected Outcome:** ACTAX `ActionId` is the *only* family-name source
  for POLCAP; AGOV-007 manifest validator gains a check that every
  declared action references an `ActionId` known to ACTAX; IORISK risk
  rows referenced via `evidence` field in the capability row carry a
  typed `RiskRef`; documentation cross-link sweep.
- **Validation:** `cargo test -p eddacraft-anvil-policy`; ACTAX-001's
  taxonomy test extended to assert no POLCAP recipe references an
  unknown action-id; AGOV-007 validator test extended to assert
  cross-reference.
- **Dependencies:** POLCAP-003; coordinates with ACTAX-001, AGOV-007,
  IORISK-001.
- **changeType:** internal
- **releaseIntent:** hold

### POLCAP-010 — Operator docs

- **Intent:** Make the recipe-authoring surface usable by operators
  without reading the implementation.
- **Expected Outcome:** `docs/guides/policy-capability-recipe-authoring.md`
  (recipe schema, version-bump rules, signing-epoch impact);
  `docs/runbooks/anvil-capability-epoch-rotation.md` (operational
  procedure for signing-epoch rotation); both indexed.
- **Validation:** `pnpm docs:check && pnpm docs:index:check`;
  documentation closeout per DOCGOV-001.
- **Dependencies:** POLCAP-006, POLCAP-008.
- **changeType:** docs
- **releaseIntent:** hold

## Risks

- **R-1 (high):** Surface inflation — without strict ACTAX-gated family
  expansion, POLCAP drifts toward Warden's 33-provider zoo. **Mitigation:**
  POLCAP-009 makes ACTAX the only family source; new families require an
  ACTAX work item, not a recipe drop.
- **R-2 (medium):** Capability-view trust regression — agents start
  treating the view as authoritative and skip refusal handling.
  **Mitigation:** Spec §12 forbids it; cross-language parity test
  asserts refusal handling in the TS driver-client; gate refuses unknown
  IDs regardless.
- **R-3 (medium):** Description prompt-injection — operator-authored
  description strings get rendered into agent context and become an
  attack vector. **Mitigation:** Treat descriptions as untrusted by
  downstream LLM steps; redact in tracing per ADR-035 + TRACE-003.
- **R-4 (medium):** Epoch-rotation thrash — long-running agents
  constantly refresh against rotating epochs. **Mitigation:** Debounce
  bumps; refresh-on-stale, not refresh-on-schedule (POLCAP-008).
- **R-5 (low):** Cap-IDs leak into long-lived caches and become reusable
  tokens. **Mitigation:** Cap-IDs are scope-bound and signing-epoch-bound;
  leaked IDs are useless outside their scope and stale once the epoch
  rotates.

## Provenance

- **Spec:** `plans/specs/2026-05-24-policy-capability-discovery.md`
- **Brainstorm:** `plans/brainstorms/agent-security-package.md`
- **External analysis:** `stephnangue/warden` repository (MPL-2.0); no
  Warden source vendored, patterns reused under clean-room
  reimplementation. The Warden gateway architecture (seal/unseal,
  credential proxying, provider zoo) is explicitly **not** in POLCAP
  scope.
