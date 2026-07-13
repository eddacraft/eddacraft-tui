# Repository Policy Contract

Load repository policy **before** asking the user for mode. Search in order;
use the first file that exists and contains a `devLoop` (or `dev_loop`) key:

1. `dev-loop.policy.yaml`
2. `.claude/dev-loop.policy.yaml`
3. `.codex/dev-loop.policy.yaml`
4. `plans/dev-loop.policy.yaml`
5. Fragment under project docs if linked from `AGENTS.md` / `CLAUDE.md`

If **no** policy file exists:

| Context                        | Default                                                                                            |
| ------------------------------ | -------------------------------------------------------------------------------------------------- |
| Interactive human session      | `interactive` — terminal state `review-ready`                                                      |
| Unattended / batch / CI agent  | still no self-merge; park `awaiting-merge-authority` unless `autonomous` + `autonomousMerge: true` |
| Explicit user "run autonomous" | session override; record in evidence                                                               |

**Never infer merge authority from silence.**

## Schema (semantics)

```yaml
devLoop:
  defaultMode: interactive # or autonomous
  integrationBranch: main
  isolation:
    provider: worktrunk # worktrunk | git-worktree | harness
    requireFor: [module, autonomous, parallel]
    alwaysRunSetup: true
  pullRequests:
    defaultBoundary: invocation-target
    autonomousMerge: false
    # Durable fix: enable GitHub "Automatically delete head branches".
    deleteBranchOnMerge: true
    allowStacks: true
  claims:
    provider: git-ref # git-ref | manual | anvil
    leaseMinutes: 30
    heartbeatMinutes: 10
  repair:
    maxCycles: 5
    stopOnNoProgress: true
  risk:
    default: standard
    pathRules: []
    mandatoryDifferentialDesign:
      [architectural, security-sensitive, irreversible, materially-ambiguous]
    crossModelVerification: [high, critical, disputed]
  gates:
    required: []
    postMerge: []
  aps:
    reconcileOnLand: true
    # When a human merges outside the loop, where may APS/docs reconcile land?
    # bookkeeping-pr     — open a small PR to integration (default, safest)
    # integration-direct — allow docs/APS-only commits on integration (must match
    #                      repo history culture; still never for product code)
    # forbid             — stop and ask; no reconcile write until a loop-owned PR
    outOfBandMergeReconcile: bookkeeping-pr
```

## Resolution

1. Load repository policy (paths above).
2. Overlay APS-declared requirements.
3. Raise risk/gates when project truth demands it.
4. Apply human override only when session-scoped, explicit, recorded, and permitted.

Mode controls checkpoints and terminal authority, never target scope.

### Out-of-band merge reconcile

Humans often merge GitHub PRs while the loop is idle. APS status then has no
feature branch left.

| Policy value         | Loop may                                                                                                                                            |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bookkeeping-pr`     | Branch from integration, APS/docs-only commit, open PR (or attach to existing bookkeeping PR)                                                       |
| `integration-direct` | Commit APS/docs-only updates directly on a clean, up-to-date integration branch after ancestor proof of the feature merge. **Not** for product code |
| `forbid`             | Report `needs-plan-update` / ask user; do not write plan files                                                                                      |

Always run **integration-ancestor** verification before any APS `Merged` write.

## Override record

Record operator, session, target, policy varied, authority, rationale, issued
and expiry timestamps, resulting actions. Overrides never bypass branch
protection or destructive-action safeguards outside their scope.
