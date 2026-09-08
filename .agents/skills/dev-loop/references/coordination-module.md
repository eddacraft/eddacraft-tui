# Claims and coordination (v1 — degraded)

The long-term goal is a tested Git-native coordination module (see "Next phase"
below). **Until that lands, use this degraded protocol.** Label claims
**advisory** in evidence; do not promise multi-operator collision prevention.

## When to claim

Before any write for a `dev-loop` target (item or module), acquire a claim.
Release on land (merge/PR-done), discard, or explicit handoff.

## Provider order

1. Repository policy `devLoop.claims.provider` if set.
2. Else **git-ref** (below).
3. Else **manual**: record claim in the run checkpoint only and mark
   `claims: degraded-manual`.

## Git-ref protocol (default)

Refs live under `refs/claims/<TARGET>` where `<TARGET>` is the APS id or
`ad-hoc-<slug>` (e.g. `refs/claims/RSLV-001`, `refs/claims/DASH`).

### Acquire

Resolve `<target>`, operator, session, branch, workspace, and a lease expiry as
RFC 3339 UTC instants through the harness. Then:

1. Check `git show-ref --verify --quiet refs/claims/<target>` and
   `git ls-remote --exit-code origin refs/claims/<target>`. An existing local or
   remote ref is a conflict.
2. Write the complete flat claim below to a temporary file through the harness's
   file tool.
3. Run `git hash-object -w -t blob <claim-file>` and capture the printed object
   ID.
4. Atomically create the ref with
   `git update-ref refs/claims/<target> <blob-id> <all-zero-object-id>`.
5. Best-effort publish with `git push origin refs/claims/<target>`, recording
   whether visibility is `published` or `local`.

```text
target: <target>
operator: <operator>
session: <session-id>
branch: <branch>
workspace: <absolute-path>
acquiredAt: <rfc3339-utc>
expiresAt: <rfc3339-utc>
status: active
```

This flat blob is the v1 degraded format; `claim.schema.json` describes the
next-phase claim module, not this blob. Do not require a heredoc, command
substitution, GNU `date`, or BSD `date` to create it.

On conflict → stop with outcome `claim-conflict`.

### Heartbeat / renew

Read the current ref and blob, write a replacement claim with a fresh
`expiresAt`, hash it, then use the compare-and-swap form
`git update-ref refs/claims/<target> <new-blob-id> <old-blob-id>`. A changed old
object is a renewal conflict, not permission to overwrite another operator.
Interval from policy (`heartbeatMinutes`), default 10 minutes during long runs.

### Release

```console
git update-ref -d refs/claims/<target> <expected-blob-id>
git push origin --delete refs/claims/<target>
```

Inspect both results. An already-absent ref is successful cleanup only when it
does not indicate that another operator replaced the expected claim.

Release after successful land, discard, or transfer. Never leave orphan claims
after `integrated` / `discarded`.

### Module vs item

- Claiming `DASH` (module) reserves the namespace; child item claims should name
  parent: include `parent: DASH` in the blob.
- Claiming `DASH-001` alone is enough for single-item work.
- Do not steal a child claim under an active parent owned by someone else.

### Evidence

Record in the run checkpoint / land notes:

```text
claim: refs/claims/<TARGET>
claimsMode: degraded-git-ref | degraded-manual
```

## Next phase (not required for v1)

Atomic hierarchical claims with CAS, lease recovery, and multi-orchestrator
tests. Acceptance criteria:

1. Claim only when neither item nor module namespace is owned by another active lease.
2. Module claims publish child leases only from the owning parent.
3. Renew leases without touching APS plan files.
4. Detect expiry without treating clock skew as immediate abandonment.
5. Recover stale claims only against expected prior revision.
6. Preserve abandoned branches, worktrees, commits, PRs, checkpoints, and evidence.
7. Link release to PR or merge revision before removing the active lock.
8. Expose claims without requiring local session history.

Until those tests pass, **always** say claims are degraded/advisory.
