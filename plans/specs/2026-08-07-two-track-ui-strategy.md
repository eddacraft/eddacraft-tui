<!-- Strategy doc: relationship and sequencing between the converged-app and ultimate-ui initiatives -->

# Two-Track UI Strategy: Converged App and Ultimate UI

Date: 2026-08-07
Status: Direction agreed; ADRs pending
Sources:
[`plans/specs/converged-app/`](converged-app/README.md) ·
[`plans/specs/anvil-ultimate-ui/`](anvil-ultimate-ui/00-index.md)

## Verdict

The two packs were written within a week of each other and are **not mutually
exclusive**. They solve different layers of the same conviction:

- **Converged app** is a *product architecture*: converge APS and anvil into
  one app layer — daemon as local control plane, command bus, projections,
  capability-composed shell, native/web/tray/CLI/TUI surfaces.
- **Ultimate UI** is a *framework and category bet*: a semantic Rust
  application runtime (entities, typed commands, Flow/Scene, renderer
  independence) whose first expression is the terminal.

Both independently arrived at the same architectural spine — typed commands as
the only mutation path, UI as projections of authoritative state, stable
durable identity, agents and accessibility consuming the same typed surface,
framework-neutral domain crates. That spine is the shared asset. The Dioxus
spike in the converged-app pack decides the **desktop client technology only**
and forecloses nothing for ultimate UI, whose own roadmap defers native
framework selection until after its terminal and web proofs.

## Strategy

Build both, **asymmetrically**:

| Track | Role | Cadence |
| --- | --- | --- |
| Converged app | Main line. Needed now. Unapologetically self-serving. | Product roadmap, phased per its migration plan |
| Ultimate UI | Gated research track. | Spike-by-spike, reviewed only at its own gates |

If forced to choose, converged app wins without hesitation. The structure
below exists so we are not forced to choose: the cheap early phases of
ultimate UI harden the shared command spine even in the world where the
framework never ships.

## Repository placement

- **Converged app lives in the anvil monorepo from day one.** It is not a
  separate application; it *is* anvil's app layer, restructuring crates that
  already live here (daemon, governance, evidence, plan read model). Building
  it elsewhere would manufacture the "second convergence effort" its own
  requirements doc warns against.
- **Ultimate UI lives in its own repository:**
  [`eddacraft/allomorph`](https://github.com/eddacraft/allomorph) (private).
  "allomorph" is the working runtime name, part of the internal umbrella
  programme "Project Ubiquity"; the Phase 0 naming brief confirms or revises
  it.
  The repo boundary *is* the guardrail: its core rule (reference app imports
  no anvil code; anvil must not become the design template) is enforced
  structurally. It also escapes monorepo ceremony (APS gates, docs surfaces,
  release discipline) that an incubation spike should not pay. Its endgame
  never requires moving in — on success anvil consumes it as a dependency,
  the way we consume Ratatui.
- **Framework spikes (Dioxus vs Tauri/React) are disposable** — run them in a
  scratch repo. Only the decision and the eventual product shell enter the
  monorepo. Do not let spike debris seed the real app by inertia.

Monorepo overhead is managed by partitioning, not separation:

- Path-gate all new app CI lanes (existing playbook: merge-gate exclusions,
  path-gated E2E and docs lint).
- Keep `apps/anvil-desktop` in its **own cargo workspace and lockfile** until
  the framework decision settles — a webview/JS dependency tree must not bloat
  `cargo build --workspace` or churn workspace-hack.

## Release gating: environment feature flag

The converged app surface must not reach users until it is ready, while its
code merges to trunk continuously (trunk + flags is the established release
model; precedent: `dashboard.web`).

- **Umbrella flag:** register `app.converged` in `flags/manifest.json` when
  the first shell code lands. Class `rollout`, boolean, `defaultVariant:
  disabled`, with an expiry/review date.
- **Environment opt-in:** `ANVIL_APP_CONVERGED=1` (and `ANVIL_DEV=1`) enables
  the surface locally for development and dogfooding. Default-off in every
  release build until the readiness criteria below are met.
- **Staged sub-flags** (`app_shell_v1`, `operational_inbox_v1`, `aps_board_v1`
  …) follow the converged-app migration plan §18 once modules exist. The
  umbrella flag gates release exposure; sub-flags gate rollout within it.
  Capabilities still govern what an installation/actor may do — flags are not
  an authorisation mechanism.

Readiness to flip default-on is a deliberate release decision, not a flag
expiry: first vertical slice complete, daemon-owned work surviving client
closure, standalone APS parity tests green, and a live UX review (the
`dashboard.web` lesson: foundations-ready ≠ release-default).

## Coupling contract (closed list)

"Build converged app with thought toward ultimate UI" means **exactly** these
commitments — each cheap now and valuable even if ultimate UI dies:

1. One command/event/run envelope design, authored once (converged ADR-B is
   the venue; reviewed against the ultimate-UI command model).
2. Stable durable IDs on every domain object.
3. Clients consume projections and issue commands; they never own domain
   logic.
4. One semantic status vocabulary across surfaces.
5. Framework isolation: domain and application crates never depend on a UI
   framework (already mandated by DS-005).

Everything beyond the list stays out: no Flow/Scene concepts, no renderer
contracts, no promotion/collapse seams, no placeholder traits "for later" in
converged-app code. Speculative generality is the named failure mode.

## Rules of engagement

- **Converged app never blocks on, or imports from, ultimate UI** until the
  framework passes its Gate E (renderer independence).
- **Ultimate UI never adopts the anvil daemon as its backend** during
  incubation. Its reference app stays self-contained; one adapter proof
  against the daemon comes later as a deliberate migration experiment.
- **Keep the converged-app TUI deliberately thin** (status, board, approvals
  over projections). A rich conventional TUI is exactly the artefact ultimate
  UI would replace — do not gold-plate it.
- **Sequence ultimate UI so its earliest spikes feed the shared spine**:
  EXP-001 (entity ergonomics) and especially EXP-002 (one typed command
  projected to plain output, JSONL, inline flow, workspace, agent schema)
  directly inform the converged command envelope.
- **Gates are kill criteria, not milestones.** Gate B (Flow/Scene promotion
  feels materially better than screen navigation) is the thesis test. Failing
  it parks the project as research that improved our command model; converged
  app loses nothing.

## Decisions still owed (not made by this doc)

- **Canonical APS source moves into the monorepo** (converged-app Phase 0–1):
  a separate ADR before any source-authority change — it affects a published
  product and the public mirror model. The app-layer work does not depend on
  it starting.
- Converged-app ADR candidates A–J (command envelope, daemon transport,
  document mutation, workspace isolation, native framework, capability model,
  evidence retention, shared-UI strategy) enter
  [`plans/decisions/DECISION-LOG.md`](../decisions/DECISION-LOG.md) via the
  normal ADR process as each is decided.
- Ultimate UI's charter, ADR backlog, and decision log belong to its own
  repository, not this one.
- New OSS surface for ultimate UI (if it goes public) requires ADR + legal
  review per the established IP boundary.
