<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Settings Inspect Surface

| ID     | Owner | Priority | Status   | Progress |
| ------ | ----- | -------- | -------- | -------- |
| SETINS | —     | medium   | Proposed | 0/10     |

**Last reviewed:** 2026-08-06 — module created from the operator-supplied
`/settings` specification v1.1
([`plans/specs/2026-08-06-settings-truth-surface.md`](../specs/2026-08-06-settings-truth-surface.md),
spec §22 Slice 1). Gated on SETCON; no release window claimed.

> **Entry gate.** SETINS starts only after SETCON-010 (settings service and
> read-model boundary) and SETCON-008 (JSON envelope) land. This module renders
> and serialises the read model — if it needs to compute resolution, constraint
> or runtime state itself, the contract slice is incomplete and the work belongs
> in SETCON.

## Purpose

Ship the inspect-first `/settings` control centre: a searchable, grouped view of
what Anvil is configured to do, what it resolved to after policy, and what the
running system can actually prove it is enforcing — plus the matching read-only
CLI and MCP surfaces.

The first-release top-level surface is exactly:

```text
Settings | Status | Sources
```

`Audit` is deliberately absent (SETGOV owns it). Shipping an empty or partial
Audit tab would imply historical coverage Anvil cannot evidence.

## In Scope

- `/settings` slash command and `anvil settings` interactive entry, including
  `/settings <key>` focus and deep links from `/status`, diagnostics and failed
  checks
- Settings view: grouping, search, keyboard navigation, row rendering and the
  expandable detail panel
- Row honesty: resolved and active shown separately when they differ; `drift` /
  `stale` / `failed` / `unknown` never collapsed into a reassuring indicator
- Status view (operational summary) and Sources view (resolution explanation)
- Non-interactive CLI: `show`, `explain <key>`, `status`, `sources`, with
  `--format text|json`, `--json`, `--check` and `--no-tui`
- Read-only MCP equivalents of the same four views from the same read model
- Accessibility, narrow-terminal and non-TTY degradation
- Telemetry guardrails for the new surface
- Snapshot, property and golden test coverage

## Out of Scope

- Any mutation, including Class A toggles (SETPREF) — `Space`, `Enter`-to-edit
  and `d` render as inspect-only or explain the controlling authority
- Audit view and `anvil settings audit` (SETGOV)
- Natural-language querying or mutation (SETNL)
- Deprecating or changing `anvil config show` / `anvil config validate` — they
  remain the low-level compatibility interface on the same resolver
- New configurable behaviour; this module surfaces what exists

## Interfaces

**Depends on:**

- [settings-truth-contract](./settings-truth-contract.aps.md) (SETCON) —
  catalogue, resolver, constraints, runtime state, health, redaction, envelope,
  exit codes, read model
- `crates/anvil-tui` — surface host, theming and snapshot-test harness
- `crates/eddacraft-tui` — terminal mode probes and lifecycle helpers
- `crates/anvil-cli` — command registration and output-mode resolution
- MCP server surface — read-only tool and resource registration

**Exposes:**

- `/settings` control centre and its three first-release views
- `anvil settings show|explain|status|sources`
- Read-only MCP settings inspection tools

**Coordinates with:**

- [activation-tui](./activation-tui.aps.md) (ACTTUI) — shared TUI posture,
  escape hatches (`--no-tui` / `ANVIL_NO_TUI=1`) and honesty copy pins
- [daemon-protection-observability](./daemon-protection-observability.aps.md) —
  status/attestation signals the Status view reports rather than re-derives
- [cli-command-truth](./cli-command-truth.aps.md) (CLICT) — command-truth slice
  before public docs describe `anvil settings`
- [documentation-sync](./documentation-sync.aps.md) (DOCSYNC) — user docs

## Constraints

- **No new truth** — every value, badge and state comes from the SETCON read
  model; the surface may format, never infer.
- **State without colour** — `active`, `unknown`, `stale`, `failed` and `drift`
  stay distinguishable without icons or colour.
- **Deterministic rendering** — no wall-clock or randomness in rendered content;
  attestation ages render from the snapshot, not from render time.
- **Redaction fails closed** — a redaction failure produces no payload and no
  partially-rendered row.
- **Non-interactive discipline** — an explicit subcommand, `--format`, `--json`,
  `--check` or `--no-tui` never starts an alternate-screen TUI; when stdout is
  not a terminal, bare `anvil settings` behaves as `show --format text`.

