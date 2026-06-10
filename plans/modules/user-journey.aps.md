<!--
APS Module: User Journey — Two Golden Paths
===========================================
Make install/upgrade → first value fast and unmistakable. Created 2026-06-10
from the v0.8.0-beta user-journey completeness review (operator-directed).
-->

# User Journey — Two Golden Paths

| ID  | Owner  | Status | Progress |
| --- | ------ | ------ | -------- |
| UJ  | @aneki | In Progress | 7/11 |

**Last reviewed:** 2026-06-10 (created from the v0.8.0-beta user-journey
completeness review. Operator direction: beta posture permits explicit guidance
— tell every installer/upgrader to run `anvil start` or `anvil welcome` — and
both paths must be strong. Out-of-the-box usefulness ranks above tutorial-led
onboarding; the tutorials overhaul (UJ-011) is deliberately last.)

## Purpose

Every user who installs or upgrades Anvil should reach something genuinely
useful fast — and keep finding it useful day in, day out. Today the flagship
save-time daemon is silent unless the user already knows `anvil start`, the
entry docs trail the product by two releases, and an upgrade is invisible.

This module makes two explicit beta golden paths strong and self-guiding:

1. **The welcome path** (`anvil welcome`) — discovery wow: see what Anvil finds
   in your own repo within minutes, landing on a populated surface.
2. **The start path** (`anvil start` → `anvil watch` / MCP) — daily value:
   daemon-backed save-time validation that announces itself and explains
   itself.

Beta posture: we can guide users explicitly. The install/upgrade message is
"run `anvil start` or `anvil welcome`" — every surface on each path must then
carry the user to the next step without a docs lookup.

## In Scope

- Next-step threading in CLI/install output along both golden paths
- Save-time posture visibility (`anvil status`, watch help/advisories)
- Entry documentation: quickstart, beta-testing guide, a consolidated
  save-time validation guide
- Upgrade experience: gate-summary dashboard reach, post-upgrade what's-new
- Design decisions: auth-wall placement, daemon offer-to-start
- Tutorials overhaul (shaping only; execution items filed after shaping)

## Out of Scope

- The daemon routing/rollout mechanics themselves (DSV-021, ADR-075 — landed)
- The first-week insights nudge mechanism (INSIGHTS owns it; INSIGHTS-005
  extends it to `welcome`)
- The watch TUI daemon-fallback indicator (CIB-047)
- Pre-tag changelog wording and release-plan reconciliation (CIB-054 /
  CIB-055)
- Telemetry or any cloud onboarding surface

## Interfaces

- **Depends on:**
  - `crates/anvil-cli/src/commands/` (welcome, init, start, watch, status,
    dashboard)
  - `crates/anvil-cli/src/commands/watch_save_time.rs` (DSV-021 routing modes)
  - `crates/anvil-cli/src/feature_flags.rs` (`CLI_GATED_COMMANDS`,
    `cli.licence-gate`)
  - `docs/public/anvil/` (quickstart, guides, beta-testing guide)
  - `install.sh`
- **Coordinates with:** DSV (daemon save-time arc), INSIGHTS (first-week
  nudge), DISTRIB-002 (update advisory surface), TUIDASH-013 / CIB-053
  (gate-summary spec single-source), CIB-047/-054/-055.

## Work Items

### UJ-001: Golden-path next-step threading in CLI and install output

- **Status:** Merged 2026-06-10 via PR #2502
- **Intent:** A new user should never need docs to know what to type next;
  every onboarding-path command names the single next step.
- **Expected Outcome:** `install.sh` closing output and the endings of
  `anvil welcome`, `anvil init`, and `anvil start` each print one next-step
  line that carries the user along the two beta paths (`anvil welcome` for
  discovery; `anvil start` then `anvil watch` for daily value). Neither path
  dead-ends; the hints are single lines, not banners.
- **Files:** `install.sh`,
  `crates/anvil-cli/src/commands/{welcome,init,start}.rs` (best-effort)
