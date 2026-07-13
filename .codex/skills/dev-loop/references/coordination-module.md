# Git-native Coordination Module — Next Phase

The skills define coordination semantics; they do not claim that documentation alone provides distributed locking.

## Objective

Implement atomic hierarchical claims across a human team where every operator may run multiple orchestrators and agents.

## Proposed transport

Use remote claim refs or claim branches. Represent each claim as a small metadata commit. Acquire a claim by atomically creating its ref. Renew and recover with compare-and-swap semantics such as `--force-with-lease` against the expected claim revision.

## Required behaviours

1. Claim an item only when neither it nor its module namespace is owned by another active lease.
2. Claim a module and publish child leases only from the owning parent claim.
3. Renew leases without touching APS plan files.
4. Detect expiry without treating clock skew as immediate abandonment.
5. Recover a stale claim only against its expected prior revision.
6. Preserve abandoned branches, worktrees, commits, PRs, checkpoints, and evidence.
7. Link release to PR or merge revision before removing the active lock.
8. Expose claims to humans and all orchestrators without requiring local session history.

## Acceptance tests

- Two simultaneous acquisitions of one target yield exactly one owner.
- `DASH` conflicts with `DASH-001` unless the child is delegated by the `DASH` owner.
- Two simultaneous stale recoveries yield exactly one successor.
- A failed heartbeat does not delete work or release a non-expired lease.
- Completion records the result revision before release.
- Network interruption and retry are idempotent.

Until this module passes concurrency and recovery tests, adapters must label claims as advisory or degraded rather than promising collision prevention.
