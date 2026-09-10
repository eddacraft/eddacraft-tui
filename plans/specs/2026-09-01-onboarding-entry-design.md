# Onboarding Entry Design — Tutorial Discovery and Activation Navigation

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Design | Authoritative | JOURNEY | Accepted | Last reviewed 2026-09-10 — JOURNEY-013 re-test closed D1 as fixed ([observation](../audits/2026-09-10-journey-013-activation-navigation.md) on `62e1facd7`). Prior: approved by the operator 2026-09-01 after a grill-design session on a shadowed first-time install |

| Upstream | Downstream |
| -------- | ---------- |
| [`release-user-journeys`](../modules/release-user-journeys.aps.md), [`activation-tui`](../archive/modules/activation-tui.aps.md), [`first-run-wow`](../archive/modules/first-run-wow.aps.md), [`continuous-improvement-backlog`](../modules/continuous-improvement-backlog.aps.md) | [`release-user-journeys`](../modules/release-user-journeys.aps.md) |

## Problem

The operator shadowed a first-time user installing anvil and reported two
things: the tutorials were never run, and moving between the `anvil start`
screens was not intuitive.

Both were investigated against the source before any design was proposed, and
both initial framings turned out to be wrong in ways that changed the design.

### Finding 1 — tutorial discovery is an entry-point gap, not an ordering problem

The tutorial is **already offered prominently** at the two places that exist:

- The post-install banner leads with the ungated command
  (`install.sh:167-168`), a deliberate CIB-288 outcome with a guard test:

  ```text
  anvil welcome    see what Anvil finds in your repo
  anvil start      activate this repo (sign-in required)
  ```

- The first-run onboarding menu offers it **second of three**
  (`crates/anvil-tui/src/surfaces/onboarding/welcome.rs:23-37`): *Set up this
  project* / **Choose a learning path** / *Go to command menu*.

The "tutorial is fifth of seven" position is the **returning-user hub**
(`crates/anvil-tui/src/surfaces/welcome/mod.rs:28-36`), a different surface reached only after
first-run is complete.

The actual gap: **`anvil start` never mentions `anvil welcome` or the tutorial
at all** — `rg 'tutorial|welcome' crates/anvil-cli/src/commands/start.rs`
returns only an unrelated code comment. A first-time user who installs anvil
and runs `anvil start` (the activation path) is never told the tutorial exists,
because the offer lives exclusively on the `anvil welcome` path.

Reordering the hub would not have helped the observed user, who never reached
the hub.

### Finding 2 — the observed navigation problem was measured against a build that no longer exists

At `v0.9.7-beta` — the released build the shadowed user ran —
`consent_help_text` **does not exist**. There was no help bar on the consent
picker at all, and the only help string in that file belongs to the evidence
pane and reads `j/k scroll  g/G top/bottom  esc close  q quit`.

The contextual help bar naming arrows landed in `e586b6e53` (2026-08-21), which
`git tag --contains` places in **no release tag**. On `main` the consent picker
now renders:

```text
↑/↓ move  ←/→ next section  space toggle  a apply  esc/q quit
```

`h`/`j`/`k`/`l` have been live as silent aliases throughout.

So an unknown share of the observed difficulty is already fixed and unreleased.
Designing a splash screen on top of that baseline risks solving a problem that
has already shipped, and a first-run splash is expensive to walk back.

## Decisions

| # | Decision | Rationale |
| - | -------- | --------- |
| D1 | **Close as fixed** — do not design an activation splash or key-hint overlay | JOURNEY-013 re-test on `62e1facd7` (carries `e586b6e53`) shows the contextual help bar on first paint and names the full key model. Residual is not enough to authorise a splash. Record: [`2026-09-10-journey-013-activation-navigation`](../audits/2026-09-10-journey-013-activation-navigation.md) |
| D2 | Tutorial discovery is fixed at the **`anvil start` exit**, not by reordering the hub | The observed user never reached the hub |
| D3 | The pointer is **self-extinguishing on tutorial completion**, not first-run-only and not manually dismissed | Survives being missed once, which is the actual failure mode; needs no new state or dismissal UI |
| D4 | **No** "press N to dismiss" affordance | Retracted by the operator as too ambitious; it would require reading a keypress at the end of a command that currently just exits |
| D5 | No change to the installer banner, the onboarding menu, or the hub ordering | All three are already correct; CIB-288 in particular is a deliberate ungated-first decision with a guard test |

## Approach

On the **interactive TTY path only**, `anvil start` ends by naming
`anvil welcome` as the next step, shown while — and only while —
`~/.anvil/tutorial-progress.json` records no completed path.

That file already exists and is already user-scoped
(`crates/anvil-cli/src/commands/tutorial.rs:400-402`), carrying
`completed_paths` (`crates/anvil-tui/src/surfaces/tutorial/mod.rs:381`). The pointer therefore
**reads existing state and writes none**: it appears for a user who has never
finished a tutorial path, and disappears permanently once they have — with no
config surface, no new persisted state, and nothing for the user to learn.

This is strictly better than a manual dismissal: it extinguishes on evidence
that the user did the thing, rather than on evidence that they dismissed a
message.

## Interfaces and boundaries

- **TTY-only.** `--json`, `--verify`, piped and CI paths carry byte-stable
  single-document contracts (ADR-103) and must be untouched. The pointer is
  additive to the interactive human render only.
- **Non-blocking.** A printed next-step, never a prompt that waits for input.
  `anvil start` must continue to exit deterministically.
- **Read-only against tutorial state.** Activation must not write, migrate, or
  create `tutorial-progress.json`; an unreadable or absent file means "not
  completed" and simply shows the pointer.
- **User-scoped, not repo-scoped.** A user who has done the tutorial once does
  not see the pointer again in a new repository.

## Risks and non-goals

- **Residual:** a user who never intends to run the tutorial has no way to
  silence the pointer short of running it. Accepted for now rather than
  building a config surface for one line of copy. Cheap follow-up if it grates:
  also stop after a small number of activations.
- **Non-goal:** the installer banner, the onboarding menu, the hub ordering,
  any new configuration or persisted state, and the activation-splash question
  (D1, closed as fixed by JOURNEY-013).
- **Watch for:** scope creep from "a printed next step" into an interactive
  prompt. The boundary above is the guard.

## Open questions

None blocking. JOURNEY-013 closed D1 as fixed. Recorded residuals (in-section
↑/↓ wrap, `a` versus Enter, 80-column quit truncation) are optional JSIMP-001
input and do not authorise a splash.
