# Council review — ACTTUI merged work + `anvil start` / `anvil welcome` first-run journeys

- **Session:** `council-d4c804e6` (batch, full pack — council-reviewer, kernel-maintainer, adversarial-reviewer, operations-reviewer, pragmatic-lead)
- **Target:** merged ACTTUI commit range `3f9a11e1b..96087ae1d` (origin/main), scoped to ACTTUI-touched files, plus an end-to-end journey trace of both first-run surfaces
- **Date:** 2026-07-09
- **Verdict:** **BLOCK the TTY-default flip; the `--tui` opt-in path is currently a first-run dead end.** Plain `anvil start` and `anvil welcome` are in good shape.

## Journey verdicts (unanimous)

| Journey | Verdict | Why |
| ------- | ------- | --- |
| `anvil start` (plain / CI / piped / `--verify` / `--json`) | **Good** | Deterministic degrade verified by live run (exit 0, no escape codes, no hang, even with `ANVIL_ACTIVATION_TUI=1` on piped stdio). CIB-163/164/166 fixes confirmed in copy. |
| `anvil start --tui` (opt-in) | **Broken** | Consent phase is unreachable; MCP + workflow install permanently no-op. See C-001/C-002. |
| `anvil welcome` | **Good** | Shared Select/HelpBar/Spinner chrome is real; compact-mode picker scaling (CIB-179) and config-file-name honesty (CIB-171) confirmed closed; copy clean, UK-spelled, jargon-free. |

## The ship-blocker (confirmed independently by all five reviewers)

`crates/anvil-cli/src/commands/start.rs` never constructs a `ConsentState` and never calls
`ActivationSurface::with_consent(...)` (zero production call sites at HEAD). The orchestrator
correctly defers MCP and GitHub Actions workflow writes under `InstallConsentMode::DeferToTui`
(`install.rs:273` forces `chosen_ids = Vec::new()`), and `start.rs:422` discards the surface's
returned state (`let _state = ...`), so no code ever reads the user's ticks back.

Net effect on a fresh repo with `--tui` / `ANVIL_ACTIVATION_TUI=1`:

- The phase strip bolds **Consent** while the body silently renders the **verdict tree**
  (`render.rs:46-52` falls through when `consent` is `None`); keys route to verdict navigation.
- MCP config and workflows are **never installed, on any run, with no way to complete consent** —
  the surface reports "skipped — consent deferred to activation TUI consent surface" forever.
- This is a **silent skip, not a consent bypass** (adversarial reviewer traced and smoke-tested:
  nothing is written without consent) — but the flagship path is strictly worse than plain.
- This is the #3238 Copilot finding, still open across four follow-up commits. The module doc
  honestly records it as ACTTUI-004 remaining scope; the fully unit-tested consent chrome is
  dead code from the CLI's perspective, and no test exercises the `run()` wiring (which is why
  it shipped unseen).

**Fix shape (small, well-understood):** build `ConsentState` from `pending_workflows(root)` +
the MCP `Candidate` list before `run_surface`, capture the returned state, and drive ticked
selections through `install_for_clients` / `ensure_github_actions_workflows`. Add one positive
integration test (tick ⇒ file write) — the existing test only proves the negative.

## Findings (18 recorded in session `council-d4c804e6`)

| ID | Sev | Where | Summary |
| -- | --- | ----- | ------- |
| C-001 | critical | `start.rs:408` | Consent picker unreachable — `with_consent` never called |
| C-002 | critical | `start.rs:422` | Surface state discarded; `DeferToTui ⇒ Vec::new()` ⇒ install permanently no-op |
| C-003 | major | `activation/render.rs:46` | Phase strip shows Consent over verdict body; no invariant guard |
| C-004 | major | `activation/verdict.rs:140` | Verdict tree fed only by substring-parsing composed human copy; typed `with_verdict_model` unused; copy edits silently misfile rows |
| C-005 | major | `start.rs:1231` | No test of the full run()→surface wiring path (root cause of C-001 shipping) |
| C-006 | major | `tests/fixtures/start-activation/` | Fixture home is README-only — byte-stability of `--verify`/`--json`/plain not actually pinned |
| C-007 | major | `activation.e2e.test.ts:143` | No automated coverage of implicit non-TTY degrade with opt-in flag set (manually verified correct today) |
| C-008 | major | `start.rs:914` | Prior-audit re-run verbosity gap still open — full recipe printed every run (pre-existing, cheap fix, CIB scope) |
| C-009 | minor | `install.rs:687` | Live plain picker still **pre-ticks** MCP candidates — CIB-165 honoured only in the unreachable TUI path (pre-existing) |
| C-010 | minor | `log_panel.rs:52` | Indentation-heuristic parsing of `render_human_verbose` — fragile against reflow |
| C-011 | minor | `activation/render.rs:193` | No snapshot tests; Preflight/Working/Done phases never rendered in any test |
| C-012 | minor | `activation/render.rs:251` | CIB-182 copy change landed inside byte-stable-pinned repair-hint text — confirm sanctioned exception |
| C-013 | minor | `start.rs:417` | In-surface tier evidence only if `--why` passed upfront; no on-demand key |
| C-014 | minor | `activation/mod.rs:400` | Help bar `q quit` vs welcome `esc/q quit` — inconsistent exit-key story |
| C-015 | minor | `activation/mod.rs:362` | `consent_mut()` dead code |
| C-016 | minor | `activation-tui.aps.md:213` | Keep ACTTUI-004 In Progress; never narrate consent as shipped |
| C-017 | nit | `start.rs:941` | `ANVIL_NO_TUI=` (empty) doesn't opt out, unlike sibling presence-only env hatches |
| C-018 | nit | `anvil-tui/Cargo.toml:16` | `big-text` dep enabled; `celebrate()` has zero production call sites pending WOW-005/006 |

## Recommended order of work

1. **Now (blocks any release cohort that advertises `--tui`):** C-001 + C-002 + C-005 (the wiring plus its regression test), and C-003's cheap invariant guard while in there.
2. **Before the TTY-default flip (folds into ACTTUI-007):** C-006, C-007, C-011 — the contract matrix ACTTUI-007 already owns; plus decide C-012's exception status.
3. **High-value cheap wins, independent of ACTTUI:** C-008 (quiet re-run — the single biggest remaining "best foot forward" gap on the plain path) and C-009 (untick the live picker — it contradicts the module's own consent principle on the path users actually hit today).
4. **With the consent wiring or opportunistically:** C-004, C-010 (move TUI models onto typed data instead of re-parsing human copy), C-013, C-014, C-015, C-017, C-018.

## What's genuinely good

The widget layer itself is clean and well-tested in isolation; eddacraft-tui prelude widgets are
used throughout with no hand-rolled substitutes (module constraint honoured). Terminal safety
(TerminalGuard, panic hook, Ctrl-C) is solid. TTY-eligibility gating is comprehensive across the
flag/env matrix. Welcome convergence is real, consistent, and closed its audit gaps. Copy quality
is high — UK spelling, no internal jargon leaks found in user-facing strings or `--help`.