- **Validation:** `cargo test -p eddacraft-anvil` closing-hint assertions for
  each command; manual transcript of both paths from a fresh repo.
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "Install and onboarding commands now point at the next step, so
    install → first value needs no docs lookup."

### UJ-002: Welcome path lands on a populated surface

- **Status:** Done 2026-06-10 (verified — no change needed)
- **Disposition:** Code-trace verification found the feared failure mode does
  not exist: every welcome hub option gathers live data at launch
  (`collect_gate_data()` runs the checks; audit/doctor collect fresh), and the
  first-run flow lands on the tutorial populated by a real discovery scan. No
  welcome surface recommends a persisted-data surface (e.g. the gate-summary
  saved spec). Manual TUI run blocked by the beta licence gate in the agent
  environment; evidence is the code trace (`crates/anvil-cli/src/commands/welcome.rs`
  hub dispatch + `gate.rs::collect_gate_data`).
- **Intent:** The first visual surface the welcome path recommends must show
  real findings from the user's own repo — an empty first view kills the wow.
- **Expected Outcome:** After `anvil welcome` on a non-trivial repo, the
  recommended next surface renders populated data from the discovery scan. A
  surface that needs data the user has not generated yet (e.g. gate-summary
  needs `anvil gate` runs in `.anvil/gates.json`) is never the recommended
  first target.
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`,
  `crates/anvil-cli/src/commands/dashboard/` (best-effort)
- **Validation:** integration test: welcome on a fixture repo → recommended
  surface renders non-empty; manual run on a real repo.
- **Dependencies:** UJ-001 (the hint must point at the surface this item
  selects)
- **Confidence:** medium

### UJ-003: Quickstart and beta guide rewritten around the two paths

- **Status:** Merged 2026-06-10 via PR #2503
- **Intent:** Entry docs sell and guide the two-path journey and stop trailing
  the product by two releases.
- **Expected Outcome:** The quickstart leads with install then "run
  `anvil start` (daily save-time protection) or `anvil welcome` (see what
  Anvil finds)"; prerequisites include Rust projects (v0.8.0 RSTLAN);
  `ANVIL_HOME` is noted in the install section; MCP/agent integration
  (`anvil start` wiring + `anvil mcp-config`; the item originally said
  `anvil mcp-install`, which does not exist) is a numbered step, not an
  appendix; stale
  `v0.7.2-beta` version pins are refreshed. The beta-testing guide reflects
  the v0.8.0 feature set.
- **Files:** `docs/public/anvil/quickstart.md`,
  `docs/public/anvil/beta-testing-guide.md`
- **Validation:** `pnpm docs:check` green; both paths walked end-to-end
  following only the rewritten quickstart.
- **Confidence:** high
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** patch

### UJ-004: Auth-wall placement vs first wow

- **Status:** Proposed
- **Intent:** Decide where the beta licence gate sits relative to first value:
  today `welcome`, `check`, `status`, `init`, and `watch` are all gated
  (`CLI_GATED_COMMANDS`), so the first command any new user runs is a login
  prompt before Anvil has shown anything.
- **Expected Outcome:** An ADR decides gate placement — for example, ungated
  read-only `anvil welcome` as the demo with durable surfaces (`init`,
  `start`, `watch`) still gated, or an affirmed gate-first posture with the
  smoothest possible interactive login — and `CLI_GATED_COMMANDS` reflects the
  decision. The decision is recorded in `plans/decisions/DECISION-LOG.md`.
- **Files:** `plans/decisions/` (new ADR),
  `crates/anvil-cli/src/feature_flags.rs`
- **Validation:** ADR Accepted; gated-command list matches the ADR; a fresh
  unauthenticated user's first-command transcript matches the decided posture.
- **Dependencies:** design-gated — needs a new ADR; coordinates with the
  intentional beta `cli.licence-gate` posture ("revisit at GA").
- **Confidence:** medium

### UJ-005: `anvil status` always states the save-time posture

- **Status:** Merged 2026-06-10 via PR #2500
- **Intent:** Status is the home screen; it must state the save-time posture
  even when the daemon is off, instead of omitting the line and hiding the
  flagship gap.
- **Expected Outcome:** With default routing and no live daemon, `anvil
  status` shows an explicit off-state save-time line that names `anvil start`
  (today the line is omitted to preserve the pre-DSV surface — this item
  deliberately revisits that choice under the beta guide-users posture).
  Live/forced/opt-out states keep their existing renderings; `--json` stays
  additive.
- **Files:** `crates/anvil-cli/src/commands/status.rs`
- **Validation:** `cargo test -p eddacraft-anvil commands::status` covering
  the four routing-mode × daemon-presence cases.
- **Dependencies:** DSV-021 (routing modes landed)
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "`anvil status` now always states the save-time posture, including
    how to enable daemon-backed validation when it is off."

### UJ-006: Daemon guidance on the watch surface and help

- **Status:** Merged 2026-06-10 via PR #2501
- **Intent:** A user must be able to learn from the CLI itself that save-time
  validation is daemon-served and how to enable it.
- **Expected Outcome:** `anvil watch --help` long help names the daemon, the
  `anvil start` prerequisite for daemon-backed validation, and the
  `ANVIL_WATCH_DAEMON` values (unset/0/1); the plain-surface daemon-absent
  fallback advisory names `anvil start`. The TUI indicator remains CIB-047.
- **Files:** `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-cli/src/commands/watch_save_time.rs`
- **Validation:** help-text assertion test; fallback-advisory unit test
  includes the `anvil start` pointer.
- **Dependencies:** coordinates with CIB-047 (TUI surface) and CIB-054
  (changelog wording)
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### UJ-007: Watch offers to start the daemon when absent

- **Status:** Proposed
- **Intent:** Close the last gap between "default-on routing" and "every user
  actually daemon-backed": watch can offer to start the daemon instead of
  silently falling back.
- **Expected Outcome:** A decision (extending the ADR-075 rollout controls)
  on offer-to-start vs auto-start vs guidance-only; if adopted, `anvil watch`
  on a TTY with no live daemon offers a one-time "start the daemon now?"
  prompt, headless behaviour is env/flag-controlled, and
  `ANVIL_WATCH_DAEMON=0` is always honoured.
- **Files:** `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-cli/src/commands/watch_save_time.rs`
- **Validation:** routing-mode tests cover prompt accept/decline/headless;
  opt-out never prompts.
- **Dependencies:** design-gated — extends ADR-075 rollout posture; UJ-001
  beta messaging ("run `anvil start`") may make guidance-only sufficient,
  which is itself an acceptable outcome of the decision.
- **Confidence:** medium

### UJ-008: Consolidated save-time validation guide

- **Status:** Merged 2026-06-10 via PR #2504
- **Intent:** The daily-driver value proposition deserves one page; today the
  daemon/save-time story is split across config.md, mcp.md, and
  agent-harness.md.
- **Expected Outcome:** A single public guide covers what is watched, the
  daemon's role, assurance states, workspace confinement, fallback behaviour,
  and `ANVIL_WATCH_DAEMON`; the existing pages cross-link to it rather than
  duplicating.
- **Files:** `docs/public/anvil/guides/` (new page),
  `docs/public/anvil/operations/config.md`,
  `docs/public/anvil/integrations/mcp.md` (cross-link edits)
- **Validation:** `pnpm docs:check` and `pnpm docs:index` green; new page
  carries the governance and Upstream/Downstream tables.
- **Confidence:** high
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** patch

### UJ-009: Gate-summary dashboard reaches existing projects

- **Status:** Merged 2026-06-10 via PR #2505
- **Intent:** Upgraders get the v0.8.0 gate-summary dashboard — today only
  fresh `anvil init` runs seed it, so existing projects never see it.
- **Expected Outcome:** Existing projects gain the gate-summary dashboard via
  a defined path (native catalogue entry, or seeding through
  `anvil migrate schema` / doctor) so `anvil dashboard` lists it after an
  upgrade without re-init. The embedded spec remains the single source
  (CIB-053 disposition).
- **Files:** `crates/anvil-cli/src/commands/dashboard/`,
  `crates/anvil-cli/src/commands/migrate.rs` (best-effort)
- **Validation:** test: project without `.anvil/dashboards/` gains a listed
  gate-summary entry after the chosen path runs; no clobbering of a
  user-customised saved spec.
- **Dependencies:** TUIDASH-013 (shipped), CIB-053 (spec single-source)
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "Existing projects now get the gate-summary dashboard on upgrade,
    not only on fresh `anvil init`."

### UJ-010: Post-upgrade what's-new one-liner

- **Status:** Ready
- **Intent:** An upgrade should announce its headline once — today new
  capability is invisible and the daemon stays cold (compounds UJ-006/-007).
- **Expected Outcome:** The first run after a version change prints one line
  (release headline + changelog pointer) exactly once, then never again for
  that version; suppressible; complements DISTRIB-002's update-available
  advisory (which covers the opposite direction).
- **Files:** `crates/anvil-cli/src/commands/status.rs` or shared banner
  surface; version-change marker under `.anvil/` (best-effort)
- **Validation:** test: version-change marker → hint exactly once; no hint on
  same-version runs; `--json` output unaffected.
- **Dependencies:** coordinates with DISTRIB-002
- **Confidence:** medium
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch

### UJ-011: Tutorials overhaul

- **Status:** Proposed
- **Intent:** Tutorials should be something every user actually uses; today
  they trail the product (no Rust analysis tutorial) and do not reflect the
  two-path journey. Operator direction: out-of-the-box usefulness ranks
  first, so this shapes after the path work.
- **Expected Outcome:** A shaped plan for the tutorial set: audit of current
  tutorials, a journey-aligned target set (including Rust project analysis),
  retirement list for stale ones, and follow-up execution items filed in this
  module or a successor.
- **Files:** `docs/public/anvil/tutorials/` (audit scope)
- **Validation:** shaping output reviewed by the operator; follow-up items
  filed.
- **Dependencies:** UJ-003 (quickstart defines the journey the tutorials
  deepen)
- **Confidence:** medium

## Sequencing

1. **UJ-003** (entry docs) and **UJ-005/-006** (posture visibility) are
   independent and front-load the beta guidance message.
2. **UJ-001 → UJ-002** thread the welcome path; UJ-002 needs UJ-001's hint
   target decided.
3. **UJ-009/-010** (upgrader reach) are independent of the above.
4. **UJ-004** and **UJ-007** are design-gated decisions; either may resolve
   to "current posture affirmed".
5. **UJ-008** can land any time; **UJ-011** shapes last.

## Release Notes

UJ items collectively justify a "install or upgrade, then run `anvil start`
or `anvil welcome` — both paths now carry you to value" line in the release
that ships them.

## Cross-References

- Source review: 2026-06-10 v0.8.0-beta user-journey completeness review
  (operator session; actions A1–E18).
- Coordinates with:
  [`CIB-047`](./continuous-improvement-backlog.aps.md) (watch TUI fallback
  indicator), [`CIB-054`](./continuous-improvement-backlog.aps.md) /
  [`CIB-055`](./continuous-improvement-backlog.aps.md) (pre-tag wording +
  release-plan reconcile), [`INSIGHTS-005`](./usage-insights.aps.md)
  (first-week nudge on welcome),
  [`DISTRIB-002`](../archive/modules/distribution-and-update.aps.md)
  (update advisory), [`DSV-021`](./daemon-save-time-validation.aps.md)
  (routing modes), ADR-075 (rollout controls).
