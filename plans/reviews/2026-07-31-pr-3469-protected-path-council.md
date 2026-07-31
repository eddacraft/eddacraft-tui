# Council Review — PR #3469 protected-path comment succinctness

**Status:** Converged  
**Tier:** full  
**Target:** `crates/anvil-intercept/src/{auth,confinement,ipc,save_time,workspace_admission}.rs`  
**Date:** 2026-07-31  
**PR:** https://github.com/eddacraft/anvil-001/pull/3469  
**Head:** final PR head at label application (protected files unchanged after this review)

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `auth.rs` | Module doc shortened to one-line contract summary | Docs only |
| `confinement.rs` | Module doc shortened; ADR/DSV identifiers retained in remaining line | Docs only |
| `ipc.rs` | Module doc + two long implementation comments condensed | Docs only |
| `save_time.rs` | Module doc shortened | Docs only |
| `workspace_admission.rs` | Module doc shortened | Docs only |

Executable code, control flow, schemas, admission logic, allowlist matching, IPC framing, and write-refusal paths are unchanged. Diff filter for non-comment non-blank added/removed lines on the five protected files: **empty**.

A separate non-protected fix on the same head (`test(flags): include dashboard.web…`) only updates the flags catalogue inventory assertion and is outside this gate surface.

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | Comment-only contraction; module contracts still name the surface |
| adversarial | approve | No executable delta to abuse or fail closed/open paths |
| security | approve | Same-uid save-time trust boundary logic untouched |
| operations | approve (GO) | No runtime, config, or deploy surface change |
| pragmatic | approve | Scope is docs/hygiene; ship with `council:reviewed` |

## Findings

No critical, major, or minor findings.

### Advisory (non-blocking)

- Long historical narrative removed from module docs is recoverable from git history and APS module/spec links still referenced in sibling modules; do not re-expand comments in follow-up PRs without need.
- `ipc.rs` retains short comments for dual status-query routing and the Windows peer-SID check so the non-obvious contracts remain local.

## Evidence

- Full council five-seat review (this document).
- Protected-path diff inspected file-by-file; non-comment line filter empty.
- `pnpm --filter @eddacraft/anvil-flags-catalogue test` — 45 passed (CI-unblocking inventory fix only).
- PR validation set (author): `pnpm format:check`, `pnpm lint:rust`, `pnpm typecheck`, `pnpm lint:check`, `pnpm test`, native checks, and `cargo test -p eddacraft-anvil-intercept --lib -- --test-threads=1` (1,062 passed).

## Decision

**Ship** the protected-path comment succinctness on PR #3469. Maintainer applies `council:reviewed` for the final head that contains this protected-path diff (no further protected-file edits after this review).
