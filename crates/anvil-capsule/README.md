# anvil-capsule

| Type   | Authority     | Owner | Status | Freshness                                                                                |
| ------ | ------------- | ----- | ------ | ---------------------------------------------------------------------------------------- |
| README | Authoritative | CAPS  | Live   | Last reviewed 2026-08-20 against `crates/anvil-capsule/src` and its tests at `f0f834b39` |

| Upstream                                                  | Downstream                                                          |
| --------------------------------------------------------- | ------------------------------------------------------------------- |
| `crates/anvil-capsule/src`, ADR-072, ADR-073, and ADR-074 | `anvil capsule` CLI commands, CI consumers, reviewers, and auditors |

File-format and verification library for review capsules. A capsule packages
evidence for one commit range in an inspectable directory without requiring a
service. This crate owns canonical encoding, the closed manifest and
verification schemas, evidence collection/writing, verification, and prune
planning. The CLI owns command parsing and user interaction.

## Lifecycle

1. Collect commits, policy/rules/baseline digests, the full witness chain,
   diagnostics, and applied exceptions.
2. Canonically encode the ten required evidence files and scan all writable
   content for identifiable secret shapes before touching the destination.
3. Record byte-level SHA-256 digests in `manifest.json` and publish the
   directory through the platform's no-follow path.
4. Verify schema, required files, digests, witness/range binding, live
   repository digests, and exception state. Verification writes a
   machine-readable worst-of result.
5. Plan pruning separately from applying it; callers can review the exact
   capsule set before deletion.

## Contract invariants

- `anvil.capsule.v1` and `anvil.capsule-verification.v1` are closed schemas;
  unknown fields or schema versions do not silently round-trip.
- All ten evidence files are present even when empty. Missing evidence is
  degraded, never equivalent to no findings.
- Canonical JSON and ordered maps make digest bytes deterministic.
- `verification.json` starts degraded until checks establish a stronger verdict;
  the combined result is the worst of its checks.
- Verdict exits are stable: pass/warn `0`, block `1`, degraded `2`, error `3`.
- The full witness chain is carried; sequence pointers locate the range window
  without claiming every line in that window belongs to the range.
- Secret-shaped evidence is rejected before publication. Entropy-only guesses
  are deliberately excluded because capsule evidence is digest-dense.
- Verification refuses unsafe file types and path traversal. Unix reads use
  no-follow opens; Windows publication uses the Win32 no-follow helper.

## Failure and fallback

Malformed/unreadable evidence produces an error or degraded result rather than a
clean pass. Digest or witness tampering and invalid/revoked exceptions block.
Repository drift and missing attributable evidence degrade. Prune planning skips
entries it cannot safely classify, and application reports per-entry failures
rather than concealing partial results.

## Local validation

```bash
cargo test -p eddacraft-anvil-capsule
```

## Source references

- `crates/anvil-capsule/src/manifest.rs`
- `crates/anvil-capsule/src/format.rs`
- `crates/anvil-capsule/src/verify.rs`
- `crates/anvil-capsule/src/verification.rs`
- `crates/anvil-capsule/src/prune.rs`
- `crates/anvil-capsule/src/canonical.rs`

## Related authorities

- [Public capsule concept](../../docs/public/anvil/concepts/review-capsules.md)
- [ADR-072](../../plans/decisions/072-git-native-governance-substrate.md)
- [ADR-073](../../plans/decisions/073-durable-vs-local-anvil-state.md)
- [ADR-074](../../plans/decisions/074-review-capsule-v0-format.md)
- [Pre-migration historical snapshot](../../docs/architecture/capsule-as-built.md)
