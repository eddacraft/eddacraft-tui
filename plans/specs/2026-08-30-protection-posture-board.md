# Protection posture board

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for POSBRD-001..005 | POSBRD | Accepted | 2026-08-30 — design approved in-thread; APS filed as Proposed |

| Upstream | Downstream |
| -------- | ---------- |
| Operator design (2026-08-28..30); [ADR-002](../decisions/002-warnings-over-blocks.md); [ADR-015](../decisions/015-intercept-loop-enforcement.md); [ADR-098](../decisions/098-policy-enforcement-reset-gate.md); [ADR-129](../decisions/129-policy-surface-inventory-and-precedence.md); [ADR-132](../decisions/132-settings-truth-contract.md); `crates/anvil-kernel-types/src/{enforcement,diagnostics}.rs`; `crates/anvil-settings/src/seed.rs`; `crates/anvil-cli/src/commands/status.rs`; `schemas/anvil-status.v1.json`; [settings truth surface](./2026-08-06-settings-truth-surface.md) | [POSBRD](../modules/protection-posture-board.aps.md); SETCON seed correction; SETINS Status panel; `anvil status` TUI/plain/JSON; public CLI status docs |

**Design approved 2026-08-30.** Execution authority is [POSBRD](../modules/protection-posture-board.aps.md).
Items are Proposed; this specification does not promote them to Ready.

## 1. Problem

Operators and agents cannot see, in one place:

- which enforcement actions exist (`warn`, `block`, `fence`, `interrupt`, kill);
- what level each lifecycle surface is at right now.

The types already exist in kernel-types (ADR-098): posture is
`EnforcementMode` (`off` < `warn` < `fence` < `interrupt`); outcomes are
`ControlDecision` (`allow` / `warn` / `block` / `fence` / `interrupt`). Kill is
an interrupt-ladder stage, not a posture. Gate and acceptance policy use native
verbs and must not be collapsed onto that ladder (ADR-098 AD-7, ADR-129).

What exists today is partial:

- `anvil status` Layers answers **armed or not** (`on` / `partial` / `off` /
  `unknown`), not warn/fence/interrupt.
- `anvil config show` echoes four **unwired** rule modes (`off` / `warn` /
  `enforce`).
- `anvil intercept status` is daemon/session/fence health.
- SETCON has a settings catalogue, but `protection.enforcement.mode` is seeded
  as `off` / `warn` / `enforce` — the rule-mode trio, not the posture ladder.
- SETINS (`/settings` inspect) is Proposed and gated; `anvil settings` is not a
  shipped command.
- POLFIT-006 / DOCDEF-007 still Draft for the public *key* catalogue of
  `enforcement.mode`. That is a different job from this *level* board.

Absent `enforcement.mode`, MCP pre-write defaults to `interrupt` and the
intercept daemon defaults to `warn` (ADR-129 D-4.5). The board must show that
split, not pick a winner.

## 2. Decision

Ship a **protection posture board**: one Status-panel snapshot of configured /
resolved / active level per lifecycle surface, with native verbs and a one-line
strictness mapping.

v1 is a **sibling section on `anvil status`** (TUI, plain, and `--json`). It is
the first consumer of the Status panel. SETINS later renders the same snapshot;
it does not invent a second resolver or a new command name in v1.

Settings holds the real keys. The four rows are **projections**, not four
writable settings.

## 3. Vocabulary

| Term | Values | Job |
| ---- | ------ | --- |
| **Posture ladder** | `off` < `warn` < `fence` < `interrupt` | How strictly a block-worthy finding is acted on. Default `warn`. `block` aliases `interrupt`. |
| **Decision** | `allow`, `warn`, `block`, `fence`, `interrupt` | What happened on this finding. `block` is a veto where there is nothing to fence. |
| **Interrupt ladder** | `sigint` → `sigterm` → `sigkill` (Windows: job kill) | How interrupt is carried out. Kill is a stage, not a fifth posture. |
| **Gate verbs** | errors always fail; warnings `warnings-pass` or `warnings-fail` | Native gate admission. Source: `--fail-on-warnings` / `ANVIL_FAIL_ON_WARNINGS`. |
| **Acceptance verbs** | `on_warn` / `on_block` / none | Native L4 commit/push policy from `anvil/policy.*`. |

