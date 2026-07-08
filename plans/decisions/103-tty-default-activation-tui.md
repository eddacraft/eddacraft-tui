# ADR-103: TTY-Default Activation TUI for `anvil start`

## Status

Proposed

## Date

2026-07-08

## Context

`anvil start` is the daily golden path (ADR-092). Today it renders a plain-text
activation dossier on stderr and drives consent through `demand` pickers, then
prints one literal `ProtectionState`. The 2026-07-04 welcome/start user-journey
audit
([`plans/audits/2026-07-04-anvil-start-welcome-user-journey.md`](../audits/2026-07-04-anvil-start-welcome-user-journey.md))
found two structural gaps: re-run verbosity (the full dossier reprints on every
`protecting` re-run) and picker scaling (hand-rolled `demand::MultiSelect`
overlays do not scale to many MCP clients / workflows). The
[`activation-tui`](../modules/activation-tui.aps.md) (ACTTUI) module replaces the
plain dossier and `demand` pickers with a single interactive surface built on the
`eddacraft-tui` widget vocabulary (ADR-047 / ADR-050 / ADR-054).

Making a TUI the **default** on an interactive terminal is a load-bearing,
hard-to-reverse public behaviour change. Two consumer classes must never
regress:

1. **Scripted / CI consumers** parse `anvil start --verify` and
   `anvil start --json` stdout, and expect a bounded compact summary when output
   is piped. A TUI that captures the terminal or changes those byte streams
   would silently break them.
2. **Downstream first-run surfaces** — the welcome hub (ACTTUI-008) and the
   first-run-wow design gates WOW-005 (guided first-fix write) and WOW-006
   (autoplay demo) — must reuse one consent posture and one widget set rather
   than hand-rolling divergent chrome. PR #3231 pins WOW-005 on ACTTUI-000 (this
   contract) + ACTTUI-004 (consent phase) and WOW-006 on the ACTTUI foundation.

A decision is needed **now**, before ACTTUI-001 code lands, because the trust
boundary (what is interactive vs plain), the byte-stability commitments, and the
rollout order are contracts that the whole module and its WOW consumers build
against. The Planning Council `proceed`-with-`amend` verdict (2026-07-08) made
this ADR the gate for marking ACTTUI-001 **In Progress**.

## Decision

Adopt a **TTY-default interactive activation TUI** for `anvil start`, behind a
staged rollout, with an explicit trust boundary and preserved machine contracts.

### 1. Trust boundary — when the TUI renders

The interactive surface renders **only** when the process is genuinely
interactive and the surface is opted in. Every other context gets a deterministic
compact plain summary with no keypress hang and unchanged exit codes.

| Context | Detection | Surface |
| ------- | --------- | ------- |
| Interactive terminal, opted in | stdin **and** stderr are a TTY, `CI` unset, `ANVIL_NO_PROMPT` unset, and the ACTTUI opt-in is active (see rollout ladder) | Interactive activation TUI |
| Piped / redirected stdout | stdout not a TTY | Compact plain summary (bounded) |
| CI | `CI` set (or non-interactive stdin/stderr) | Compact plain summary |
| `--no-tui` / `ANVIL_NO_TUI=1` | explicit opt-out (mirrors the global `--no-tui`) | Compact plain summary |
| `--verify` | read-only probe | Byte-stable verify stdout (unchanged) |
| `--json` | machine document | Byte-stable JSON stdout (unchanged) |

Interactivity uses the existing `start_is_interactive()` predicate (stdin **and**
stderr TTY, `CI` absent, `ANVIL_NO_PROMPT` absent). The TUI is **additive on the
default TTY path only**; it never changes the read-only (`--verify` / `--json`)
or piped byte streams.

### 2. Machine-contract stability

- `anvil start --verify` and `anvil start --json` stdout are **byte-stable** and
  are pinned by the fixtures defined in the ACTTUI-000 fixture spec
  ([`plans/specs/2026-07-08-activation-tui-contract-fixtures.md`](../specs/2026-07-08-activation-tui-contract-fixtures.md)).
- Piped (non-TTY) compact stdout is bounded (target ≤10 lines on a `protecting`
  re-run) and is also pinned by a fixture.
- The `read_only` code path stays isolated from the TUI path; a byte-diff test
  against the pre-ACTTUI fixtures guards it (ACTTUI-007).
- Exit-code semantics (including MCP install failure → non-zero) are unchanged.
- Honesty pins are unchanged: `ProtectionState` labels, tier vocabulary, and the
  `protecting` / `watching` / `ready_restart_required` gates (LAUNCH-014,
  ADR-092); L0 is never listed as "active" when only wired (CIB-164).

### 3. Consent posture (the contract WOW-005 reuses)

The interactive consent phase is a shared, reusable contract, not a
start-only surface:

