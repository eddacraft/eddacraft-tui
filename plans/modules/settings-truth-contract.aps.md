<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Settings Truth Contract

| ID     | Owner | Priority | Status   | Progress |
| ------ | ----- | -------- | -------- | -------- |
| SETCON | —     | medium   | Proposed | 0/11     |

**Last reviewed:** 2026-08-13 — created 2026-08-06 from the operator-supplied
`/settings` specification v1.1
([`plans/specs/2026-08-06-settings-truth-surface.md`](../specs/2026-08-06-settings-truth-surface.md),
spec §22 Slice 0). No ADR accepted yet; no release window claimed. Not competing
with the active release window (named in the index header).

> **Activation gate.** SETCON promotes to **Ready** when (a) SETCON-001 is
> accepted as an ADR fixing the terminology, runtime-state model and service
> boundary, and (b) the programme is scheduled against a named release window.
> Until both hold, downstream SETINS / SETPREF / SETGOV stay gated: they consume
> contracts this module has not yet fixed.

## Purpose

Fix the vocabulary, data contracts and service boundary that let Anvil
distinguish **configured**, **requested**, **resolved** and **active** settings
state — and prove which is which — before any surface renders a row or any
interface writes a value.

This is the foundation slice. It ships no user-visible `/settings` screen. Its
product value is that every later surface (TUI, CLI, MCP, diagnostics, tests)
derives from one catalogue, one resolver, one constraint engine, one redaction
rule set and one read model, so no surface can invent a second, softer truth.

The governing invariant this module encodes (spec §1):

> Anvil may infer desired configuration from its resolver, but it may describe a
> control as active only when accepted, current evidence from the responsible
> runtime component proves it.

## In Scope

- Canonical terminology and the mutually exclusive runtime-state model
  (`active` / `drift` / `stale` / `failed` / `unknown`), kept distinct from
  configuration and workflow states (`invalid`, `locked`, `pending activation`)
- Typed settings catalogue: keys, types, scopes, defaults, merge semantics,
  consequence class, sensitivity classification, health relevance, activation
  owner, deprecated aliases, extension namespacing and collision failure
- Precedence and composite-value resolution with field/member-level provenance,
  including deletions and exclusions as first-class resolution events
- Policy constraints as a post-resolution constraint layer (not a precedence
  source), including signed-bundle trust and fail-closed behaviour
- Runtime attestation contract: evidence shape, trust classification, acceptance
  rules, and the deterministic runtime-state classifier
- Health aggregation over health-relevant and policy-mandated controls only
- Redaction invariants: recursive, fail-closed, applied to every output channel
- Versioned JSON envelope `anvil.settings.v1` and the settings semantic outcomes
  mapped onto a **global** Anvil CLI exit-code registry
- Authoritative settings-service boundary and consistent read-model snapshots

## Out of Scope

- Any `/settings` TUI view, CLI subcommand or MCP tool (SETINS)
- Any configuration write path (SETPREF for Class A, SETGOV for Class B/C)
- Audit storage and material-event recording (SETGOV)
- Natural-language intent handling (SETNL)
- Replacing `.anvilrc` / config file formats — SETCON reads what
  [UCFG](./unified-config-format.aps.md) defines; it does not redesign it
- Building a new policy engine — constraints evaluate over the existing policy
  surfaces ([POLLC](./policy-lifecycle.aps.md),
  [ORGHIER](./org-policy-hierarchy.aps.md)); SETCON defines how a constraint
  reaches a settings row, not how policy is authored

## Interfaces

**Depends on:**

- `crates/anvil-config` — existing config load/resolve surface; the catalogue
  and resolver are expected to live here or in a sibling crate (the SETCON-001
  ADR decides)
- `crates/anvil-policy-engine` — policy evaluation used by the constraint layer
- `crates/anvil-intercept`, `crates/anvil-kernel` — activation owners that must
  expose attestation for enforcement-critical controls
- `crates/anvil-cli` — existing `anvil config show` / `validate` compatibility
  surface and the global exit-code contract

**Exposes:**

- Typed catalogue registration API for core components, adapters and extensions
- Read-model snapshot (`revision`-stamped) consumed by TUI, CLI, MCP and tests
- `anvil.settings.v1` JSON envelope and canonical key space
- Runtime-state classifier and health aggregator

**Coordinates with:**

- [unified-config-format](./unified-config-format.aps.md) (UCFG) — source
  discovery and file layout
- [org-policy-hierarchy](./org-policy-hierarchy.aps.md) (ORGHIER) — org →
  team → project resolution feeding the constraint layer
- [feature-flag-catalogue](./feature-flag-catalogue.aps.md) (FLAGCAT) — flags are
  catalogue entries, not a parallel registry
- [cli-command-truth](./cli-command-truth.aps.md) (CLICT) — `anvil settings`
  gets a command-truth slice before docs claim it exists

## Constraints

- **Deterministic** — identical sources, constraints and evidence yield an
  identical read model and identical JSON bytes (modulo `generated_at`).
- **Fail closed on redaction** — a redaction failure emits no settings payload,
  on any channel, ever. This outranks usefulness.
