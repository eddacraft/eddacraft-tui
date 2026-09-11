# Post-merge: feat-cib-031-scope-dep-audit

PR: #2128
Branch: `feat/cib-031-scope-dep-audit`
APS: CIB-031
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Open the next Rust-only `Cargo.lock` PR after this lands
      and confirm the `Dependency Audit` and `license-check` jobs are
      **skipped** (not just non-required-red), and that the required
      `Lint & Format` / `Type Check` / `Unit Tests (Node 22.x)` resolve
      via the documented PR-only skip-shims rather than blocking merge.
      (human required — needs a real Rust-only-Cargo.lock PR to land)
- [ ] Step 2 — Confirm the next npm-lockfile change (`pnpm-lock.yaml`
      or `package.json`) still triggers `Dependency Audit` + `license-check`
      as today. (agent: yes — observe in CI checks of any such PR)
- [ ] Step 3 — Advance CIB-031 to `Released/Shipped` once it lands in a
      release tag; the module Continuous Improvement Backlog stays In
      Progress (continuous-improvement modules don't reach Complete).
      (agent: yes — on release evidence)

## Notes

The fix is a single restriction in `scripts/ci/classify-changes.sh`:
the `lockfile` case no longer matches Rust paths (`Cargo.lock`,
`Cargo.toml`, `crates/*/Cargo.toml`, `crates/**/Cargo.toml`). Rust
changes already route to the `rust` class, which triggers `cargo-deny`
via `.github/workflows/rust.yml`.

`.github/actions/detect-changes/action.yml` is a pure consumer of the
classifier (line 177 calls `classify-changes.sh`; line 240 reads
`has_check dependency-audit` and forces `FORMAT_REQUIRED`/`LINT_REQUIRED`/
`TYPECHECK_REQUIRED`/`UNIT_TESTS_REQUIRED=true`). After the fix, a
Rust-only `Cargo.lock` PR no longer takes that forced path — the JS
required gates skip via the PR-only skip-shims documented in
`.github/workflows/README.md:67-68`, which keeps the required-check
status satisfied for merge.

Incidental: Cargo.lock-only PRs no longer emit a false
`mixed-change-set` warning (caused by matching both `rust` and `lockfile`
classes today).

If a future PR needs to extend the lockfile gate to npm manifests outside
the existing pattern surface (e.g., `tools/dev/package.json`), that is
out of scope here — file a separate CIB item.
