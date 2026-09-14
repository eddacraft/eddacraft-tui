# Council Review — PR #4568 JREL-013 scoped attestation

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `feat/jrel-013-scoped-attestation`
(protected: `crates/anvil-intercept/src/ipc.rs`; `status.rs` reviewed with it)
**Date:** 2026-09-10
**PR:** https://github.com/eddacraft/anvil-001/pull/4568
**Contract:** JREL-013 — Scope pre-write protection evidence to one worktree (ADR-141 follow-up).
**Code head reviewed:** final push of this review pack (canonicalize-keyed scoped filter included).

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `ipc.rs` | `query_status` accepts only `params: { worktree }` or empty/null; routes to `query_status_for_worktree` | Scoped JSON-RPC; INTD-011 unexpected-field rejection preserved |
| `status.rs` | `query_status_for_worktree`; production uses `sessions_for_worktree` then canonicalize-keyed filter | Avoids materialising unrelated sessions; non-canonical caller paths keep successful lookup |
| `intercept.rs` / `validation.rs` | MCP protection-claim query sends scoped worktree; lossy UTF-8 encode | Pre-write claim cost scales with one worktree |
| Tests | Scoped happy path, unexpected params reject, foreign fences dropped, non-canonical filter key | Contract pins |
| `ARCHITECTURE.md` | Freshness for scoped query + canonicalize filter | Diagrams unchanged — request shaping / session filter |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | Contract delivered; unexpected params still `-32602`. |
| adversarial | approve | Foreign fences dropped; canonicalize-keyed filter closes empty-snapshot hole. |
| security | approve | No new auth surface; strict param allow-list; no cross-worktree leakage. |
| operations | approve (GO) | Full snapshot path intact; MCP 500 ms claim budget retained. |
| pragmatic | approve | Scope matches JREL-013; ARCHITECTURE.md in diff satisfies diagram-impact. |
| **judge** | **Ship** | No in-contract critical or major `must_fix`. |

## Findings

No critical findings. No in-contract major `must_fix` items.

### In-contract minor / nit (advisory)

- general: client-side claim match still uses exact path equality; MCP admitted roots are expected canonical.
- adversarial: foreign primary fence rows dropped from scoped `fences` list; per-worktree `fenced` overlay remains alias-aware.
- pragmatic: APS stays In Progress here; post-merge reconcile moves JREL to 13/13 Merged.

### Later / out of scope

- CIB-405 connection reuse; new JSON-RPC verb; JOURNEY-014/-015 acceptance.

## Evidence

- Full five-seat Council against JREL-013 and `origin/main...HEAD`.
- Local: `cargo test -p eddacraft-anvil-intercept --lib query_status_for_worktree_keeps_sessions_for_noncanonical_caller_path`.
- Diagram-impact: owning `ARCHITECTURE.md` is in the changed set.