Do not use `enforce` as a posture value. `enforce` remains a **rule-mode** value
for the four stored kernel-invariant IDs only.

Do not number posture rows L0–L5. Those numbers belong to the Layers grid.

## 4. Board contract

### Rows

| id | Label | Verbs | Configured from | Resolved |
| -- | ----- | ----- | --------------- | -------- |
| `mcp_pre_write` | mcp pre-write | ladder | `.anvil.yaml` `enforcement.mode` | key if set; else **interrupt** |
| `intercept` | intercept | ladder | same key; daemon may raise via user config (`EnforcementMode::stricter`) | key if set (possibly raised); else **warn** |
| `gate` | gate | native | fail-on-warnings flag/env | `warnings-pass` (default) or `warnings-fail` |
| `acceptance` | acceptance | native | `anvil/policy.*` | `on_warn` / `on_block` / none |

MCP does not merge user config. The two ladder rows can disagree even when the
project key is set.

### Columns

Every row has **configured**, **resolved**, and **active**.

- **Configured** is the declared source value. Use `(unset)` / `(none)` when the
  source is absent. Never copy Resolved into Configured.
- **Resolved** is the value after defaults, stricter-wins, and policy
  constraints (SETCON requested → resolved).
- **Active** is SETCON-strict: the value current evidence from the activation
  owner proves (`active`). The full SETCON `RuntimeState` set is `active`,
  `drift` (attested evidence disagrees with Resolved), `unknown`, `stale`, and
  `failed`. YAML is never proof.

### Last action

Active is **live posture**. Last action is intercept-row detail only, and only
when attested:

- decision: `warn` / `fence` / `interrupt` (or `block` if that is what the
  surface recorded)
- stage when decision is `interrupt`: `sigint` / `sigterm` / `sigkill` /
  `job_object` / `already_exited`
- timestamp

Omit the detail when the daemon cannot attest it. `query_status` does not carry
this today; v1 may add an additive field, or leave last-action absent until that
evidence exists. Do not synthesise last-action from fences alone.

### Strictness mapping (legend, not a column)

```text
scale: off < warn < fence < interrupt
       warn ≈ warnings-pass ≈ on_warn
       interrupt ≈ warnings-fail ≈ on_block   (block → interrupt)
```

Gate and acceptance never display `fence` or `interrupt` as their own value.

## 5. Placement on `anvil status`

Layers stays the armed/not-armed grid (L0 mcp … L5 audit).

Posture is a **sibling block** after Layers. Same snapshot feeds:

- TUI status surface (`crates/anvil-tui/src/surfaces/status`)
- plain `anvil status` (`--no-tui` / non-TTY)
- `anvil status --json`

Sketch (plain):

```text
  Posture:
    surface         configured     resolved        active
    mcp pre-write   (unset)        interrupt       unknown
    intercept       (unset)        warn            warn
    gate            (unset)        warnings-pass   unknown
    acceptance      (none)         (none)          unknown
    intercept last: interrupt  sigkill  12s ago
    scale: off < warn < fence < interrupt
           warn ≈ warnings-pass ≈ on_warn
           interrupt ≈ warnings-fail ≈ on_block   (block → interrupt)
```

Three of four Active cells may be `unknown` on a quiet repo. That is honest.

## 6. Active evidence (v1)

| Row | Activation owner | v1 evidence |
| --- | ---------------- | ----------- |
| mcp pre-write | MCP / intercept pre-write path | Current attested pre-write posture; else `unknown` |
| intercept | intercept daemon | `query_status` current mode while IPC is live |
| gate | last gate run | `.anvil/gate-history.ndjson` most recent run; else `unknown` |
| acceptance | hook / witness attester | Current attested L4 posture; else `unknown` |

Missing, stale, or failed evidence is named. Do not fill Active from Resolved.