## Acceptance Criteria

- [ ] `/settings` opens in an active Anvil TUI session; settings are grouped,
      searchable and fully keyboard navigable
- [ ] Every displayed resolved value shows its source or composite provenance
- [ ] Resolved and active values display separately when they differ
- [ ] A control without accepted runtime evidence displays as `unknown`
- [ ] Attested mismatch shows `drift`; superseded evidence shows `stale`;
      activation failure shows `failed`
- [ ] At least one overridden value shows its complete precedence and constraint
      chain
- [ ] Status reports runtime, project, protection posture, integration health
      and every non-healthy runtime state
- [ ] CLI and MCP inspection derive from the same revisioned read model as the TUI
- [ ] Secret and unclassified values never appear in rendering, JSON, logs,
      diagnostics or telemetry
- [ ] Snapshot tests cover normal, narrow, invalid-config, locked, unknown,
      stale, failed, drift and redacted-value states

## Ready Checklist

Change status to **Ready** when:

- [ ] SETCON-008 and SETCON-010 are Done
- [ ] SETCON-011 seeds enough catalogue entries for a non-trivial surface
- [ ] Status-view signal sources confirmed against existing diagnostics
- [ ] A CLICT slice is opened for the `anvil settings` command family

## Work Items

### SETINS-001: Settings view — grouping, search and navigation

- **Intent:** Give users one discoverable place to find any setting by name,
  key, description, group or deprecated alias.
- **Expected Outcome:** The Settings view renders the five catalogue groups in
  catalogue order; `/` focuses search; arrows and `j/k` navigate; `g/G` jump;
  `Esc` backs out without mutation; search matches labels, canonical keys,
  descriptions, groups and deprecated aliases; results retain group, source,
  constraint and state context; the footer shows only commands valid in the
  current state.
- **Non-scope:** Row detail panel (SETINS-002); any edit affordance
- **Dependencies:** SETCON-010
- **Validation:** `cargo test -p anvil-tui settings_view`
- **Confidence:** medium
- **Status:** Proposed

### SETINS-002: Row honesty and expandable detail

- **Intent:** Make a row incapable of presenting desired state as proven state.
- **Expected Outcome:** Rows carry label, resolved value, active value or
  runtime state, source badge, policy-constraint badge, runtime state, separate
  configuration/workflow state and consequence badge; the compact single-value
  form appears only when resolved and active agree; expanded detail shows the
  canonical key, per-scope declarations, merge behaviour, requested value and
  every applied constraint, active or last-known evidence with responsible
  instance and age, full provenance, accepted values and default, consequence,
  affected components, validation findings, restart requirements and the
  equivalent CLI command; reduced-evidence attestations (digest or conformance)
  explain their lower evidence level.
- **Non-scope:** Editing from the detail panel
- **Dependencies:** SETINS-001
- **Validation:** `cargo test -p anvil-tui settings_row`
- **Confidence:** medium
- **Status:** Proposed

### SETINS-003: Status view

- **Intent:** Summarise operational truth without re-deriving it.
- **Expected Outcome:** Status reports Anvil version and runtime, active project
  and worktree and session, resolved protection posture, attested active
  posture, overall health with reasons, every non-healthy runtime state, enabled
  integrations and failures, active agent adapters, pending restart or
  reactivation, configuration and policy validation state, and attestation age
  and source — all from the same status model as existing diagnostics.
- **Non-scope:** New health computation (SETCON-007 owns it)
- **Dependencies:** SETCON-007, SETINS-001
- **Validation:** `cargo test -p anvil-tui settings_status`
- **Confidence:** medium
- **Status:** Proposed

### SETINS-004: Sources view

- **Intent:** Explain how desired state was reached, and by whom it was
  constrained.
- **Expected Outcome:** Sources lists every discovered source in precedence
  order with scope and redacted path, shows organisation constraints separately
  from ordinary precedence, marks which sources this session can write, shows
  overridden declarations alongside the winning declaration, exposes
  field/member provenance for composite values, renders environment-provided
  values per catalogue redaction policy, lists unknown and deprecated keys, and
  states the revision used to generate the view.
- **Non-scope:** Editing or reset from this view
- **Dependencies:** SETCON-004, SETINS-001
- **Validation:** `cargo test -p anvil-tui settings_sources`
- **Confidence:** medium
- **Status:** Proposed

### SETINS-005: Entry points and deep links

- **Intent:** Make the control centre reachable from wherever a user notices a
  configuration question.
