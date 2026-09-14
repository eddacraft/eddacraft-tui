---
id: status
title: anvil status
description:
  Read the protection posture board on anvil status — configured, resolved, and
  attested active levels for each lifecycle surface.
owner: POSBRD
upstream:
  - crates/anvil-cli/src/commands/status.rs
  - crates/anvil-cli/src/commands/status_posture.rs
  - crates/anvil-settings/src/posture.rs
  - crates/anvil-tui/src/surfaces/status
  - schemas/anvil-status.v1.json
  - plans/specs/2026-08-30-protection-posture-board.md
verified_against: 0.10.0-beta
---

# anvil status

`anvil status` shows project protection health. Beside the Layers grid (armed or
not) it now prints a **Posture** section: the enforcement action ladder and the
current configured / resolved / active level of each lifecycle surface. The same
snapshot feeds the TUI, plain text, and `anvil status --json`. This is not a new
command.

This page does **not** document a settings command. Catalogue keys for
`enforcement.mode` stay with the public key catalogue.

## Posture rows

| Row             | Label         | Verbs  | Configured from                                 | Absent-key resolved |
| --------------- | ------------- | ------ | ----------------------------------------------- | ------------------- |
| `mcp_pre_write` | mcp pre-write | ladder | `.anvil.yaml` `enforcement.mode`                | `interrupt`         |
| `intercept`     | intercept     | ladder | same key; daemon may raise                      | `warn`              |
| `gate`          | gate          | native | `--fail-on-warnings` / `ANVIL_FAIL_ON_WARNINGS` | `warnings-pass`     |
| `acceptance`    | acceptance    | native | `anvil/policy.*`                                | `(none)`            |

Ladder values are `off` < `warn` < `fence` < `interrupt`. `block` aliases
`interrupt` on posture. Gate and acceptance never display `fence` or `interrupt`
as their own value.

## Columns

Every row has **configured**, **resolved**, and **active**.

- **Configured** is the declared source value. `(unset)` or `(none)` when the
  source is absent. Resolved is never copied into Configured.
- **Resolved** is the value after defaults, stricter-wins, and policy
  constraints.
- **Active** is attested. YAML is never proof. Missing evidence is `unknown`,
  `stale`, or `failed`.

Three of four Active cells may be `unknown` on a quiet repo. That is honest.

## Last action

Active is live posture. **Last action** is intercept-row detail only, and only
when the daemon attests it:

- decision: `warn` / `fence` / `interrupt` (or `block` if that is what the
  surface recorded)
- stage when decision is `interrupt`: `sigint` / `sigterm` / `sigkill` /
  `job_object` / `already_exited`
- timestamp

Kill is an interrupt-ladder stage, not a fifth posture. Fences listed on the
snapshot are not last-action evidence. If `query_status` cannot attest the
detail, it is omitted.

## Strictness mapping

```text
scale: off < warn < fence < interrupt
       warn ≈ warnings-pass ≈ on_warn
       interrupt ≈ warnings-fail ≈ on_block   (block → interrupt)
```

## JSON

`anvil status --json` keeps `schema_version` as `anvil.status.v1` and adds an
optional `posture` object. The wire shape is documented in the shipped
`anvil.status.v1` JSON Schema (`schemas/anvil-status.v1.json`).

## See also

- [CLI command reference](cli.md)
- [Policy model](../concepts/policy-model.md)
