# Council Review — PR #3582 portable peer-exe off Linux (CIB-160)

**Status:** Converged (re-council after fix-up)
**Tier:** full
**Target:** `crates/anvil-intercept/src/ipc.rs` (protected), helper crates, CLI refusal copy
**Date:** 2026-08-05
**PR:** https://github.com/eddacraft/anvil-001/pull/3582
**Head reviewed:** `9be222a37685cf816a1b5ab32a90ebadeee613d1` (`fix/cib-160-portable-peer-exe`)
**Prior council head:** `31efee5a5205` (Ship-with-conditions) → fix commit `c61fa7601` → re-council Ship

## Change under review

Portable peer-exe durable membership off Linux (CIB-160): macOS `proc_pidpath`, Windows `QueryFullProcessImageNameW`, real Windows pipe client PID, faithfulness probe, `--persist` refusal copy. Does **not** widen MLP2-025b spoof cross-check.

## Seats (re-council on fix-up head)

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve-with-nits | Prior majors/minors fixed |
| adversarial | approve-with-nits | Majors closed; residual hardening polish only |
| security | approve | No blocking issues |
| operations | GO-with-conditions | Observability closed; native matrix residual for **release claims** only |
| pragmatic | approve | Maintainer may apply `council:reviewed` |
| **judge** | **Ship** | Prior majors closed; no criticals |

## Prior findings — disposition on fix-up

| Finding | Status |
| --- | --- |
| Stale Windows `peer_pid` / `launcher_pid` docs | **Fixed** (`registry.rs`, `lib.rs`) |
| Advisory starttime on macOS/Windows with real peer_pid | **Fixed** (`rederive_pid_starttime`) |
| PATH canary | **Fixed** (absolute Unix; Windows prefers `C:\Windows\System32\ping.exe`) |
| Probe failure logging | **Fixed** (`warn!`) |
| Win32/macOS known-answer tests | **Fixed** |
| Refusal shell path embedding | **Fixed** |

## Decision

**Ship.** Maintainer may apply **`council:reviewed`** on the reviewed fix-up head.

### Conditions (release language only)

1. Do not claim native macOS/Windows intercept **runtime** matrix green without evidence or an explicit waiver.
2. Re-council if protected-path head moves after label.

## Evidence

- Full five-seat council + judge (first pass + re-council).
- `cargo test -p eddacraft-anvil-intercept --lib` — 1064 passed.
- Clippy `-D warnings` clean for intercept + CLI.
- Fix commit addresses all first-pass major/minor findings.

## Residual risks

- Native runtime green claims still need evidence or waive.
- Pre-existing PID-reuse TOCTOU accept → image-read class unchanged.
- Windows path-form mismatch can false-refuse (fail-closed).
