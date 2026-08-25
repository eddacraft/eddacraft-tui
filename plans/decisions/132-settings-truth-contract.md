# ADR-132: Settings Truth Contract

## Status

Accepted 2026-08-25 (operator)

## Date

2026-08-25

## Context

Anvil configuration is spread across project files, user settings, environment
variables, runtime adapters, hooks, organisation policy and command defaults.
Surfaces today can show a declared value; they cannot prove that the running
system is enforcing it. The operator-supplied `/settings` specification v1.1
([`plans/specs/2026-08-06-settings-truth-surface.md`](../specs/2026-08-06-settings-truth-surface.md))
fixes the product vocabulary, but later slices will invent a second, softer
truth unless the contract is an accepted decision.

SETCON (settings-truth-contract) is Slice 0 of that programme. It ships no
`/settings` screen. It must still lock:

- the terminology (`configured` / `requested` / `resolved` / `active`);
- the mutually exclusive runtime-state model;
- the settings-service boundary (one writer, one read model);
- the catalogue home crate;
- the attestation transport;
- the mapping from settings semantic outcomes onto Anvil's global exit codes.

UCFG / ADR-120 already owns file layout and discovery. This ADR does not
redesign `.anvil.<ext>`.

## Decision

### 1. Terminology

These words are canonical. Do not introduce synonyms in product copy, JSON
fields, logs, or diagnostics.

| Term | Meaning |
| --- | --- |
| Configured value | A declaration made by one configuration source. |
| Requested value | The value requested by the applicable declarations **before** policy constraints. |
| Policy constraint | A rule that limits the permitted value space. Constraints are **not** a precedence source. |
| Resolved value | The desired value after precedence, merge rules **and** policy constraints. |
| Active value | The value that accepted, current evidence from the responsible runtime component proves it is enforcing. |
| Runtime attestation | Structured runtime evidence bound to a responsible component instance and configuration revision. Not hardware-backed unless a later ADR says so. |

The word **`effective` is banned**. It collapses resolved and active, which is
the failure mode this contract exists to prevent.

Configuration and workflow states stay distinct from runtime evidence:

- `invalid` — the declaration or resolved value failed validation;
- `locked` — a constraint or authority forbids mutation;
- `pending activation` — a write succeeded and activation has been requested
  but is not yet proven.

None of those three is a runtime-state value.

### 2. Runtime-state model

For every evidence-bearing key, the classifier assigns **exactly one** of:

1. `unknown` — no accepted evidence exists.
2. `stale` — accepted evidence refers to a previous instance or revision, has
   expired, or its component has disconnected or exited.
3. `failed` — current evidence reports activation failure or incompatibility.
4. `drift` — current evidence proves active enforcement differs from resolved
   state.
5. `active` — current evidence proves active enforcement matches resolved state.

The order above is total and deterministic. Absence of attestation is
`unknown`, never `active`. No path may convert `stale`, `failed`, `unknown` or
`drift` into a reassuring indicator.

Evidence is accepted only from the catalogue's registered activation owner,
over a channel that meets the catalogue's trust requirement. Evidence from an
unregistered owner, an untrusted channel, or an incompatible component is
rejected and cannot produce `active`.

Keys declared `evidence_mode: none` never classify as `active`.

### 3. Settings-service boundary

One settings service owns catalogue loading, source discovery and revision
capture, resolution, constraint evaluation, evidence acceptance, snapshot
generation, health aggregation and (later, SETPREF/SETGOV) mutation.

- TUI, CLI, MCP, diagnostics and tests consume the **same** catalogue,
  resolver, constraint engine, redaction rules and read-model snapshot.
- No consumer reads configuration files directly for settings purposes.
- **No interface writes configuration except through the settings service.**
  Existing `anvil config set` / `anvil init` / `anvil start` writers are
  compatibility paths that SETPREF/SETGOV must route through or subsume; they
  are not a second truth.
- Inspection may return a valid model that describes invalid or unhealthy
  state.
- Redaction is recursive, applied to every output channel from one
  implementation, and **fail-closed**: a redaction failure emits no settings
  payload, on any channel.

### 4. Catalogue home crate

The typed catalogue, resolver, constraint layer, runtime-state classifier,
health aggregator, `anvil.settings.v1` envelope and settings service live in a
new crate:

- package: `eddacraft-anvil-settings`
- library: `anvil_settings`
- path: `crates/anvil-settings`

`eddacraft-anvil-config` (`anvil_config`) remains the multi-format file
loader and discovery crate (ADR-120). The settings crate depends on it for
source discovery; it does not absorb it.

Rejected: stuffing the service into `anvil-config` (that crate's contract is
parse/discover/canonicalise, not a read-model service) and a later extract
(the extraction is the expensive part; Slice 0 is the moment to draw the
line).