- MCP, workflow, and hook consent use `Select` / `Confirm` / `OverlayStack`
  chrome with **all items unticked by default** (CIB-165 parity — plain Enter
  writes nothing).
- Drift state appears in item descriptions only, never as implicit consent;
  `UnsafeDrift` never appears in a multi-select.
- Repo-scoped write offers (workflows, hooks, `.anvilrc`, baseline, witness) are
  **suppressed** when `project_writes_gated` (a gated `ANVIL_HOME`); a persistent
  banner states the reason.
- Non-interactive contexts keep the existing orchestrator auto-install policy
  (CI/piped/`--no-tui` may auto-install MCP; they never auto-write
  workflows/hooks/project seeding under a gated `ANVIL_HOME`).

### 4. Rollout ladder

1. **Release 1 — opt-in.** The TUI ships behind an opt-in: `--tui` **or**
   `ANVIL_ACTIVATION_TUI=1`. The default TTY path is unchanged
   (`render_compact()` / current behaviour) until the contract matrix is green.
2. **Release 2 — TTY-default flip.** Flip the TUI to default on the interactive
   path **only after** the ACTTUI-007 contract matrix (verify/json/CI/PTY) and
   the welcome surface (ACTTUI-008) pass together — the release cohort ships
   `anvil start` and `anvil welcome` in the same train.
3. `--no-tui` / `ANVIL_NO_TUI=1` remains a permanent escape hatch after the flip.

`ANVIL_NO_TUI=1` mirrors the global `--no-tui`. `ANVIL_ACTIVATION_TUI=1` is the
first-release opt-in and is retired (accepted as a no-op alias) once TTY-default
lands.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| TTY-default TUI, staged opt-in first (chosen) | Flagship wow on the default path; machine contracts pinned before the flip; one consent posture for start + welcome + WOW | Two-step rollout; snapshot/e2e churn while compact and TUI paths coexist |
| Flip TTY-default immediately | Less flag bookkeeping | Ships a load-bearing default before the verify/json/PTY matrix is proven; high regression risk for scripted consumers |
| Keep plain dossier; add TUI only behind a permanent flag | Zero default-path risk | Fails the operator goal of a flagship default experience; leaves the audit gaps unaddressed |
| Per-surface bespoke chrome (start, welcome, WOW each own theirs) | Independent teams move fast | Divergent consent posture (CIB-165 risk) and inconsistent first-run product feel; duplicated widget work |

## Consequences

- **Positive:** The default interactive `anvil start` becomes the flagship
  `eddacraft-tui` surface with progressive disclosure, honest unticked consent,
  and collapsible diagnostics. Machine contracts are pinned by fixtures before
  any default flip. WOW-005/006 and ACTTUI-008 inherit one consent posture and
  widget vocabulary.
- **Negative:** A staged rollout carries flag bookkeeping (`--tui` /
  `ANVIL_ACTIVATION_TUI`) and snapshot/e2e churn while the compact-plain and TUI
  paths coexist.
- **Risks:** First production load on `Tree` / `ParallelProgress`; raw-mode
  leaks between phases; `--watch` teardown of the alternate screen.
- **Mitigations:** Opt-in first release; ACTTUI-007 PTY matrix and `--watch`
  teardown test are release-cohort blockers; `TerminalGuard` / `eddacraft-tui`
  `lifecycle` panic-restore on every phase; fixture-first contract pinning.

## References

- APS module: [`activation-tui`](../modules/activation-tui.aps.md) (ACTTUI-000
  gates this ADR; ACTTUI-001 first `In Progress` after it lands)
- Fixture spec: [`plans/specs/2026-07-08-activation-tui-contract-fixtures.md`](../specs/2026-07-08-activation-tui-contract-fixtures.md)
- [ADR-092](092-mcp-optional-activation-spine.md) — activation spine semantics
  (frozen; ACTTUI owns presentation only)
- [ADR-047](047-eddacraft-tui-canonical-source-mirror.md),
  [ADR-050](050-eddacraft-tui-runner-and-cli-policy.md),
  [ADR-054](054-json-render-tui-engine-home.md) — `eddacraft-tui` / `anvil-tui`
  ownership
- [ADR-056](056-format-flag-output-selector.md) — output selector precedence
  (`--json` / `--no-tui` compatibility aliases)
- [ADR-060](060-anvil-home-install-root-override.md) — `ANVIL_HOME` gated posture
- Downstream: first-run-wow WOW-005 / WOW-006 design gates (PR #3231);
  [`first-run-wow`](../modules/first-run-wow.aps.md)
- Audit: [`plans/audits/2026-07-04-anvil-start-welcome-user-journey.md`](../audits/2026-07-04-anvil-start-welcome-user-journey.md)
