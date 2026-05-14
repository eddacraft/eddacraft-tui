# Release Plan Proposal — `v0.7.0-beta` "Let's Use This"

**Status:** **Accepted (2026-05-14).** Reframe landed in the live
[`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) and
[`plans/index.aps.md`](../../plans/index.aps.md); the four new modules
(ADTRUST, ADOPT, DISTRIB, INSIGHTS) are promoted to `Ready`. This file
remains as the **acceptance record** — the audit trail for why the cut
shape changed and what was committed to alongside the change. Open
items are listed in the Acceptance Checklist below.

**Authoring context:** Conversation 2026-05-14 reframed the next release away
from "daemon working end-to-end" (a capability claim) and towards "let's use
this" (a retention claim). The product surface is largely in place; the gap is
the friction that turns "wow, when's it ready?" into "I've stopped using it."

| Type             | Authority | Owner       | Status                | Freshness                            |
| ---------------- | --------- | ----------- | --------------------- | ------------------------------------ |
| Release proposal | Derived   | APS modules | Accepted (2026-05-14) | Live plan reshaped; 2 follow-ups open |

---

## Why The Plan Changes Shape

The current `RELEASE-PLAN.md` defines `v0.7.0-beta` as the daemon-working slate:
MLP backbone, INTL launcher, WATCHUX hotfix, documentation. As of 2026-05-13 MLP
is effectively complete (17/17 shipped; MLP-018 is a closeout backlog of v1
deferrals). INTL Wave 3 is queued. WATCHUX hotfix is in flight.

Capability-wise that is sufficient to ship. **Retention-wise it is not.** The
release we *sit on* needs four things the current plan does not name as work
items:

1. **A protection claim that is trustworthy during normal use** — MLP-009 proves
   the claim is well-formed in fixtures. It does not prove it is legible to a
   user on a Tuesday afternoon when the daemon has been up for six hours and
   something silently degraded.

2. **Zero-surprise integration with existing developer environments** — hook
   coexistence with lefthook/husky/pre-commit-framework, AI tool auto-detect,
   resource budget guarantees, clean uninstall. Beta feedback already showed
   these are first-week killers (see WATCHUX origin story).

3. **A trivial path from "bug" to "fix on my laptop"** — `anvil update` exists
   but the surrounding ecosystem (Homebrew formula maintenance, release cadence
   policy, version-check UX, security-advisory surfacing) is what determines
   whether senior users get hotfix value or get stuck on the version where their
   bug lives.

4. **A periodic value signal during the silent middle** — when Anvil works
   correctly it is mostly invisible. Users who do not see periodic "Anvil
   caught X this week" output question the value. `anvil insights` closes the
   silent-middle gap.

This proposal adds four new APS modules — **ADTRUST**, **ADOPT**, **DISTRIB**,
**INSIGHTS** — covering items 1–4. It also adds a Wave 5 *Boring Week* gate to
the cut criteria: before the tag ships, the team uses Anvil on their own real
work for one full week under a fresh-user configuration. Any disabled check,
unresolved suppression, or hook bypass during that week is a cut blocker.

---

## Cut Claim For `v0.7.0-beta`

> **Anvil is ready to live on a senior engineer's machine for a month without
> being uninstalled.**

This is a strictly stronger claim than "daemon working end-to-end" and is
explicitly what the existing roadmap principle "first-touch wow … the first
minute matters more than the next ten" implies once you extend it past the
first minute.

| Anti-claim                                      | Why we do not make it                                                                  |
| ----------------------------------------------- | -------------------------------------------------------------------------------------- |
| "Anvil is production-ready"                     | Beta. We want users; we do not want to over-state maturity.                            |
| "Anvil protects every architecture violation"   | Honest-claim principle. The protection claim is closed-set and partial by design.      |
| "Anvil replaces code review"                    | Vision text deliberately positions Anvil as augmentation, not replacement.             |
| "Zero false positives"                          | Untestable. The signal-quality metric is `<10% warnings suppressed without resolution`. |

---

## Current State (2026-05-14)

| Module    | Status                          | Progress | Role for `v0.7.0-beta`                                                          |
| --------- | ------------------------------- | -------- | ------------------------------------------------------------------------------- |
| MLP       | In Progress                     | 17/18    | Protection backbone. MLP-018 closeout backlog is the only remaining work.       |
| INTL      | Ready (ready-to-start-Wave-3)   | 0/9      | Wrapped-launch ingress (`anvil-run`).                                           |
| WATCHUX   | In Progress                     | 0/8      | First-run watch UX. WATCHUX-001..-004 in flight, -005..-008 sequenced.          |
| ADTRUST     | **Proposed (new)**              | 0/6      | Make the protection claim legible and verifiable during sustained daily use.    |
| ADOPT     | **Proposed (new)**              | 0/6      | Remove first-week adoption friction — coexistence, resource budget, uninstall. |
| DISTRIB   | **Proposed (new)**              | 0/5      | Harden the update/distribution loop so hotfix iteration actually reaches users. |
| INSIGHTS  | **Proposed (new)**              | 0/4      | Provide a periodic value signal during the silent middle.                       |
| Docs      | Owned, scoped (6 lanes)         | 0/6      | Adoption / air-gap / witness-chain / hooks-integration / migration / `anvil-run`. |

**Total remaining work for tag:** roughly 50 items across 8 surfaces. This is
substantially larger than the original `v0.7.0-beta` shape (≈22 items), and is
the deliberate cost of "the release we sit on."

---

## Wave Structure

Existing waves remain. New waves slot in alongside them.

| Wave   | Theme                                | Status        | Composition                                                                                |
| ------ | ------------------------------------ | ------------- | ------------------------------------------------------------------------------------------ |
| **0**  | Promote Contracts                    | Done          | ADRs 036–039 accepted, AgentTag stub landed, carry-forward gates confirmed.                |
| **1**  | Load-Bearing Backbone                | Done          | MLP-001 / -002 / -011 / -013 / -017.                                                       |
| **1A** | Beta Watch UX Hotfix                 | In Progress   | WATCHUX-001 through -004 (urgent beta remediation).                                        |
| **2**  | Hook, Policy, Baseline               | Done          | MLP-003 / -005 / -006 / -007 / -008 / -012.                                                |
| **3**  | Coordination + Launcher Ingress      | Ready/Active  | MLP-014 / INTL-001..-009.                                                                  |
| **3A** | Adoption Friction (NEW)              | Ready on accept | ADOPT-001..-006, DISTRIB-001..-005.                                                      |
| **3B** | Trust Surface (NEW)                  | Ready on accept | ADTRUST-001..-006.                                                                         |
| **4**  | Release-Gate Closure                 | Active        | MLP-018, WATCHUX-005..-008, INSIGHTS-001..-004, documentation lanes.                       |
| **5**  | Boring Week Validation (NEW gate)    | Pre-tag       | Internal users run Anvil on real work for one full calendar week under fresh-user config.  |

Waves 3A and 3B run in parallel with the in-flight Wave 3. They do not depend on
INTL completion. They depend on MLP being effectively done (true today) and on
WATCHUX-002 (shared ignore policy) landing before ADOPT-004.

---

## Wave 3A — Adoption Friction

Parallel-safe. Each item is a discrete polish surface; no shared single-point-
of-failure inside the wave.

| Work               | Parallel?           | Notes                                                                                              |
| ------------------ | ------------------- | -------------------------------------------------------------------------------------------------- |
| `ADOPT-001`        | Parallel            | Hook coexistence with lefthook/husky/pre-commit-framework. Extends MLP-008 bootstrap recovery.     |
| `ADOPT-002`        | Parallel            | Resource budget — CPU/RSS measurement and ceiling in `crates/anvil-bench`.                         |
| `ADOPT-003`        | Parallel            | AI tool auto-detect (Claude Code, Cursor, Aider, Windsurf, Codex).                                 |
| `ADOPT-004`        | After WATCHUX-002   | Complete the local-noise ignore policy across `anvil-run`, hooks, and audit (extends WATCHUX-002). |
| `ADOPT-005`        | Parallel            | `anvil uninstall` — clean removal, leaves repo and git config exactly as found.                    |
| `ADOPT-006`        | Parallel            | Editor surface coexistence — no LSP/formatter/extension conflicts on representative configs.       |
| `DISTRIB-001`      | Parallel            | Harden `anvil update` resolution chain (Homebrew vs sidecar vs library) and signature verification. |
| `DISTRIB-002`      | After `DISTRIB-001` | `anvil version --check` surfaces newer versions and security advisories without auto-update.       |
| `DISTRIB-003`      | Parallel            | Homebrew formula maintenance — auto-bump on release, tested on macOS arm64 + x64.                  |
| `DISTRIB-004`      | Parallel            | Release cadence and EOL policy doc (`docs/policies/release-cadence.md`).                           |
| `DISTRIB-005`      | After `DISTRIB-002` | `anvil migrate` — config reconciliation across minor versions.                                     |

## Wave 3B — Trust Surface

| Work          | Parallel?       | Notes                                                                                              |
| ------------- | --------------- | -------------------------------------------------------------------------------------------------- |
| `ADTRUST-001`   | First           | `anvil status` plain-mode is readable at a glance (state, hooks, daemon, last witness).            |
| `ADTRUST-002`   | After ADTRUST-001 | Degraded state surfacing — user is told within 60s of next save-time interaction.                  |
| `ADTRUST-003`   | Parallel        | `anvil doctor` diagnose-and-fix recovery for the common bad states.                                |
| `ADTRUST-004`   | Parallel        | Daemon-down auto-recovery — hooks detect, re-arm; `anvil start` is idempotent.                     |
| `ADTRUST-005`   | After ADTRUST-001 | Pin the `anvil status --json` schema for editor/CI consumption.                                    |
| `ADTRUST-006`   | Parallel        | First `anvil start` prints a short, accurate claim summary the user can verify themselves.        |

## Wave 4 (extended) — Release-Gate Closure

Existing Wave 4 items plus INSIGHTS and the closeout backlogs.

| Work                                  | Parallel?  | Notes                                                                                 |
| ------------------------------------- | ---------- | ------------------------------------------------------------------------------------- |
| `MLP-018`                             | Parallel   | v1-deferrals module closeout backlog (already opened via PR #1507).                   |
| `WATCHUX-005..-008`                   | Sequenced  | Language correction, progressive warm-up TUI, rule modes, config command surface.     |
| `INSIGHTS-001..-004`                  | Parallel   | `anvil insights` weekly summary, suppression health, drift trend.                     |
| Documentation lanes                   | Gate       | Adoption, air-gap, witness-chain, hooks-integration, migration, and `anvil-run` docs. |
| Release runbook `v0.7.0-beta`         | Gate       | Cut from the `v0.6.0-beta` template; lands before tag.                                |

---

## Wave 5 — Boring Week Validation (NEW pre-tag gate)

**Premise:** No amount of test fixture coverage proves "ready to use." The only
proof is using it.

**Protocol:**

1. After Wave 3A / 3B / 4 all green, freeze the candidate SHA.
2. Three or more internal users install the candidate via the fresh-user path
   (Homebrew install or curl installer, no developer overrides) on their primary
   work machine.
3. For one calendar week, each user runs Anvil against their normal daily work.
4. Each user keeps a journal of every visible warning, every suppression, every
   bypass, every disabled check, every daemon failure, every `anvil doctor`
   invocation.
5. End-of-week review: any disabled check or unresolved suppression is a cut
   blocker. Any "I gave up and turned it off" event is a cut blocker. Any daemon
   failure that did not auto-recover is a cut blocker.

**Exit criterion:** All three users finish the week with Anvil still enabled,
the same configuration they started with, and at least one journal entry
describing a real catch they would not have wanted to ship.

**Non-goal:** Wave 5 is not a perf test or a stress test. It is a sustained-use
trust test. The instrument is the journal, not a benchmark.

---

## Cut Criteria For `v0.7.0-beta`

Extends the existing criteria. Items marked **NEW** are gates introduced by this
proposal.

| Criterion                             | Required Evidence                                                                                                    |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Protection claim correctness          | `MLP-009` green: every allowed state is reachable in fixtures and rendered honestly.                                 |
| Protection claim legibility (**NEW**) | `ADTRUST-001` and `ADTRUST-002` green: a non-Anvil developer reads `anvil status` once and explains what it means.       |
| Save-time and hook layers             | Pre-commit, pre-push, post-commit/-merge/-rewrite hooks run with ADR-038 noise discipline.                           |
| Hook coexistence (**NEW**)            | `ADOPT-001` green: install + run alongside lefthook/husky/pre-commit-framework on representative configs.            |
| Witness integrity                     | Concurrent writes, rollover, tamper detection, DAG verification, and worktree survival tests pass.                   |
| Baseline adoption                     | Existing repositories adopt without broad warning noise; hard-pinned classes remain enforced.                        |
| Agent launch path                     | `anvil-run` registers sessions, isolates process groups / Job Objects, heartbeats, cleans up, and reports refusals.  |
| Resource budget (**NEW**)             | `ADOPT-002` green: CPU < 5% steady-state, RSS < 200MB on the reference repo, measured in CI.                         |
| Update path (**NEW**)                 | `DISTRIB-001` green: `anvil update` works on Homebrew, curl-installer, and library paths with signature verification. |
| Clean uninstall (**NEW**)             | `ADOPT-005` green: `anvil uninstall` returns a repo to byte-identical pre-install state for tracked files.           |
| Air-gapped guarantee                  | Core commands pass under a network-blocked sandbox.                                                                  |
| Release machinery                     | `scripts/release/*` can assess, prepare, tag, monitor, verify, and close out the exact `main` SHA being released.    |
| Docs/runbooks                         | User-facing docs and runbooks match the shipped claim; no protection state is described more strongly than evidence. |
| Boring Week (**NEW**)                 | Wave 5 protocol completed; no cut blockers raised.                                                                   |

**Anti-goal (carried forward):** do not ship a partial MLP/INTL slice under the
full-protection claim. If scope must be cut, rename the release claim before
tagging.

**Anti-goal (NEW):** do not bypass Wave 5 because the candidate looks good. The
candidate always looks good. The Boring Week exists because looks-good and
gets-used diverge.

---

## Hotfix Iteration Plan (Post-Tag)

`v0.7.0-beta` is the release we sit on. "Sit on" means **no major release for
six weeks** unless a Boring-Week-tier regression appears. Hotfix iteration
shape:

| Cadence            | Channel                                | Scope                                                                       |
| ------------------ | -------------------------------------- | --------------------------------------------------------------------------- |
| `v0.7.x` patch     | Weekly while user signal is non-empty  | Bug fixes, false-positive reductions, doc corrections.                      |
| `v0.7.x` patch     | Within 48h of any P0 bug               | Crash, data loss, false-claim regression, daemon corruption.                |
| `v0.7.y` minor     | Not before 6 weeks post-tag            | Feature additions; only if `v0.7.0` baseline retention is stable.           |
| `v0.8.0-beta`      | Demand-pulled                          | Driven by a real adopter requirement, not by completion of a backlog.       |

The hotfix policy is the half of "let's use this" that does not show up in this
release plan — it is what makes the *next* release plan trustworthy.

---

## Risks

| Risk                                                            | Mitigation                                                                                                                                                  |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scope expansion makes ship date slip indefinitely.              | Each new module has explicit defer-to-`v0.7.x` items marked in its task body. Cut to the gate criteria, not below.                                          |
| Boring Week protocol is performative, not real.                 | Require journal artefacts as a release record. Journals land in `plans/audits/2026-XX-XX-boring-week-vN.md`.                                                |
| `anvil-run` (INTL) is not actually needed for "let's use this." | Defer cleanly: INTL-001..-005 ship in `v0.7.0`; INTL-006..-009 can land in `v0.7.1` if Wave 3 slips and they would block Boring Week.                       |
| ADTRUST surface increases noise.                                  | ADTRUST-002 has an explicit noise budget (≤1 banner per 60s, never more than one concurrent). Tests pin this.                                                 |
| New modules clash with existing in-flight work.                 | All Wave 3A / 3B work depends only on Done MLP surfaces and one WATCHUX item (WATCHUX-002, already in flight).                                              |

---

## Later Windows (unchanged)

After `v0.7.0-beta`, promote the next slice based on real adoption signals
rather than pre-allocating a fixed version. Team-lead surface, enterprise
compliance, and wider language coverage remain demand-pulled. See
[`ROADMAP.md`](../../ROADMAP.md) Horizon 2.

---

## Acceptance Checklist

The first four items were satisfied on 2026-05-14 when this proposal was
accepted and the live planning surface was reshaped. The last two are
pre-tag follow-ups the live `RELEASE-PLAN.md` carries.

- [x] @aneki accepts the cut claim ("the release we sit on") as the v0.7.0
      shape. *(Accepted 2026-05-14 during the planning conversation.)*
- [x] Each new module (ADTRUST, ADOPT, DISTRIB, INSIGHTS) is reviewed and
      promoted from `Proposed` → `Ready` at the module level (per
      `plans/aps-rules.md` Status Rules). *(Done; see individual module
      headers, all four are `Ready` as of 2026-05-14.)*
- [x] Wave 3A / 3B sequencing is reflected in the live `RELEASE-PLAN.md`.
      *(Done; see Wave 3A / 3B / 5 sections.)*
- [x] `plans/index.aps.md` Contents list and module status tables updated
      to cite the new modules. *(Done; see the new "Adoption & Sustained
      Use" section and the N5-N9 pick rows.)*
- [ ] Wave 5 (Boring Week) participants identified (≥3 names) — pre-tag
      follow-up owned by @aneki.
- [ ] Hotfix iteration policy referenced from a release runbook — the
      policy itself is in `RELEASE-PLAN.md`; runbook cross-link still to
      land (will be picked up alongside the
      `v0.7.0-beta-release-runbook.md` cut from the v0.6.0 template,
      Wave 4 of the live plan).