- **No second writer** — nothing in this module writes configuration.
- **Additive schema** — `anvil.settings.v1` may gain optional fields; removing,
  reinterpreting or changing a closed-enum value requires a new schema version.
- **Evidence, not inference** — absence of attestation is `unknown`, never
  `active`. No code path may convert a non-`active` state into a reassuring one.

## Acceptance Criteria

- [ ] TUI, CLI and MCP consumers can only reach settings state through the
      shared catalogue, resolver, constraint engine and redaction rules
- [ ] An organisation constraint cannot be bypassed by a higher-precedence
      repository, environment or session declaration
- [ ] Scalar and composite resolution preserve complete provenance
- [ ] Runtime-state classification is deterministic and total: exactly one of
      `active` / `drift` / `stale` / `failed` / `unknown` per evidence-bearing key
- [ ] Health aggregation considers only health-relevant or policy-mandated keys
- [ ] Evidence from an unregistered owner, untrusted channel or incompatible
      component is rejected and cannot produce `active`
- [ ] Golden tests pin the JSON envelope and the documented global exit-code map

## Ready Checklist

Change status to **Ready** when:

- [ ] SETCON-001 ADR accepted (terminology, runtime-state model, service boundary)
- [ ] Catalogue home crate decided (extend `anvil-config` vs new crate)
- [ ] Attestation transport decided (reuse daemon RPC vs new channel)
- [ ] Programme scheduled against a named release window

## Work Items

### SETCON-001: Truth-contract ADR

- **Intent:** Fix, as an accepted decision, the settings terminology, the
  mutually exclusive runtime-state model, and the authoritative settings-service
  boundary so later slices cannot re-litigate them.
- **Expected Outcome:** An ADR in `plans/decisions/` is Accepted and recorded in
  `DECISION-LOG.md`, defining: configured / requested / resolved / active;
  `active`/`drift`/`stale`/`failed`/`unknown` as mutually exclusive at a point in
  time and separate from `invalid`/`locked`/`pending activation`; the ban on
  `effective` as an ambiguous term; the rule that no interface writes config
  outside the settings service; and the catalogue home crate.
- **Scope:** ADR text, decision-log entry, spec cross-reference
- **Non-scope:** Any implementation
- **Dependencies:** —
- **Validation:** `pnpm run lint:md`; ADR listed in
  `plans/decisions/DECISION-LOG.md`
- **Confidence:** high
- **Status:** Proposed

### SETCON-002: Typed settings catalogue

- **Intent:** Give every inspectable setting a typed catalogue entry so surfaces
  render from data instead of hard-coded rows.
- **Expected Outcome:** A catalogue type carrying the spec §11 fields (key,
  label, owner, group/order, type and allowed values, default, supported scopes,
  precedence, merge semantics, mutability and canonical writer, consequence
  class, sensitivity, validation, runtime-evidence mode, health relevance,
  activation owner, docs reference, deprecated aliases, version compatibility);
  adapter and extension keys are namespaced; a key collision fails catalogue
  validation rather than picking a winner.
- **Non-scope:** Populating every existing Anvil setting (SETCON-011 seeds the
  first-release groups); rendering
- **Dependencies:** SETCON-001
- **Validation:** `cargo test -p anvil-config catalogue`
- **Confidence:** medium
- **Status:** Proposed

### SETCON-003: Sensitivity classification and redaction invariants

- **Intent:** Make it impossible to emit a secret or unclassified value from any
  settings channel.
- **Expected Outcome:** Every catalogue entry carries a sensitivity
  classification; recursive redaction is applied to rendering, JSON, MCP, logs,
  diagnostics and telemetry from one implementation; values lacking a trusted
  classification are hidden by default; a redaction failure aborts the payload
  entirely rather than degrading it; Class D keys expose only presence and
  explicitly-safe metadata.
- **Non-scope:** Secret storage or rotation; the secure update path itself
- **Dependencies:** SETCON-002
- **Validation:** `cargo test -p anvil-config redaction`
- **Confidence:** medium
- **Status:** Proposed

### SETCON-004: Precedence and composite resolution with provenance

- **Intent:** Resolve declarations across scopes while retaining where every
  part of the answer came from.
- **Expected Outcome:** The resolver produces a resolved value plus complete
  provenance; each structured setting resolves by its own declared behaviour
  (replace, append, union, keyed merge) with no implicit global collection rule;
  composite values retain field/member-level provenance; overridden declarations
  and deletions/exclusions remain visible as resolution events.
- **Non-scope:** Policy constraints (SETCON-005); source file parsing (UCFG)
- **Dependencies:** SETCON-002
- **Validation:** `cargo test -p anvil-config resolver`
- **Confidence:** medium
- **Status:** Proposed

### SETCON-005: Policy constraint layer

- **Intent:** Represent organisation policy as constraints on the permitted
  value space rather than as another last-writer-wins source.
- **Expected Outcome:** Constraints evaluate after ordinary resolution and can
  require or prohibit values, set minimum/maximum postures, mandate or forbid
  collection members, restrict override scopes and demand approval authorities; a
  local, environment or session declaration cannot escape a controlling
  constraint by having higher precedence; an unverifiable, expired or
  incompatible policy bundle is never silently ignored, and the bundle cannot
  select its own failure behaviour.
