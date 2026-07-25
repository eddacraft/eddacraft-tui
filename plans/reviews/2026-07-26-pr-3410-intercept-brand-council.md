# Council Review — PR #3410 protected-path brand casing

**Status:** Converged  
**Tier:** full  
**Target:** `crates/anvil-intercept/src/confinement.rs`, `crates/anvil-intercept/src/auth.rs`  
**Date:** 2026-07-26  
**PR:** https://github.com/eddacraft/anvil-001/pull/3410

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `confinement.rs` | `FutureConfigVersion` Display: `Anvil` → `anvil` | Operator-facing error text only |
| `auth.rs` | Test fixture comment `# Anvil drivers v1` → `# anvil drivers v1` | Test-only; comments skipped by allowlist parser |

No control-flow, schema, admission, allowlist matching, or write-refusal logic changes.

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | Aligns with `drift.rs` lowercase brand; tests match enum variants |
| adversarial | approve | Gate is numeric; Display not used for decisions; claim string-only holds |
| security | approve | No auth/confinement/secret control-path impact |
| operations | approve (GO) | No in-repo greps of capital-A form; residual external-grep risk advisory only |
| pragmatic | approve | Completes deferred brand tail; ship on this PR with `council:reviewed` |

## Findings

No critical, major, or minor findings.

### Advisory / nit (non-blocking)

- Nearby capital “Anvil” in developer-facing doc comments in the same files is out of product-Display scope; do not expand this PR to rewrite them.
- Prefer enum-variant matching over Display text if anything later instruments this error path (already the in-tree pattern).

## Evidence

- Full council five-seat review (this document).
- `cargo test -p eddacraft-anvil-intercept --lib` — 1062 passed.
- Diff is Display/`#[error]` string and a `#[cfg(test)]` comment only.

## Decision

**Ship** the protected-path brand casing on PR #3410. Maintainer applies `council:reviewed` after the commit that contains this final protected-path diff is on the PR head.
