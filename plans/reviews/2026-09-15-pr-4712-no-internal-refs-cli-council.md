# Council Review — PR #4712 drop internal planning ids from CLI

**Status:** Converged
**Tier:** full (required: protected `crates/anvil-intercept/src/save_time.rs`)
**Target:** `origin/main...HEAD` on `fix/cib-426-no-internal-refs-in-cli`
**Date:** 2026-09-15
**PR:** https://github.com/eddacraft/anvil-001/pull/4712
**Contract:** GitHub issue #4711 / CIB-426 — consumer-facing and operator-visible
CLI/daemon output must not cite internal planning IDs (ADR/CIB/DLIFE/…).
Behaviour of witness append, ChainBroken refusal, and hook control flow is out
of scope except that messages keep the same meaning without the IDs.
**Session:** `council-pr-4712-cib-426` (assurance / full)
**Code head reviewed:** `2607bfaafe1f383bb0f7f877e0cd83233b9625db` (daemon
ChainBroken warn string). Protected `save_time.rs` last changed in that commit.
This review-file commit must not edit `save_time.rs`; re-council if it moves
after the label.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `save_time.rs` | `WitnessRoute::Daemon` ChainBroken `tracing::warn` drops `(ADR-038)` | Operator stderr no longer leaks the planning ID; refusal outcome unchanged |
| `hook.rs` (earlier on branch) | Embedded-writer ChainBroken log already omitted ADR-038 | Daemon and embedded legs now match |
| CLI/tests (earlier) | Intercept bail + unit test point at `anvil start`/`anvil watch`; assert no `ADR-`/`DLIFE` | Copilot thread 1 already satisfied on this head |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | One-line string hygiene; `WitnessOutcomeKind::ChainBroken` path untouched. |
| adversarial | approve | No reseeding, no control-flow change, no new error swallowing. |
| security | approve | Trust boundary behaviour identical; only operator-visible copy. No auth/IPC/confinement widening. |
| operations | approve (GO) | Matches embedded-writer message; CI Rust Test/Clippy/Check already green on `2607bfaaf`. |
| pragmatic | approve | Completes Copilot thread 2; proportional to CIB-426. |
| **judge** | **Ship** (decision PASS, gate PASS) | No `must_fix`. |

## Findings

None in-contract. No `must_fix` / `should_fix`.

## Evidence

- Diff on protected file is a single string literal removal of ` (ADR-038)`.
- Branch head `2607bfaaf`: Rust Test, Clippy, Check, Format, NAPI builds green
  before this review-file commit.
- Copilot threads `PRRT_kwDOPxkQR86imFWN` and `PRRT_kwDOPxkQR86imFWr` replied and
  resolved.

## Label

Maintainer applies `council:reviewed` to the **live** PR head after this
review file lands. Do not label a stale SHA. The label is dismissed on
synchronize.
