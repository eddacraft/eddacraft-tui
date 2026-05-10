# Anvil Release Plan

**Last updated:** 2026-05-10 (current delivery: OPMODEL + CI/CD readiness)

> Companion: [ROADMAP.md](./ROADMAP.md) for thematic horizons. Execution source
> of truth: [`plans/index.aps.md`](./plans/index.aps.md) and the linked APS
> modules. This file selects the release slate and shows what can run in
> parallel; it does not duplicate every APS work item.

---

## Current State

**Latest verified tag in repo:** `v0.6.1-beta`

`v0.6.0-beta` and `v0.6.1-beta` have moved Anvil past wow-start activation and
daemon-backed validation. The active release work is no longer product-surface
expansion. It is the operating-model cutover that makes future releases
repeatable:

- `OPMODEL`: Plan / Build / Release operating model migration, 6/12 complete.
- CI/CD readiness: release-readiness workflow, candidate artefacts, release
  records, drift checks, and release-command integration.
- `RELORCH`: deterministic `scripts/release/*.sh` command surface, currently
  Proposed, 0/11.

The next release should be scoped as an operational release, not another feature
bundle.

---

## Current Candidate: `v0.6.2-beta`

**Claim:** _The release operating model is executable._ A selected SHA can be
assessed, checked, reviewed, and prepared for release through deterministic
guidance and CI evidence rather than prose-only runbooks.

**Recommended scope:** ship the minimum viable operating model before the
`main`-first cutover.

**Release type:** beta patch/minor operational release. Do not market it as the
full daemon-working product release; it is the release machinery and workflow
foundation that lets that later claim be safe.

**Hard gates:**

- `OPMODEL-007` deterministic agent guidance exists in advisory mode.
- `OPMODEL-008` review/council entrypoints use the same review tiers.
- `OPMODEL-010` APS/repo/release drift checks exist at least in warning mode.
- Release-readiness CI can validate an exact SHA or the implementation gap is
  explicitly tracked under RELORCH/CI and not claimed as shipped.
- `OPMODEL-012` does not execute until rollback playbooks and drift checks are
  usable.

---

## Release Lanes

These lanes can run in parallel after the listed gates are respected.

