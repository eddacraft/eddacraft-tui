# Activation TUI completion programme

| Type | Authority     | Owner | Status | Freshness                                      |
| ---- | ------------- | ----- | ------ | ---------------------------------------------- |
| Spec | Authoritative | ACTTUI | Live  | Filed 2026-08-03 from owner-directed scoping |

| Upstream                                                                 | Downstream                                      |
| ------------------------------------------------------------------------ | ----------------------------------------------- |
| [`activation-tui.aps.md`](../modules/activation-tui.aps.md), ADR-103, ADTRUST-006, CIB-164/165/166/183 | ACTTUI-014..017 execution |

## Purpose

Define what **full completion** of the interactive `anvil start` TUI means after
the TTY-default flip (ACTTUI-013) and the continuous-surface follow-up
(ACTTUI-014). This spec is the product contract for ACTTUI-015..017 and for
expanding ACTTUI-014 acceptance beyond “one alt-screen session.”

## Product principle

Great is not more chrome. It is **one honest loop** that answers:

1. **Am I protected right now?** — one state word, one meaning.
2. **What should I do next?** — exactly one next step (CIB-166 arbiter).
3. **Can I prove it in under 30 seconds?** — optional real proof, never a fake finding.

## Personas and success

| Persona | Job | Success |
| ------- | --- | ------- |
| First day | Wire editors, reach protecting | Consent once → clear protecting or one restart step → optional real proof |
| Daily return | Health check | No consent parade; headline + Next; quit in two keys |
| Repair | Something broken | Auto-expanded repair evidence; one fix recipe; re-verify in place |
| Skeptic | Show value | One key runs a real check and reports a real finding or an honest “can’t prove here” |
| Script / CI | Automation | Unchanged `--verify` / `--json` / `--no-tui` contracts |

## Work packages (map to APS)

| Package | APS item | Intent |
| ------- | -------- | ------ |
| **WP0 — Honesty** | [ACTTUI-015](../modules/activation-tui.aps.md#acttui-015-honesty-hotfix-for-smokeprove-and-help) | Stop advertising a broken Smoke control; one help bar; no false plain-path recipe claim on re-runs |
| **WP1 — Continuous quiet surface** | [ACTTUI-014](../modules/activation-tui.aps.md#acttui-014-continuous-live-activation-surface) (expanded) | Live preflight→consent→verdict; skip Consent when nothing is actionable; progress hand-off; pinned Next |
| **WP2 — Prove** | [ACTTUI-016](../modules/activation-tui.aps.md#acttui-016-prove-protection-in-surface) | In-surface execution of the ADTRUST-006 recipe with real engine results |
| **WP3 — Posture alignment** | [ACTTUI-017](../modules/activation-tui.aps.md#acttui-017-align-start-and-status-posture) | Shared protection / next-step language between start Verdict and `anvil status` |

## What great looks like

### First run

```text
Working (live steps)
  → Consent (only real offers, unticked)
  → Apply selection
  → Verdict: protecting (or ready_restart_required with one Next)
  → optional Prove → real secret-detection finding, file cleaned up
  → q
```

### Healthy re-run

```text
Working (short)
  → Verdict immediately (no multi-client consent list when nothing to write)
  → protecting · Next: …
  → q
```

### Repair re-run

```text
→ Verdict with Layers/Install auto-expanded
→ Next names the single repair action
→ e for evidence detail
→ Prove still available when gates allow
```

## Prove (WP2) contract

Rename the user-facing control from **Smoke** to **Prove** (key may remain `t`
if help text is clear).

| Rule | Requirement |
| ---- | ----------- |
| Engine | Real check path (same family as `anvil check`), not a staged demo finding |
| Recipe | Reuse ADTRUST-006 throwaway secret-shaped fixture and `secret-detection` expectation |
| Claims | Proof of **check pipeline** only — do **not** claim MCP pre-write is live from a CLI check |
| Gates | Disable with reason when: all languages unsupported; no secret-detection in active checks; project writes gated and repo write would be required; disk not writable |
| Cleanup | Always remove the throwaway file (best-effort on panic/quit) |
| Temp location | Prefer OS temp (or other non-durable path) so daily Prove does not need repo-write consent; if the file must live in-repo, one explicit confirm per session |
| Language | Global on Verdict — not “smoke this language row.” May pick fixture extension from supported languages that can exercise secret-detection |
| Failure honesty | No finding → honest failure + Next; never invent success |

## Explicit non-goals

- Per-language smoke actions on every Languages row
- Full-repo `gate` or full scan from start
- Auto-ticking MCP/workflow consent
- Celebration on every re-run (first protecting only remains JOURNEY-008)
- json_render / Pretext adoption for activation
- Replacing `anvil status` as a command
- Claiming editor MCP intercept from CLI Prove

## Constraints (carry forward)

- Honesty pins (LAUNCH-014 / ADR-092): state labels and wired-vs-live layer vocabulary unchanged (CIB-164).
- Consent defaults remain unticked (CIB-165); empty apply writes nothing.
- Machine contracts (`--verify`, `--json`, piped/`--no-tui`) stay byte-stable.
- `ANVIL_HOME` gated posture suppresses repo-scoped offers and Prove if it would write the project.

## Validation programme (cross-item)

- Unit/snapshot coverage for verdict help labels, Prove gates, and toast/panel copy.
- PTY: flag-free start → verdict; empty-offer re-run skips Consent; Prove produces a finding then cleans up; `q` restores terminal mode.
- Plain path: CIB-183 re-run collapse still omits the full recipe; Prove does not depend on that plain recipe being printed.
- `pnpm aps:active-lint` / `pnpm aps:index:check` after plan edits; focused `cargo test` per work item Validation field.

## Sequencing

```text
ACTTUI-015 (honesty)  ──parallel──►  ACTTUI-014 (continuous + quiet consent)
         │                                    │
         └──────────► ACTTUI-016 (Prove) ◄────┘  (015 before or with 016; 014 preferred before Prove UX polish)
ACTTUI-017 (start↔status) parallel after 014 verdict model is stable
```

## Source of this programme

Owner-directed scoping 2026-08-02/03 after live Homebrew validation of the
activation TUI (Consent re-offer noise, placeholder `t` smoke toast claiming a
“contract-hardening slice” and a `--no-tui` recipe that CIB-183 re-runs omit).
Related shipped foundation: ACTTUI-005 thin-v1 toast, ACTTUI-007 fixture
hardening (not in-surface smoke), ADTRUST-006 plain recipe, CIB-183 quiet re-run.