### 5. Attestation transport

Reuse the intercept **daemon RPC** already used for save-time
`validate_paths` and session registration. No new evidence channel in SETCON.

- Trust class for first-release enforcement-critical controls: daemon-attested
  over that RPC.
- Components that have not yet implemented the activation-status contract
  stay `unknown`. Instrumenting each owner remains with the owning module.
- A new purpose-built attestation protocol is out of scope until a later ADR
  shows the daemon RPC cannot carry the evidence shape.

### 6. Global exit-code mapping

Settings semantic outcomes map onto the existing global CLI registry. They do
**not** invent a command-local numbering. Existing command exits are not
renumbered.

| Semantic outcome | Code | Existing constant | Notes |
| --- | --- | --- | --- |
| `success` | 0 | `EXIT_OK` | Unchanged. |
| `internal_error` | 1 | `EXIT_ERROR` | Unchanged general failure. |
| `check_failed` | 2 | `EXIT_GATE_FAIL` | `--check` found unhealthy or indeterminate required state. |
| `access_error` | 3 | `EXIT_AUTH_REQUIRED` | Unchanged auth gate. |
| `resolution_error` | 4 | `EXIT_CONFIG_ERROR` | Catalogue, sources, constraints or read model could not resolve safely. |
| `usage_error` | 2 | clap parse failure / `EXIT_GATE_FAIL` | **Recorded, not changed:** clap usage failures already exit 2. Settings commands must not invent a different usage code. |
| `redaction_error` | 8 | `EXIT_REDACTION_ERROR` (new) | Fail-closed; no settings payload. Claims reserved code 8, previously held for future expansion. Code 9 stays reserved. |

Codes 5, 6, 7 and 10 remain the MLP/DLIFE reservations already declared in
`crates/anvil-cli/src/main.rs` and are unused by settings.

### 7. Envelope

Machine output is one JSON document, schema `anvil.settings.v1`, fields:

`schema_version`, `command`, `generated_at`, `model_revision`, `context`,
`health`, `data`, `diagnostics`.

Additive optional fields may appear within `anvil.settings.v1`. Removing,
reinterpreting, or changing a closed-enum value requires a new schema
version. Display labels are not API. Canonical keys are.

## Rationale

The specification already decided the product words and the evidence-not-
inference rule. What it left open was where the code lives and how evidence
moves. Putting the service in a new crate keeps ADR-120's loader small and
gives SETINS/SETPREF a stable dependency. Reusing daemon RPC avoids a second
trust channel before any component is instrumented. Mapping settings
outcomes onto the existing exit registry keeps `anvil && deploy` semantics
intact.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| New `anvil-settings` crate (chosen) | Clean boundary; loader stays a loader; SETINS depends on one crate | New workspace member; hakari/lock churn |
| Extend `anvil-config` | Matches the draft SETCON validation package names | Mixes parse/discover with a read-model service; hard to reverse |
| Sibling module, extract later | Least churn now | The expensive extract happens after surfaces have coupled to the wrong crate |
| New attestation protocol | Purpose-built evidence shape | Two trust channels; nothing to speak it yet |
| Command-local settings exit codes | Could make `usage_error` distinct from clap's 2 | Breaks the global registry the spec requires |

## Consequences

- **Positive:** SETINS can render from one read model. Organisation
  constraints cannot be bypassed by higher-precedence local declarations.
  Secrets and unclassified values have one redaction implementation.
- **Negative:** Slice 0 is a new crate plus an ADR; `/settings` still does
  not exist until SETINS. Daemon RPC must grow an activation-status payload
  when owners are instrumented.
- **Risks:** `usage_error` and `check_failed` share numeric code 2 (pre-
  existing clap/gate collision). Callers that need to distinguish them must
  read the JSON envelope, not the exit code.
- **Mitigations:** Golden-test the mapping. Envelope `diagnostics` carry the
  semantic outcome. SETINS `--json` is the machine contract; exit codes stay
  the coarse signal.

## References

- Related ADRs: [ADR-120](120-config-surface-consolidation.md) (config
  discovery), [ADR-002](002-warnings-over-blocks.md) (warnings vs blocks),
  [ADR-088](088-dpo-observation-kind-taxonomy.md) (observation kinds)
- APS modules: [SETCON](../modules/settings-truth-contract.aps.md),
  [SETINS](../modules/settings-inspect-surface.aps.md),
  [SETPREF](../modules/settings-safe-preferences.aps.md),
  [SETGOV](../modules/settings-governed-changes.aps.md)
- Spec: [`2026-08-06-settings-truth-surface.md`](../specs/2026-08-06-settings-truth-surface.md)