| Lane                        | Owner module                                                                                                                                       | Can run now?                                         | Main outputs                                                                         | Blocks                                   |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------- |
| L1: Deterministic guidance  | [`OPMODEL-007`](./plans/modules/operating-model-migration.aps.md#opmodel-007-deterministic-agent-guidance-script)                                  | Yes                                                  | `scripts/agent/guidance.sh`, fixtures, advisory CI/hook output.                      | L3, L4, L6                               |
| L2: Review routing          | [`OPMODEL-008`](./plans/modules/operating-model-migration.aps.md#opmodel-008-review-and-council-entrypoint-alignment)                              | Yes                                                  | `/review`, `/council`, planning-council entrypoints aligned to one tier model.       | L6                                       |
| L3: Release CI/CD readiness | [`OPMODEL-005`](./plans/modules/operating-model-migration.aps.md#opmodel-005-release-readiness-and-candidate-artefact-workflow-design), RELORCH/CI | Yes, after L1 contract shape is stable enough        | `.github/workflows/release-readiness.yml`, candidate metadata, exact-SHA validation. | L6, release tags that claim CI readiness |
| L4: Drift checks            | [`OPMODEL-010`](./plans/modules/operating-model-migration.aps.md#opmodel-010-apsreporelease-drift-checks)                                          | Yes, can start with warning fixtures                 | APS/repo/release consistency checks, CI warning mode.                                | L6, L7                                   |
| L5: Release commands        | [`RELORCH-001..011`](./plans/modules/release-orchestration.aps.md)                                                                                 | Yes, but RELORCH-001 then RELORCH-002 are sequential | `scripts/release/{assess,preflight,prepare,promote,tag,monitor,verify,closeout}.sh`. | Full target `/release` mode              |
| L6: Recovery playbooks      | [`OPMODEL-011`](./plans/modules/operating-model-migration.aps.md#opmodel-011-rollback-and-incident-playbooks)                                      | Yes, after L3/L4 failure classes are named           | Bad main, bad candidate, bad artefact, bad release, hotfix playbooks.                | L7                                       |
| L7: Main-first cutover      | [`OPMODEL-012`](./plans/modules/operating-model-migration.aps.md#opmodel-012-main-first-cutover-and-dev-retirement)                                | No                                                   | Promote current `dev` to `main`, retarget normal PRs, retire/protect `dev`.          | L1, L2, L4, L6                           |

---

## Parallel Delivery Shape

### Wave 0: Lock Contracts

Run these first. They unblock parallel execution without requiring branch
cutover.

| Work                            | Parallel? | Notes                                                                        |
| ------------------------------- | --------- | ---------------------------------------------------------------------------- |
| `OPMODEL-007` guidance contract | Yes       | Defines the path/risk/check JSON consumed by CI, hooks, and agents.          |
| `OPMODEL-008` review tiers      | Yes       | Can proceed from the existing council/review specs while L1 builds fixtures. |
| `RELORCH-001` command design    | Yes       | Must finish before RELORCH command harness or command implementation.        |

### Wave 1: Build Advisory Controls

Once Wave 0 contracts exist, split into independent implementation lanes.

| Work                           | Parallel?               | Notes                                                                                           |
| ------------------------------ | ----------------------- | ----------------------------------------------------------------------------------------------- |
| `OPMODEL-010` drift checks     | Yes                     | Start warning-only with fixtures; do not fail normal CI yet.                                    |
| Release-readiness workflow     | Yes                     | Implement exact-SHA validation and candidate metadata. Keep publishing credentials unavailable. |
| `RELORCH-002` harness          | No, after `RELORCH-001` | Harness encodes the command JSON and exit-code schema.                                          |
| `RELORCH-003` assess           | Yes, after harness      | Low-risk first command; useful before the full release surface exists.                          |
| `RELORCH-004` preflight        | Yes, after harness      | Parity with existing checks; JSON output is the main new contract.                              |
| `RELORCH-010` closeout dry-run | Yes, after harness      | Can start in dry-run/input-gated mode before tag/monitor commands exist.                        |

### Wave 2: Prove Failure Handling

This wave turns advisory controls into something operators can trust.

| Work                             | Parallel?          | Notes                                                                                    |
| -------------------------------- | ------------------ | ---------------------------------------------------------------------------------------- |
| `OPMODEL-011` rollback playbooks | Yes                | Depends on named failure classes from readiness and drift checks.                        |
| `RELORCH-005` prepare            | Yes, after harness | Highest-complexity command; protect it with kill/re-run tests.                           |
| `RELORCH-006` promote            | Yes, after harness | Compatibility mode may still understand `dev -> main`, but must label it migration-only. |
| `RELORCH-008` monitor            | Yes, after harness | Can be tested against recent release workflow runs.                                      |
| `RELORCH-009` verify             | Yes, after harness | Can replay checks against `v0.6.1-beta`.                                                 |

### Wave 3: Cutover Decision

Do not start until Waves 1 and 2 have produced usable evidence.

| Work                               | Parallel? | Notes                                                                                                      |
| ---------------------------------- | --------- | ---------------------------------------------------------------------------------------------------------- |
| `RELORCH-007` tag                  | Limited   | Tagging is the irreversible command; implementation can happen, but real use waits for readiness evidence. |
| `RELORCH-011` wire-up/decommission | No        | Last RELORCH item; requires commands green through a differential window.                                  |
| `OPMODEL-012` main-first cutover   | No        | Requires guidance, review routing, drift checks, and rollback playbooks.                                   |

---

## What Can Be Done In Parallel Now

Start these as separate branches/worktrees if capacity exists:

| Track                       | First PR                          | Why safe in parallel                                                                                        |
| --------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Guidance                    | `OPMODEL-007`                     | Mostly new deterministic script + fixtures; downstream consumers can wait on JSON shape.                    |
| Review routing              | `OPMODEL-008`                     | Skill/command docs can align to the already-defined operating model without touching CI.                    |
| RELORCH design              | `RELORCH-001`                     | Spec-only; gates release command implementation but does not conflict with OPMODEL docs.                    |
| Readiness workflow skeleton | CI implementation for OPMODEL-005 | Can create a disabled/manual `release-readiness.yml` with exact-SHA checkout and no publishing permissions. |
| Drift check design/fixtures | `OPMODEL-010`                     | Warning-only checks can land without changing release authority.                                            |
| Recovery playbook outline   | `OPMODEL-011` draft               | Can draft structure now, then fill concrete commands after readiness/drift failure classes settle.          |

Avoid parallelising these too early:

- `RELORCH-002` before `RELORCH-001`; the harness needs the schema.
- `RELORCH-005..010` before `RELORCH-002`; command contracts should fail in CI
  from day one.
- `OPMODEL-012` before `OPMODEL-007`, `OPMODEL-008`, `OPMODEL-010`, and
  `OPMODEL-011`; cutover without guardrails creates a bad-main recovery problem.
- Any release claim that says CI readiness is authoritative before the workflow
  validates the exact SHA.

---

## Suggested `v0.6.2-beta` Cut

Ship this if the goal is a tight, honest operational release:

| Pick                      | Include                                                                                                   | Exclude                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Minimum                   | `OPMODEL-007`, `OPMODEL-008`, `OPMODEL-010` warning mode, release-readiness workflow skeleton/manual mode | `OPMODEL-012`, full RELORCH command retirement                    |
| Strong                    | Minimum + `RELORCH-001`, `RELORCH-002`, `RELORCH-003`, `RELORCH-004`                                      | `RELORCH-007` real tag command as primary path                    |
| Full operating-model beta | Strong + `OPMODEL-011`, `RELORCH-005`, `RELORCH-008`, `RELORCH-009` replayed against `v0.6.1-beta`        | `OPMODEL-012` unless rollback and branch protections are verified |

**Recommendation:** take the **Strong** cut. It gives agents and humans a real
operating-model loop: deterministic guidance, aligned review, warning drift
checks, exact-SHA readiness design starting to execute, and the first release
commands under harness. Save `OPMODEL-012` for a separate cutover release after
a green dry run.

---

## Later: Daemon-Working Product Release

The daemon-working product slate remains valuable, but it should not compete
with OPMODEL/CI/CD cutover work in the same release. Keep it queued until
release machinery is trustworthy.

| Future slice                             | Source                                                | Gate before promotion                                                                    |
| ---------------------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Multi-Layer Protection v1                | [`MLP`](./plans/modules/multilayer-protection.aps.md) | OPMODEL cutover complete or explicitly deferred; release readiness can prove exact SHAs. |
| Intercept Launcher v1                    | [`INTL`](./plans/modules/intercept-launcher.aps.md)   | Shared `AgentTag` schema coordinated with MLP-014.                                       |
| Enterprise/compliance/language expansion | Queued APS modules                                    | Promote only after daemon-working and release machinery are stable.                      |
