# OPMODEL-012 — Main-first cutover and dev retirement (action plan)

> **Spec:** [`plans/archive/modules/operating-model-migration.aps.md`](../modules/operating-model-migration.aps.md) → OPMODEL-012 (archived 2026-05-11)
> **Status:** Complete — Phase 0 (PR #1410), Phase 2 (operator-driven 2026-05-11; cutover SHA `b6f236e9`), and Phase 3 (this PR) all landed in the same window. Module archived to `plans/archive/modules/operating-model-migration.aps.md`.
> **Owner:** Josh + Claude (operator + agent split per phase)

## Context as of 2026-05-11

- `dev` = 64 commits ahead of `main`; `main` = 0 unique commits → fast-forward
  cutover is mechanically possible.
- No branch protection on `main` or `dev` → no protection migration burden;
  fresh protection on `main` is part of the cutover.
- Open PRs targeting `dev` at Phase 0 PR open-time (snapshot): #1406
  (`fix(release): align RELORCH contract checks`), #1408
  (`feat(cicd): move coverage to nightly assurance`), #1333 (dependabot
  production-dependencies bump). All need retarget or merge-before-cutover;
  re-run `gh pr list --base dev --state open` immediately before Phase 2.
- 15 CI workflows in `.github/workflows/` total; 1 is cutover-blocking
  (`pr-base-guard.yml`), 6 need post-cutover cleanup, 8 need no change. See
  [`workflow audit`](../../audits/2026-05-11-opmodel-012-workflow-audit.md).
- The four cutover docs (`branching-strategy.md`, `worktree-policy.md`,
  `release-runbook.md`, `SKILL.md`) already carry dual-mode structure with
  explicit "Current Compatibility Model" / "Target Model" sections — Phase 3
  flips authority rather than rewriting from scratch.
- The PR template (`.github/PULL_REQUEST_TEMPLATE.md`) does not reference a
  base branch; no template change needed unless we add a base-branch note.

## Phasing

### Phase 0 — Pre-cutover audit + playbook (one PR, agent-driven, non-destructive)

Outputs in this PR:

1. **Workflow audit** — produce a checked-in inventory at
   `plans/audits/2026-05-11-opmodel-012-workflow-audit.md` listing each of the
   15 workflows under `.github/workflows/`, its current `dev`/`main` triggers,
   and what (if anything) needs to change for cutover. Anything that needs a
   code change before cutover gets a follow-up task in OPMODEL-012's row
   table.
2. **Cutover playbook** at `docs/runbooks/main-first-cutover.md` — the
   step-by-step the operator runs in Phase 2. Triggers, freeze rule,
   commands, verification, rollback, APS/release-record consequences. Style
   matches the OPMODEL-011 playbooks.
3. **Open-PR coordination note** — added to the cutover playbook: lists PRs
   targeting `dev` at cutover time and the retarget/merge-before policy.
4. **APS:** OPMODEL-012 → `In Progress`; nothing marked Complete in this PR.

Validation: `pnpm format:check && pnpm lint:md && pnpm aps:drift`.

Reviewer: operations-reviewer (single council member, same as OPMODEL-011).

### Phase 1 — Workflow updates (collapsed into Phase 2)

Phase 0 audit found exactly **one** cutover-blocking workflow:
`.github/workflows/pr-base-guard.yml`, which actively rejects feat/fix/docs
branches targeting `main`. After cutover those are exactly the PRs that must
target `main`, so the guard's deletion has to happen in the cutover commit
window — there is no useful pre-cutover landing for it.

The other 6 dev-triggered workflows (ci, codeql, napi, release-harness, rust,
security) continue working after cutover (they keep triggering on `main`); the
`dev` triggers become dead. Their cleanup moves to Phase 3 or a follow-up PR.

**Phase 1 is now a no-op** and Phase 2 absorbs the `pr-base-guard.yml`
deletion (Step 4 of the playbook).

### Phase 2 prerequisites

Before opening the Phase 2 cutover window:

- **PR #1410 merged** — Phase 0 outputs (audit + playbook) live on `dev`.
- **CICD-012 complete** — `Main-first cutover readiness for validation
  workflows` lands the validation-side preparation. Without it, the cutover
  can still proceed but any subsequent CICD-012 work would have to bridge
  validation contracts across the cutover; landing it first lets the cutover
  happen against a settled validation surface. CICD-009
  (release-readiness workflow) merged 2026-05-11 in PR #1413 — that
  satisfies one of CICD-012's coordinating items.

### Phase 2 — Coordination window + the cutover (operator-driven, no PR)

Operator-owned per the playbook. Agent does not push, does not change branch
protection, does not retarget anyone else's PRs. Steps the operator runs,
quoting from the playbook:

1. Notify open-PR owners (PRs #1406, #1408, #1333 plus any new ones at
   cutover time): merge-before-cutover or retarget-after.