- **Expected Outcome:** `/settings` opens the control centre in an active TUI
  session; `/settings <key>` opens focused on a canonical key or recognised
  alias; `anvil settings` opens the same surface on a supported terminal;
  `/status`, diagnostics, failed checks and integration health can deep-link to
  a setting; a slash command invoked outside a TUI returns a concise explanation
  plus the equivalent CLI command.
- **Non-scope:** Non-interactive output formats (SETINS-006)
- **Dependencies:** SETINS-001
- **Validation:** `cargo test -p anvil-cli settings_entry`
- **Confidence:** high
- **Status:** Proposed

### SETINS-006: Non-interactive CLI inspection

- **Intent:** Give automation and non-TTY contexts the same truth as the TUI.
- **Expected Outcome:** `anvil settings show|explain <key>|status|sources` exist
  with `--format text|json` and the `--json` alias; `--check` is accepted on bare
  `anvil settings`, `show` and `status`, implies non-interactive output, and
  returns a non-zero result when a health-relevant control is invalid, `drift`,
  `stale`, `failed` or `unknown`, when a mandatory policy bundle is invalid, or
  when resolution is incomplete; advisory findings stay in the payload without
  failing the check; `--no-tui` forces structured text; no explicit-mode
  invocation starts an alternate screen; a redaction failure emits no payload.
- **Non-scope:** `set`/`unset` or any apply command; `anvil settings audit`
- **Dependencies:** SETCON-008, SETCON-009
- **Validation:** `cargo test -p anvil-cli settings_cli`
- **Confidence:** medium
- **Status:** Proposed

### SETINS-007: Read-only MCP inspection

- **Intent:** Let agents inspect settings truth without gaining a mutation path.
- **Expected Outcome:** MCP exposes read-only equivalents of `show`, `explain`,
  `status` and `sources` from the same read model, using the same canonical
  keys, classifications and recursive redaction as the CLI even where the
  transport envelope differs; no MCP mutation tool exists in v0.1.
- **Non-scope:** Audit inspection (SETGOV); any write tool
- **Dependencies:** SETINS-006
- **Validation:** `cargo test -p anvil-cli settings_mcp`
- **Confidence:** medium
- **Status:** Proposed

### SETINS-008: Accessibility and terminal degradation

- **Intent:** Keep the surface usable and honest on constrained terminals and
  assistive setups.
- **Expected Outcome:** No state is communicated by colour alone; narrow
  terminals get responsive detail views; the surface is fully keyboard operable;
  reduced-motion preferences are respected; text labels are stable for snapshot
  tests and screen readers; where alternate-screen support is unavailable the
  surface degrades to a structured text listing rather than failing.
- **Non-scope:** Editing accessibility preferences (SETPREF)
- **Dependencies:** SETINS-002
- **Validation:** `cargo test -p anvil-tui settings_a11y`
- **Confidence:** medium
- **Status:** Proposed

### SETINS-009: Telemetry guardrails

- **Intent:** Ensure the new surface cannot leak configuration through
  telemetry.
- **Expected Outcome:** If telemetry is enabled, only coarse interaction signals
  (panel opened, search used, validation failed) are collected; configured,
  resolved and active values, paths, search strings, policy contents, proposal
  diffs, secret metadata, audit details and attestation payloads are never
  collected; existing telemetry controls are honoured; telemetry failure does not
  prevent local inspection.
- **Non-scope:** Telemetry transport or backend changes
- **Dependencies:** SETINS-001
- **Validation:** `cargo test -p anvil-observability settings_telemetry`
- **Confidence:** high
- **Status:** Proposed

### SETINS-010: Inspect-surface test suite

- **Intent:** Pin the honesty guarantees so a later refactor cannot quietly
  soften them.
- **Expected Outcome:** Snapshot tests cover normal, narrow, invalid-config,
  locked, unknown, stale, failed, drift and redacted-value states; property
  tests cover precedence, constraints, composite merges, runtime-state
  classification, health aggregation and redaction invariants; golden tests pin
  the JSON contract and exit-code mapping; a test asserts no CLI/MCP/TUI surface
  can render `active` without accepted current evidence.
- **Non-scope:** Mutation-path tests (SETPREF / SETGOV)
- **Dependencies:** SETINS-002, SETINS-006, SETINS-007
- **Validation:** `cargo test -p anvil-tui`; `cargo test -p anvil-cli settings`
- **Confidence:** medium
- **Status:** Proposed