- **Non-scope:** Policy authoring, distribution or signing-key management
- **Dependencies:** SETCON-004
- **Validation:** `cargo test -p anvil-config constraints`
- **Confidence:** medium
- **Status:** Proposed

### SETCON-006: Runtime attestation and evidence trust

- **Intent:** Let responsible components prove what they are actually enforcing,
  and classify that evidence deterministically.
- **Expected Outcome:** An activation-status contract carrying component and
  instance identity, evidence channel and trust class, applicable keys, active
  value or classified digest or conformance result, applied configuration
  revision, timestamps, validity interval, restart requirement and failure
  status; the service accepts evidence only from the registered activation owner
  over a channel meeting the catalogue's trust requirement; the classifier
  assigns exactly one state in the spec §12 order; process exit, disconnection,
  revision change or expiry makes evidence stale immediately.
- **Non-scope:** Instrumenting each component (tracked per owning module);
  hardware-backed attestation
- **Dependencies:** SETCON-001
- **Validation:** `cargo test -p anvil-config runtime_state`
- **Confidence:** low
- **Status:** Proposed

### SETCON-007: Health aggregation

- **Intent:** Derive one honest overall health verdict from required controls
  only.
- **Expected Outcome:** `healthy` when every required control is valid and
  `active`; `unhealthy` when a required control is invalid, `failed` or in
  `drift`, or a mandatory policy bundle is invalid; `indeterminate` when a
  required control is `unknown` or `stale` or evidence collection could not
  establish current truth; advisory findings stay visible without failing
  health; an unavailable optional integration does not fail health merely by
  being inspectable.
- **Non-scope:** `--check` exit wiring (SETCON-009); presentation
- **Dependencies:** SETCON-006
- **Validation:** `cargo test -p anvil-config health`
- **Confidence:** high
- **Status:** Proposed

### SETCON-008: Versioned JSON envelope

- **Intent:** Give automation a stable, redacted machine contract.
- **Expected Outcome:** A single JSON document with the spec §7 envelope
  (`schema_version`, `command`, `generated_at`, `model_revision`, `context`,
  `health`, `data`, `diagnostics`), no decoration on stdout, canonical keys as
  API, display labels excluded, additive-only evolution within
  `anvil.settings.v1`, and recursive redaction applied to the whole document.
- **Non-scope:** The CLI commands that emit it (SETINS)
- **Dependencies:** SETCON-003, SETCON-007
- **Validation:** `cargo test -p anvil-config envelope`
- **Confidence:** high
- **Status:** Proposed

### SETCON-009: Global CLI exit-code registry

- **Intent:** Map the settings semantic outcomes onto Anvil's global exit-code
  contract instead of inventing a command-local numbering.
- **Expected Outcome:** A documented global registry exists (established here if
  absent); `success`, `check_failed`, `usage_error`, `resolution_error`,
  `access_error`, `redaction_error` and `internal_error` each map to a stable
  numeric code; the mapping is golden-tested; existing command exit behaviour is
  recorded rather than silently renumbered.
- **Non-scope:** Changing any existing command's observed exit code
- **Dependencies:** SETCON-001
- **Validation:** `cargo test -p anvil-cli exit_codes`; `pnpm run docs:check`
- **Confidence:** medium
- **Status:** Proposed

### SETCON-010: Settings service and read-model boundary

- **Intent:** Establish the one component every surface calls, and the
  revisioned snapshot it hands back.
- **Expected Outcome:** A settings service owns catalogue loading, source
  discovery and revision capture, resolution, constraint evaluation, evidence
  acceptance, snapshot generation and health aggregation; a snapshot is
  internally consistent and carries `model_revision`; inspection can return a
  valid model that describes invalid or unhealthy state; no consumer reads
  configuration files directly for settings purposes.
- **Non-scope:** Mutation, approval, persistence and audit responsibilities
  (SETPREF / SETGOV extend the same service later)
- **Dependencies:** SETCON-004, SETCON-005, SETCON-006, SETCON-007
- **Validation:** `cargo test -p anvil-config service`
- **Confidence:** medium
- **Status:** Proposed

### SETCON-011: Seed catalogue for first-release groups

- **Intent:** Populate enough real catalogue entries that the inspect surface has
  honest content on day one.
- **Expected Outcome:** Catalogue entries exist for the spec §8 groups —
  Protection, Agents & approvals, Privacy & egress, Integrations, Interface —
  each with correct classification, health relevance and activation owner;
  enforcement-critical entries marked health-relevant declare an attestation
  owner; keys with no compatible attestation are declared evidence-mode `none`
  and can never read as `active`.
- **Non-scope:** Adding new configurable behaviour to Anvil; exhaustively
  cataloguing advanced or experimental fields
- **Dependencies:** SETCON-002, SETCON-006
- **Validation:** `cargo test -p anvil-config catalogue_seed`
- **Confidence:** medium
- **Status:** Proposed