## 7. JSON

Keep `schema_version` as `anvil.status.v1`. Add an optional `posture` object.
Top-level `additionalProperties: true` already allows additive fields; still
document `posture` in `schemas/anvil-status.v1.json` so the contract is explicit.
Renaming or removing fields still requires a schema bump.

Required shape (conceptual):

```json
{
  "posture": {
    "scale": ["off", "warn", "fence", "interrupt"],
    "rows": [
      {
        "id": "mcp_pre_write",
        "verbs": "ladder",
        "configured": { "value": null, "source": "enforcement.mode" },
        "resolved": { "value": "interrupt", "reason": "absent_key_default" },
        "active": { "value": null, "state": "unknown" },
        "last_action": null
      }
    ]
  }
}
```

`last_action` is present only on `intercept` when attested; otherwise `null` or
omitted. TUI, plain, and JSON must serialise the same snapshot.

## 8. Settings catalogue correction

Same programme, not a second product.

Today `crates/anvil-settings/src/seed.rs` allows
`protection.enforcement.mode` = `off` / `warn` / `enforce`, and
`crates/anvil-settings/src/types.rs` `Posture` is `Off` / `Warn` / `Enforce`.
That is wrong for this key.

Required:

- Catalogue enum for `protection.enforcement.mode`: `off`, `warn`, `fence`,
  `interrupt`. Default remains `warn`.
- Four rule-mode keys keep `off` / `warn` / `enforce`.
- Org min-posture constraints on the enforcement-mode key use `EnforcementMode`
  order (`off` < `warn` < `fence` < `interrupt`). Do not collapse `fence` /
  `interrupt` to `enforce`.

The board is a Status view over that catalogue plus surface defaults. It is not
four new catalogue keys.

## 9. Boundaries

**Uses** SETCON read model (`eddacraft-anvil-settings`) for configured/resolved
and runtime-state classification. Status renders; it does not become a second
resolver.

**Does not replace** Layers, `anvil intercept status`, or `anvil config show`.

**Does not add** `anvil settings` / `anvil protection` / `anvil enforcement`
(CLICT / SETINS). SETINS later consumes this snapshot on its Status tab.

**Does not** pick an absent-key winner (ADR-129 D-4.5), wire rule modes, or add
finding-family rows. `--fail-on-warnings` is the gate row's source, not its own
row. Watch remains Layers L2.

POLFIT-006 / DOCDEF-007 remain the public *key* catalogue for `enforcement.mode`.
This spec is the *level* board. They should agree on the ladder values when both
ship.

## 10. Non-goals

- SETINS TUI `/settings` delivery, mutation, or Audit tab
- A single merged default for MCP vs daemon when the key is unset
- Treating kill, `ANVIL_POLICY_ENFORCEMENT=off`, or feature-flag kill switches as
  posture rungs
- Evaluating the four stored rule modes
- Changing intercept routing, gate exit codes, or L4 `on_block` semantics
- A release claim for this board

## 11. Execution

Exclusive module **[POSBRD](../modules/protection-posture-board.aps.md)**.
Items are Proposed until an operator promotes them to Ready:

1. **POSBRD-001** — correct SETCON seed and constraint types; tests that
   `protection.enforcement.mode` rejects `enforce` and accepts `fence` /
   `interrupt`.
2. **POSBRD-002** — posture snapshot read model (four rows, three columns,
   runtime state, mapping legend).
3. **POSBRD-003** — `anvil status` TUI, plain, and `--json` sibling section;
   schema documents `posture`; golden/snapshot coverage.
4. **POSBRD-004** — intercept last-action evidence (additive daemon status or
   honest omit).
5. **POSBRD-005** — public CLI status docs for the section; no `anvil settings`
   claim.

Validation (when items are Ready):

```text
cargo test -p eddacraft-anvil-settings -- catalogue_seed
cargo test -p eddacraft-anvil -- status
cargo test -p eddacraft-anvil-tui -- status
pnpm docs:check
```

Exact commands belong on the APS items, not here.

## 12. Open questions

none
