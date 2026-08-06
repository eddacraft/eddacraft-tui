<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Settings Natural-Language Proposals

| ID    | Owner | Priority | Status | Progress |
| ----- | ----- | -------- | ------ | -------- |
| SETNL | —     | low      | Draft  | 0/4      |

**Last reviewed:** 2026-08-06 — module created from the operator-supplied
`/settings` specification v1.1
([`plans/specs/2026-08-06-settings-truth-surface.md`](../specs/2026-08-06-settings-truth-surface.md),
spec §16 and §22 Slice 4). Horizon work; not scheduled.

> **Posture.** Natural language is an **untrusted proposal-authoring input**,
> never an authority or a mutation path. This module adds a way to *draft* a
> proposal, and nothing else. If implementing it requires any new capability in
> the mutation, authority or validation path, that is a signal the design is
> wrong.

## Purpose

Let a user say what they want changed and get back an exact, redacted proposal —
which then travels the identical governed path a hand-authored proposal takes,
with the same consequence analysis, confirmations, approvals and audit.

## In Scope

- Parsing requested intent into candidate canonical keys and a target scope
- Displaying assumptions and ambiguity before anything is generated
- Generating the proposal through the settings service
- Consequence explanation in user language, sourced from the deterministic
  evaluator
- Normal approval and audit integration

## Out of Scope

- Any independent authority — the model cannot weaken policy, select authority
  on the user's behalf or bypass deterministic validation
- Any model access to secret values
- Applying a change without the ordinary confirmations and approvals
- A parallel key resolution or validation implementation

## Interfaces

**Depends on:**

- [settings-governed-changes](./settings-governed-changes.aps.md) (SETGOV) —
  proposal contract, consequence analysis, approval routing, audit
- [settings-truth-contract](./settings-truth-contract.aps.md) (SETCON) —
  catalogue, aliases, redaction

**Exposes:**

- "Ask Anvil to change a setting" authoring entry inside `/settings`

## Constraints

- **Untrusted input** — parsed intent is a suggestion, validated the same way a
  typed key would be.
- **No secrets to the model** — enforced by SETCON classification, verified here.
- **Same path or no change** — the flow ends in the authoritative mutation path
  or it makes no change at all.

## Acceptance Criteria

- [ ] A natural-language request produces an exact redacted proposal, never a
      direct write
- [ ] Assumptions and ambiguity are shown before proposal generation
- [ ] The model receives no secret values and cannot alter required authority,
      approvals or validation outcomes
- [ ] Outcomes appear in Audit indistinguishably in rigour from hand-authored
      proposals

## Ready Checklist

Change status to **Ready** when:

- [ ] SETGOV is Done and the governed path has real usage
- [ ] Prompt-injection posture for settings intent parsing is reviewed
- [ ] Operator confirms the feature is wanted

## Work Items

### SETNL-001: Intent parsing to candidate keys

- **Intent:** Turn a natural-language request into candidate canonical keys and
  a target scope.
- **Expected Outcome:** A request resolves to zero or more candidate catalogue
  keys (including via deprecated aliases) and a candidate scope; unresolvable or
  out-of-catalogue requests return an honest "no candidate" result rather than a
  guess.
- **Dependencies:** SETGOV-001
- **Validation:** `cargo test -p anvil-config settings_nl_parse`
- **Confidence:** low
- **Status:** Draft

### SETNL-002: Assumption and ambiguity disclosure

- **Intent:** Never act on an interpretation the user has not seen.
- **Expected Outcome:** Before a proposal is generated, the surface shows the
  interpreted key, scope, value and every assumption made, plus alternatives
  when the request is ambiguous; the user selects or cancels.
- **Dependencies:** SETNL-001
- **Validation:** `cargo test -p anvil-tui settings_nl_ambiguity`
- **Confidence:** medium
- **Status:** Draft

### SETNL-003: Proposal generation through the service

- **Intent:** Reuse the governed path rather than shadowing it.
- **Expected Outcome:** The confirmed interpretation produces a proposal via the
  settings service with identical contents, consequence analysis and validation
  to a hand-authored one; model output cannot alter required authority,
  approvals or validation results; a rejected or stale proposal behaves exactly
  as it would in SETGOV.
- **Dependencies:** SETNL-002
- **Validation:** `cargo test -p anvil-config settings_nl_proposal`
- **Confidence:** medium
- **Status:** Draft

### SETNL-004: Approval and audit integration

- **Intent:** Make natural-language origin visible in the record without
  weakening it.
- **Expected Outcome:** The normal confirmations and approvals are requested;
  the change applies through the authoritative mutation path or makes no change;
  the audit record captures the outcome and marks the proposal's authoring
  origin; no secret value or unredacted diff reaches the model or the record.
- **Dependencies:** SETNL-003
- **Validation:** `cargo test -p anvil-config settings_nl_audit`
- **Confidence:** medium
- **Status:** Draft
