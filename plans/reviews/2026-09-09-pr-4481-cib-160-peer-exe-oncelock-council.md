# Council Review — PR #4481 OnceLock peer-exe cache

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `fix/cib-160-peer-exe-oncelock`
(protected: `crates/anvil-intercept/src/ipc.rs`)
**Date:** 2026-09-09
**PR:** https://github.com/eddacraft/anvil-001/pull/4481
**Contract:** GitHub issue #4449 (CIB-160 residual: cache daemon `current_exe` in a
process-lifetime `OnceLock` so probe warm-up and the live durable-membership
gate cannot diverge after an in-place binary replace). The PR description is
evidence of that small-fix residual. Parent contract is #4439 / #4440. CIB-160's
original portable peer-exe work is Released/Shipped; this review does not reopen
that item. JREL-004 rendezvous coordinator is `out_of_scope` unless this PR
regresses it.
**Session:** `council-35eb7667` (assurance / full)
**Code head reviewed:** `14466d5330bc7a40958d57f33c989454bbaec2e8` (last change
to protected `ipc.rs`, including the Clippy `doc_markdown` `OnceLock` backtick).
Later commits on this branch, if any, must be this review only; re-council if
`ipc.rs` moves after the label.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `ipc.rs` | `canonical_daemon_exe` caches `current_exe`+canonicalize in `OnceLock<Option<PathBuf>>`; probe and `peer_authorised_for_durable_membership` both use it | Probe-time and gate-time daemon paths cannot diverge after an in-place binary replace |
| `ipc.rs` tests | `canonical_daemon_exe_is_cached_for_the_process` asserts pointer identity plus a live `current_exe` match | Feasible coverage for #4449; in-place replace is not unit-testable |
| `ARCHITECTURE.md` | Membership invariant names the process-lifetime cache; freshness records the residual | Component authority matches the cache |
| Five `docs/architecture/*` and review-checklist freshness tables | Downstream diagram-impact review recorded | Save/validation/fence diagrams unchanged |

Fail-closed behaviour is unchanged on missing pid, unreadable peer or daemon
path, unfaithful foreign reads (#3130), missing canary, and canary-still-equals-
this-binary after the exec wait. Sticky first-call `None` matches the probe
cache. In-process `register_on_start` / `--persist` remains the escape hatch.
Non-anvil peers stay downgraded. Canary paths remain absolute OS binaries (no
`PATH`). CIB module file is not edited.

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | `canonical_daemon_exe` is the only daemon-path source for probe and live gate. Fail-closed and canary wait are unchanged. Pointer-identity test plus backticked `OnceLock`. |
| adversarial | approve | `OnceLock` serialises init with no re-entrant deadlock. Path `PartialEq` is value, not pointer. Faithfulness-first then same-exe is intact. Same-path inode overwrite remains path-string comparison. |
| security | approve | Faithfulness still runs first. Missing pid / unreadable exe / unfaithful still false. Canary still uses absolute binaries. Cache does not widen same-uid neighbour forge. Path-vs-inode residual is `later_item`. |
| operations | approve (GO) | Warm-up from #4440 pins both caches off the RPC budget. Restart resets the cache. `--persist` remains the in-process hatch. Residual: sticky first-call `None` is silent. |
| pragmatic | approve | Smallest fix that closes #4449. Freshness tables are diagram-impact stamps. Test grain is pointer identity, not two live reads. No CIB module edit. |
| **judge** | **Ship** (decision PASS, gate WARN) | No in-contract `must_fix`. Advisories only. |

## Findings

No critical or major `must_fix` items. In-contract notes are advisory.

### In-contract consider (advisory)

- operations: first-call failure of `canonical_daemon_exe` is sticky for the
  daemon lifetime and silent (`ipc.rs` ~5686). Sibling probe fail-closed paths
  (`tracing::warn` on canary spawn, unreadable canary, aliased read) already
  log. Origin/main also swallowed `current_exe` failure with `let Ok(...) else`.
  A one-shot warn before storing `None` would help operators; do not retry.
  #4449 does not require this.

### Later / out of scope (not must-fix)

- security: the `OnceLock` caches a canonical path string, not the mapped
  daemon image (`ipc.rs` ~5686). After unlink-and-replace at the same pathname,
  a replacement CLI whose live canonical path equals that string can be
  authorised against the still-running old image. On Linux a later
  `current_exe()` typically errors once `/proc/self/exe` gains the `(deleted)`
  suffix, so origin/main fail-closed here. Not a same-uid neighbour forge.
  Leave off #4449. A later item could bind device+inode after
  `foreign_exe_reads_faithful` while keeping the path cache (`later_item`).
- adversarial: sticky `None` is untested because `current_exe` cannot be
  injected (`no_contract`).
- Original CIB-160 portable peer-exe (Released/Shipped).
- Windows `SystemRoot` / `WINDIR` fallback candidates (council #3582).
- JREL-004 rendezvous coordinator.
- Changing the once-per-process probe `OnceLock` lifetime, CIB module edits.

## Evidence

- Full five-seat Council (general, adversarial, security, operations,
  pragmatic) plus supervisor and judge against issue #4449, PR #4481, and
  `origin/main...14466d533` on `fix/cib-160-peer-exe-oncelock`.
  Session `council-35eb7667`.
- Anvil developer MCP graph tools were unavailable for this grok worktree
  (`workspaceRoot` is not the daemon's linked root). Reviewers used `git show`
  / `git diff` of the pinned SHA. Pre-write validation of this file was
  allowed against the main checkout.
- Escalated to **full** because `ipc.rs` is on
  `.claude/hooks/council-protected-paths`.
- Clippy `OnceLock` backticks confirmed on HEAD `14466d533`: the test doc
  comment at `ipc.rs` ~7722 reads "Pointer identity proves the `OnceLock`"
  (commit message: *style(intercept): backtick OnceLock in peer-exe doc
  comment*). CI Clippy (linux) passed; CI Clippy (windows-msvc) passed on this
  SHA.
- Author-reported:
  `cargo test -p eddacraft-anvil-intercept --lib --no-fail-fast canonical_daemon_exe`
  — 1 passed; durable-claim / wait / peer-exe path suites green on the
  implementation commit.
- CIB module file not in the diff (shared multi-writer).

## Decision

**Ship** the CIB-160 OnceLock peer-exe cache on PR #4481. Decision **PASS**
(gate **WARN** for the advisory consider). A maintainer applies
**`council:reviewed`** on the live head that contains this review and the
protected-path diff at `14466d533`. Re-council if `ipc.rs` moves after the
label.
