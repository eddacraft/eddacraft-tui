# WOW-006 Autoplay Demo Design

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Design | Authoritative | first-run-wow | Accepted | Approved by the operator 2026-07-26 after a five-decision design-gate grill; closes the WOW-006 design gate |

| Upstream | Downstream |
| -------- | ---------- |
| [`first-run-wow`](../modules/first-run-wow.aps.md) (WOW-002 reveal driver, WOW-006), [`activation-tui`](../modules/activation-tui.aps.md) (ACTTUI widget vocabulary), [`release-user-journeys`](../modules/release-user-journeys.aps.md) (JOURNEY-007) | [`first-run-wow`](../modules/first-run-wow.aps.md) WOW-006 implementation, [`release-user-journeys`](../modules/release-user-journeys.aps.md) JOURNEY-007 |

## Problem

WOW-006 wants a hands-free "watch anvil work" demo: the tutorial's
`ProtectionLoop` path plays end-to-end unattended — commands, inline-editor
ghost-typing, verification — so a new user or a presenter can watch Anvil work
for real without touching their own repository, and any keypress hands control
back to the normal interactive tutorial. The intent was clear; the executable
shape was not. This design closes the WOW-006 design gate: sandbox lifecycle,
mutating-command policy, entry point, watch-demo transition, and the pacing
mechanism.

The demo must never be a substitute for real repository value (JOURNEY's
honesty contract): it is explicitly labelled, isolated, and offered for clean
repositories and demonstrations — not presented as findings from the user's own
code.

## Decisions

### D1 — Sandbox lifecycle: fresh OS tempdir, RAII cleanup, offline

Each run scaffolds a **pinned, deterministic fixture** into a **new OS
tempdir** (reusing the established `scaffold_project` / tempdir-fixture
patterns). The fixture content is committed to the binary so findings are
**identical every run** and require **no network**. The tempdir is removed on
exit via RAII (`Drop`). The demo never touches the user's repository or
`ANVIL_HOME`.

Rationale: determinism is required for a demo (the same beats every time);
isolation is the whole point; RAII cleanup avoids leaving state on disk or
needing a bespoke reset path.

### D2 — Mutating-command policy: execute for real in-sandbox, with a containment guard

All steps — including file writes and git — **execute for real** against the
tempdir fixture; that authenticity is what makes it "watch Anvil *work*" rather
than a scripted animation. A **path-containment guard** canonicalizes each
step's target path and **hard-aborts** the demo if it resolves outside the
sandbox root. By construction nothing can touch the user's environment.

Rationale: the sandbox is disposable, so real execution is safe and honest;
the guard is defense-in-depth against a fixture or step definition that ever
references an absolute or `..`-escaping path.

### D3 — Entry point: explicit flag, plus a discovery row; never auto-fires

The canonical entry is an explicit **`anvil tutorial --autoplay`** flag
(deterministic and scriptable, honouring the ACTTUI-000 public scripting
contract). For discovery, the tutorial picker gains a **"Watch anvil work
(demo)"** row. The demo **never** auto-triggers unprompted — a clean repo does
not silently start playing an animation.

Rationale: a flag keeps it scriptable and testable; the picker row makes it
discoverable without cluttering the first-run welcome hub; no-surprise start
respects the honesty posture.

### D4 — State and transitions: session-scoped autoplay; keypress-anywhere exits

Autoplay state lives on the **top-level tutorial session**, not per-surface, so
it **survives the watch-demo surface transition** and keeps driving — preserving
the "watch catches a save" beat, the most compelling moment of the demo. **Any
keypress on any surface** converts the session to the normal interactive
tutorial (the WOW-006 hands-back invariant).

Rationale: per-surface autoplay would reset at the watch overlay and drop the
key beat; session-scoped state is a single source of truth for the mode.

### D5 — Pacing: extend the WOW-002 reveal driver; no bespoke autoplay clock

Autoplay reuses the existing WOW-002 `reveal_tick` driver (`CommandReveal`,
`REVEAL_CHARS_PER_TICK`): ghost-type the command through the existing driver,
then — instead of waiting for `Enter` — **dwell a beat and auto-advance** to the
next step. One pacing mechanism is shared between the demo and the interactive
tutorial; there is **no bespoke autoplay chrome** (the WOW-006 gate cautioned
against a second pacing system that would drift from the interactive path).

Rationale: a single pacing source keeps demo and interactive rendering
consistent and avoids two timers to synchronise.

## Interfaces and boundaries

- New `--autoplay` flag on the tutorial command (clap); machine/plain paths
  stay deterministic.
- An autoplay flag on `TutorialState` / the tutorial session (session-scoped
  per D4).
- Reuse: `CommandReveal` / `reveal_tick` (D5), `TutorialPath::ProtectionLoop`,
  `WatchDemoState`, and the ACTTUI widget vocabulary (`ParallelProgress`,
  `OverlayStack`, `Toast`, `BigBanner`, shared `HelpBar`) — no bespoke demo
  chrome.
- New: a deterministic fixture scaffolder (pinned content) + RAII sandbox
  handle with the D2 containment guard.

## Validation (to pin at build via TDD)

- The scaffolded fixture yields an **identical** finding set on repeated runs
  (determinism), with no network access.
- Autoplay runs the `ProtectionLoop` path unattended **to completion** in the
  sandbox.
- A keypress on each surface (intro, step, watch-demo overlay, verdict)
  converts the session to the interactive tutorial.
- The path-containment guard **rejects** a step whose target canonicalizes
  outside the sandbox root.
- The sandbox tempdir is **removed** on exit (including on interrupt).

## Risks and non-goals

- **Non-goal:** the demo is not a substitute for the real first-run value path;
  it is labelled and isolated, offered for clean repos and demonstrations.
- **Non-goal:** no network, no persistence, no execution outside the sandbox.
- **Risk:** fixture drift from real check behaviour — mitigated by running the
  real `ProtectionLoop` checks against the fixture rather than hard-coding
  outputs.
