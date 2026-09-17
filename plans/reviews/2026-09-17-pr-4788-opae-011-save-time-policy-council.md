# Council Review — PR #4788 OPAE-011 / OPAE-023 save-time policy hook

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `feat/OPAE-011-prewrite-cache`
(protected: `crates/anvil-intercept/src/save_time.rs`; also
`validate_paths.rs`, CLI pre-write cache, `CliPolicyEvaluator`)
**Date:** 2026-09-17
**PR:** https://github.com/eddacraft/anvil-001/pull/4788
**Contract:** ADR-149 (save-time `PolicyEvaluator` hook; no regorus in
`anvil-intercept`; `check_families` becomes antipattern+policy when injected;
coverage stays graph+antipattern; kill switch
`ANVIL_INTERCEPT_DISABLE_POLICY_EVALUATOR=1`; fail-open). APS OPAE-011
(process-local compiled-engine cache) and OPAE-023 (save-time injection).
Issues #4784 / #4785.
**Session:** `council-pr4788-full` (assurance / full)
**Code head reviewed:** `69b61f4fee3a2e5843f508573a8e1f2a41e0a6b5` (dead_code cfg + cache
mutex release). Protected `save_time.rs` last changed in `604a4327a` (trait +
wiring); later commits on this branch after label must be this review only;
re-council if `save_time.rs` moves after the label.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `save_time.rs` | `PolicyEvaluator` trait; `SaveTimeState` / `ValidateEnv` wire | Daemon stays antipattern-only until CLI injects |
| `validate_paths.rs` | Append policy diagnostics; label `CheckFamily::Policy` | Hook-present saves report antipattern+policy |
| `lib.rs` (`ForegroundOpts`) | `with_policy_evaluator` | Production daemon entry injects CLI impl |
| `intercept_policy_evaluator.rs` | CLI impl → `policy_prewrite::evaluate_many` | Reuses MCP packs + OPAE-011 cache |
| `policy_prewrite.rs` | Process-local mtime-keyed engine cache; `evaluate_many` | Warm pass eval-only; shared by MCP + save-time |
| `watch_save_time.rs` | Honesty for antipattern+policy scope | Attestation wording matches families |
| `daemon_dep_boundary.rs` | Still forbids regorus in intercept | ADR-098 AD-4 unchanged |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | repair then approve | Hook mirrors ADR-067 parser injection. First pass: global cache mutex held across regorus eval on warm hit — would serialise MCP + save-time. Repaired by take/eval/put-back. |
| adversarial | repair then approve | Fail-open (panic → empty diagnostics; kill switches) holds. ChangeKind collapsed to Modified in CLI impl is accepted for this ADR slice (trait is paths-only). Mutex release required under concurrent validate_paths. |
| security | approve | No new trust boundary into intercept; packs stay in CLI process. Kill switch + ANVIL_POLICY_ENFORCEMENT still gate eval. Cache fingerprint is path+mtime+len, not attacker-controlled content injection into the crate graph. |
| operations | approve (GO) | After mutex repair, warm eval no longer holds the process-global lock. Poisoned engines dropped. Truncation remains warning-class. Watch honesty names antipattern+policy. |
| pragmatic | approve | Trait + CLI wiring is the right ADR-071 boundary. Dead evaluate_with_budget without cfg(test) broke -D warnings CI — gated. Cache invalidation tests cover overlay/member mtime. |
| **judge** | **Ship** (decision PASS, gate PASS) | First pass REPAIR. Bounded repairs landed on this tip. No remaining must_fix. |

## Findings

### In-contract must_fix (fixed on reviewed tip)

- **C-001** (general + adversarial + operations): warm cache hit called
  `eval_compiled_engine` while holding `prewrite_engine_cache()` MutexGuard,
  serialising every concurrent MCP pre-write and save-time policy pass in the
  CLI process. Repair: remove under short lock, eval unlocked,
  `remember_compiled_engine` on success / non-poison.
- **C-002** (pragmatic / CI): `evaluate_with_budget` lived outside
  `#[cfg(test)]` but was only called from tests → `-D dead-code` failed Rust
  Tests. Repair: `#[cfg(test)]` on the helper.

### Consider (not must-fix)

- `CliPolicyEvaluator` maps every path to `ChangeKind::Modified`. ADR-149's
  trait is paths-only; create/delete nuance stays a follow-on if packs need
  kind-accurate save-time input (file a real APS item if productised).
- Fence/watch clients that cannot deserialise `CheckFamily::Policy` fail
  closed — ADR already documents same-binary ship as the supported pairing.

## Evidence

- `cargo check -p eddacraft-anvil --bins` — exit 0 after repairs.
- `cargo test -p eddacraft-anvil --bin anvil policy_prewrite` — 19 passed
  (cache warm/invalidate, kill switch, truncation, overlay, routing).
- `cargo test -p eddacraft-anvil-intercept --lib validate_paths` — 31 passed
  including policy hook tests.
- `cargo test -p eddacraft-anvil-intercept --test daemon_dep_boundary` — 7
  passed (no policy-engine / regorus in intercept).

## Label

Maintainer applies `council:reviewed` to the **live** PR head after this
review file lands. Do not label a stale SHA. The label is dismissed on
synchronize.
