# Claims and coordination (v1 — degraded)

The long-term goal is a tested Git-native coordination module (see "Next phase"
below). **Until that lands, use this degraded protocol.** Label claims
**advisory** in evidence; do not promise multi-operator collision prevention.

## When to claim

Before any write for a `dev-loop-core` target (item or module), acquire a claim.
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

```bash
TARGET="RSLV-001"   # example
CLAIM_REF="refs/claims/${TARGET}"
OPERATOR="${USER:-unknown}"
SESSION="${SESSION_ID:-local}"
LEASE_MINUTES="${LEASE_MINUTES:-30}"
EXPIRES_AT=$(date -u -d "+${LEASE_MINUTES} minutes" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
  || date -u -v+${LEASE_MINUTES}M +%Y-%m-%dT%H:%M:%SZ 2>/dev/null \
  || echo "")

# Fail if claim already exists (advisory lock)
if git show-ref --verify --quiet "$CLAIM_REF" 2>/dev/null \
   || git ls-remote --exit-code origin "$CLAIM_REF" >/dev/null 2>&1; then
  echo "claim-conflict: $CLAIM_REF already exists"
  exit 1
fi

BLOB=$(git hash-object -w -t blob --stdin <<EOF
target: ${TARGET}
operator: ${OPERATOR}
session: ${SESSION}
branch: ${BRANCH:-}
workspace: ${WORKSPACE:-}
acquiredAt: $(date -u +%Y-%m-%dT%H:%M:%SZ)
expiresAt: ${EXPIRES_AT}
status: active
EOF
)
git update-ref "$CLAIM_REF" "$BLOB"
# Optional publish for multi-clone visibility (best-effort)
git push origin "$CLAIM_REF" 2>/dev/null || true
```

On conflict → stop with outcome `claim-conflict`.

### Heartbeat / renew

Rewrite the blob with a fresh `expiresAt` and `git update-ref` the same ref.
Interval from policy (`heartbeatMinutes`), default 10 minutes during long runs.

### Release

```bash
git update-ref -d "$CLAIM_REF" 2>/dev/null || true
git push origin --delete "$CLAIM_REF" 2>/dev/null || true
```

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