2. Open the cutover window. Freeze merges to `dev`.
3. Confirm `git rev-list --count origin/dev..origin/main` is `0` (still a
   clean fast-forward).
4. Push `dev`'s HEAD to `main` (fast-forward; no force).
5. Add branch protection on `main` (required CI checks, PR review, no force
   push). Concrete settings live in the playbook.
6. Set repo default branch to `main` if not already.
7. Restrict `dev`: branch protection that blocks direct push and (optionally)
   requires admin approval to PR. Or delete `dev` if the team is done with it.
8. Smoke check: open a no-op PR against `main`, confirm CI runs, close PR.
9. Communicate cutover complete.

### Phase 3 — Docs flip + APS close-out (one PR, agent-driven)

Opens after the operator confirms Phase 2 is done. **The Phase 3 PR body
must include three machine-verifiable Phase 2 evidence blocks** (cutover SHA,
branch-protection JSON, default branch) per
[`docs/runbooks/main-first-cutover.md#verification`](../../../docs/runbooks/main-first-cutover.md#verification).
Without those, do not open the PR. This is the interlock that prevents a
fresh agent session from flipping the docs based on chat-memory alone.

Outputs:

1. Flip authority in the four cutover docs:
   - `docs/guides/branching-strategy.md` — promote "Target Model" to
     authoritative; move "Current Compatibility Model" to a clearly-marked
     Archive section (or remove if no longer cited anywhere).
   - `docs/guides/worktree-policy.md` — same flip.
   - `docs/runbooks/release-runbook.md` — drop normal `dev -> main` promotion;
     keep emergency-recovery references to OPMODEL-011 playbooks.
   - `.claude/skills/release/SKILL.md` — drop the "compat-mode" hedge in Mode
     Selection if RELORCH-011 has shipped; otherwise keep but reword to
     reference completed cutover authority.
2. Update `.github/PULL_REQUEST_TEMPLATE.md` with a base-branch note if the
   review surfaces ambiguity (small addition, may skip).
3. Sweep for stale references to `dev` as the integration target:
   `grep -rn 'dev branch\|target dev\|dev -> main' docs/ .claude/ plans/`.
4. APS: OPMODEL-012 → `Complete` with completion line citing the cutover
   commit, the protection-rule change, and the docs flip PR.
5. Update `plans/index.aps.md` OPMODEL row to 12/12 and mark module
   In Progress → Complete; queue the module for archive per APS rules.
6. Sweep cross-cutting callouts per `aps-rules.md#cross-cutting-modules`
   (resolve / downgrade / document-and-close) before archive.

Validation: `pnpm format:check && pnpm lint:md && pnpm aps:drift`. Plus
`grep -rn 'dev branch\|target dev\|dev -> main' docs/ .claude/ plans/`
returns only archive/historical references.

Reviewer: operations-reviewer (single council member).

## Hard constraints

- Agent does not force-push, does not change branch protection, does not edit
  CI workflow `branches:` triggers without an explicit per-PR approval, and
  does not retarget PRs owned by other people.
- Phase 2 stays operator-owned even though the playbook is checked in; the
  agent does not transition into "execute the playbook" autonomously.
- If the fast-forward window closes (e.g. someone pushes to `main` between
  Phase 0 and Phase 2), the playbook stops and the operator decides whether
  to merge-then-cutover or to abort.

## Risk register

| Risk | Mitigation |
|---|---|
| Open PRs targeting `dev` get stranded | Phase 2 step 1 forces explicit merge-before/retarget decision per PR. |
| CI workflow pinned to `dev` silently stops running after cutover | Phase 0 audit catches; Phase 1 PRs land before Phase 2. |
| Branch protection misconfigured on `main` | Playbook lists the exact required check names from `gh pr checks` on a recent merged PR; operator copies that list. |
| Someone pushes to `main` directly between Phase 0 and Phase 2 (no protection yet) | Phase 2 step 3 re-checks fast-forward window; if broken, abort and reschedule. |
| Docs flip lands before Phase 2 completes | Phase 3 PR body must paste three machine-verifiable Phase 2 evidence outputs (cutover SHA, branch-protection JSON, default branch) per the [cutover playbook Verification section](../../../docs/runbooks/main-first-cutover.md#verification). PR cannot be reviewed without those three blocks present. Stops a fresh agent in a new session from opening Phase 3 on chat-memory alone. |

## Decision points awaiting operator

1. Phase 0 scope as above? (default: yes)
2. Should Phase 2 retain `dev` as a dated compatibility branch, or delete it?
   (Spec says "protected, retired, or given an explicit compatibility expiry"
   — operator picks at Phase 2 time.)
3. Required CI checks for `main` branch protection — Phase 0 lists candidates
   from `gh pr checks 1407`; operator confirms the canonical list at Phase 2.
