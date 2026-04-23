<!--
APS Module: Anvil TS Scanner Retirement
========================================
Retire the TypeScript antipattern scanner now that the Rust engine is
authoritative. Follows ADR-026. ADR-030 supersedes the napi-cutover
plan for TSRET-003/-004 with surface drivers on the intercept daemon
(module DRVR); TSRET-005 is retained with re-pointed dependencies.
See: plans/aps-rules.md
-->

# Anvil TS Scanner Retirement

| ID    | Owner | Status      |
| ----- | ----- | ----------- |
| TSRET | —     | In Progress |

> **Plan change (2026-04-23, ADR-030):** TSRET-003 and TSRET-004 are
> superseded by the `surface-drivers` module (**DRVR**). Both consumers
> cut over to drivers on the anvil-intercept daemon rather than
> embedding a napi binding in-process. TSRET-002's remaining residual
> is narrowed: the napi crate stays `"private": true`; no npm publish,
> no provenance decision, no OOB install-test requirement. TSRET-005
> is retained but now blocks on DRVR rather than on TSRET-003/-004.

## Purpose

ADR-026 made the Rust scanner authoritative but explicitly preserved the
TypeScript scanner in `packages/anvil/core/src/antipattern/` for the
in-process surfaces that cannot shell out to the CLI: the VSCode
extension and the MCP server. That duplication was a pragmatic H1
compromise. It carries ongoing costs:

- Every rule change must land twice (source `.anvil`, then re-verify the
  TS loader still consumes the compiled registry identically).
- Every regex engine divergence becomes a user-visible UX split
  (see `anvil-scanner-parity-gaps` / SPG).
- `tests/scanner-parity/` is indefinitely load-bearing maintenance
  until one engine retires.

This module retires the TS scanner by binding the Rust engine into Node
via `napi-rs` (or WASM as fallback if napi-rs is infeasible on the
targets we ship). After it lands, the VSCode extension and MCP server
call into the same compiled Rust code the CLI uses; the TS scanner code
path is deleted; the parity harness is retired.

Authoritative ADR: [ADR-026](../decisions/026-rust-scanner-authoritative.md).

## Background

- The `packages/anvil/core/src/antipattern/` scanner is a full TS
  implementation of the registry-driven scan loop.
- The VSCode extension at `packages/vscode-extension/` imports it for
  in-editor diagnostics.
- The MCP server at `packages/mcp-server/` imports it for tool calls.
- `napi-rs` is the standard way to expose Rust functions to Node with
  prebuilt per-platform binaries; Anvil already ships native binaries
  per-platform so the distribution story is compatible.
- Dependency: SPG must land first, or at minimum SPG-001..003, so the
  Rust engine actually runs every default rule before the TS path is
  removed.

## Scope

**In scope:**

- `napi-rs` binding crate exposing `scan_artifact` and related surface
- Platform prebuilds for every platform VSCode + MCP must run on
  (Linux x64/arm64, macOS x64/arm64, Windows x64)
- Replacing the TS scanner invocation in the VSCode extension and MCP
  server with the napi binding
- Deleting `packages/anvil/core/src/antipattern/scanner.ts`,
  `registry-loader.ts`, and supporting TS files
- Retiring the TS half of `tests/scanner-parity/`
- Documentation updates reflecting the now-single-engine state

**Out of scope:**

- Further scanner feature work (lives in SPG or a successor module)
- Replacing other TS engine code beyond the antipattern scanner
  (e.g. gate runner, provenance) — separate modules

## Interfaces

**Depends on:**

- `anvil-scanner-parity-gaps` (SPG-001..003 at minimum — cannot retire
  the TS engine while it's the only engine that fires 6 default rules)
- `crates/anvil-checks/` — Rust scanner surface to expose
- `packages/vscode-extension/`, `packages/mcp-server/` — consumers
- `packages/anvil/core/src/antipattern/` — being retired

**Exposes:**

- A single authoritative scanner, reached by every surface
- `@eddacraft/anvil-checks-native` (or similar) napi package

## Tasks

### TSRET-001: napi-rs binding spike

- **Intent:** Prove the Rust scanner can be called from Node with
  acceptable startup and per-call overhead.
- **Expected Outcome:** A new `crates/anvil-checks-napi` crate exposes
  `scan_artifact(artifact, options) -> ScanResult` via `napi-rs`. A
  minimal Node test script calls it and gets identical output to the
  Rust CLI on the same input. Startup cost (cold) and per-call cost
  (warm) measured; numbers recorded in the task comment or an ADR.
- **Scope:** `crates/anvil-checks-napi/` (new), `package.json` test
  script
- **Dependencies:** SPG-001 (so flags are honoured)
- **Validation:** `pnpm --filter @eddacraft/anvil-checks-native test`
  asserts parity with the CLI on a sample fixture
- **Confidence:** medium
- **Priority:** High
- **Status:** Complete

---

### TSRET-002: CI prebuilds for all target platforms

- **Intent:** The napi binding must distribute prebuilt binaries for
  every platform VSCode and MCP run on; otherwise installs fall back
  to local Rust compilation and break non-Rust users.
- **Expected Outcome:** GitHub Actions workflow cross-compiles the napi
  binding for Linux x64, Linux arm64, macOS x64, macOS arm64, and
  Windows x64. Artifacts published to npm as the napi-rs standard
  `@eddacraft/anvil-checks-native-linux-x64-gnu` etc. sub-packages.
- **Scope:** `.github/workflows/`, `crates/anvil-checks-napi/`
- **Dependencies:** TSRET-001
- **Validation:** Fresh clone on each platform installs without a
  Rust toolchain and the scanner runs
- **Confidence:** medium
- **Priority:** High
- **Status:** Scope-reduced (per ADR-030)
- **Scope after ADR-030:** The daemon is the runtime bundling point;
  npm publication of `@eddacraft/anvil-checks-native` is not required
  to complete this module. The CI matrix + test-without-Rust-toolchain
  path remains valuable as a canary on the binding, so the workflow
  stays. What was "remaining to reach Complete" is now **not planned**:
  - `"private": true` stays; no `NPM_TOKEN` wiring needed.
  - OOB install smoke tests for `aarch64-unknown-linux-gnu` and
    `x86_64-apple-darwin` are **not planned** — the binding is
    internal-only.
  - `--provenance` / `id-token: write` decision is **not planned** —
    there is no publish to provenance.
- **Residual work (minor):** Keep `crates/anvil-checks-napi/` building
  in CI as today so the binding doesn't rot while DRVR is in flight.
  Mark this TSRET-002 as Complete once the napi.yml workflow has run
  cleanly on the feat/TSRET-resume work and this plan change is
  merged.

---

### TSRET-003: VSCode extension cuts over to the napi scanner — **Superseded**

- **Status:** Superseded by ADR-030 / DRVR-003.
- **Why:** The driver-framework direction (ADR-015, ADR-030, and
  the design in `plans/specs/anvil-driver-framework/`) routes the
  VSCode extension through a driver on the `anvil-intercept` daemon
  rather than through an in-process napi binding. See `DRVR-003` in
  `plans/modules/surface-drivers.aps.md`.
- **Residual link:** The napi spike work (TSRET-001) is retained as
  an internal CLI acceleration path; nothing from this work item
  needs to be re-executed under DRVR.

---

### TSRET-004: MCP server cuts over to the napi scanner — **Superseded**

- **Status:** Superseded by ADR-030 / DRVR-004.
- **Why:** MCP becomes a driver on the daemon (per the
  driver-framework ADR's "MCP as secondary / fallback driver"
  position). The MCP tool handlers stop importing
  `@eddacraft/anvil-runtime`'s `GateRunner` and instead translate
  into daemon RPCs. See `DRVR-004` in
  `plans/modules/surface-drivers.aps.md`.

---

### TSRET-005: Delete the TS scanner and retire the parity harness

- **Intent:** One engine. One implementation. One place to change.
- **Expected Outcome:** `packages/anvil/core/src/antipattern/scanner.ts`,
  `registry-loader.ts`, `prepare.ts`, and
  `scanner-parity.test.ts` are deleted. Any lingering type-only exports
  from that module that other code depends on are preserved as a thin
  shim re-exporting from the daemon's contracts package (or moved to
  `@eddacraft/anvil-contracts` if they're pure types). The Rust side of
  `tests/scanner-parity/` and the root `test:scanner-parity` script are
  retired. `docs/architecture/rust-architecture-endstate.md` and
  `docs/guides/anvil-rule-authoring.md` updated to describe the
  single-engine state.
- **Scope:** `packages/anvil/core/src/antipattern/`,
  `crates/anvil-checks/tests/scanner_parity.rs`,
  `tests/scanner-parity/`, `package.json`, docs
- **Dependencies:** DRVR-003, DRVR-004 (per ADR-030 supersession),
  SPG-003 (already Complete)
- **Validation:** `pnpm test` passes with scanner-parity suite
  removed; `grep -r "scanner-parity" docs/` returns nothing
  load-bearing
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Proposed

## Risks

- **napi-rs overhead on the VSCode hot path.** VSCode expects
  diagnostics within a few hundred ms of a save. Per-call napi overhead
  plus Rust scan latency must stay inside that budget. Measure in
  TSRET-001 before committing to cutover.
- **Windows arm64 or other niche targets.** If a target we ship to
  lacks a napi prebuild, we either drop the target or keep the TS
  path as a fallback. Decide in TSRET-002 based on actual targets.
- **Transitive consumers of the TS module.** Any code that imports
  `@eddacraft/anvil-core/antipattern` outside the extension/MCP must be
  inventoried in TSRET-005; some may need a TS-side adapter rather
  than a straight import swap.

## Milestones

- **M1 (TSRET-001, TSRET-002):** napi binding exists and installs
  without a Rust toolchain on every target platform.
- **M2 (TSRET-003, TSRET-004):** VSCode + MCP run on the Rust engine
  in production; parity harness still running green.
- **M3 (TSRET-005):** TS scanner deleted; parity harness retired;
  single-engine state documented.

## Progress Log

- **2026-04-21 — module proposed.** Created from operations-reviewer
  OPS-006 in the council review of RSCAN-008 commit `f17a074e`.
  Blocks on `anvil-scanner-parity-gaps` / SPG landing first so the
  Rust engine actually fires every default rule before TS goes away.
- **2026-04-23 — TSRET kicked off; SPG dependency complete.** Module
  status moved to In Progress. TSRET-001 spike landed:
  `crates/anvil-checks-napi/` exposes `scan_artifact` to Node via
  napi-rs v3 with a JSON-in/JSON-out wire format (typed surface
  deferred to TSRET-003/-004 once VSCode/MCP consumer needs are
  pinned). Numbers on the spike fixture (`fixtures/sample.ts`,
  `node v25.9.0`, Linux x64, release build):
  - cold call: ~1.1 ms
  - warm avg over 200 calls: ~0.48 ms
  - debug-build comparison: cold ~6.8 ms, warm ~6.4 ms (use release
    for any VSCode hot-path budget question)

  Well inside the few-hundred-ms VSCode diagnostic budget on a small
  file. TSRET-002 needs to repeat the measurement on a realistic
  (multi-KB, regex-heavy) artefact and confirm prebuild distribution
  works on every shipping target before we cut consumers over.
- **2026-04-23 — TSRET-002 scaffolding landed.**
  `.github/workflows/napi.yml` adds a 5-target build matrix
  (linux-x64-gnu, linux-arm64-gnu, darwin-x64, darwin-arm64,
  win32-x64-msvc) plus a 3-platform install-test matrix (linux-x64,
  darwin-arm64, win32-x64 — the platforms with native GH runners). The
  test job installs with no Rust toolchain to prove the TSRET-002
  contract. Publishing is tag-gated (`napi-v*`), `private: true` on
  the package blocks any accidental publish until the checklist in
  `crates/anvil-checks-napi/README.md` is cleared. `napi
  create-npm-dirs` verified locally — generates the five per-platform
  `npm/<platformArchABI>/package.json` stubs correctly.

  Left open before TSRET-002 can be marked Complete:
  1. Real CI run of `napi.yml` on every target, end-to-end green.
  2. Out-of-band install test on aarch64-linux and x86_64-darwin
     (cross-compiled, no native runner).
  3. Publish-path rehearsal: `@eddacraft` scope ownership, NPM_TOKEN
     secret, one successful `napi pre-publish --dry-run` against the
     registry.
  4. Decision on npm provenance (`id-token: write`, `--provenance`).
- **2026-04-23 — council review (session council-515b14ec).** 5
  reviewers (council-reviewer, security-analyst, adversarial-reviewer,
  operations-reviewer, pragmatic-lead). 5 critical + 14 major findings.
  Fixed in-session:
  - **C-001 (CI workflow ERR_MODULE_NOT_FOUND):** `napi.yml` now uploads
    `index.js` + `index.d.ts` alongside the `.node` artefact and drops
    the `napi create-npm-dirs` "regen" step (it never produced what its
    comment claimed).
  - **C-002 (false parity claim):** README + `lib.rs` crate-level doc
    + test comments rewritten. The binding's `ScanResultOutput` is
    deliberately distinct from the CLI's aggregate `CheckOutput`
    shape. Warning content is parity by construction; envelope is not.
  - **C-003 (`panic = "abort"` host-kill risk):** new
    `[profile.release-napi]` in workspace Cargo.toml inherits release
    but sets `panic = "unwind"`. `package.json` build script now uses
    `--profile release-napi`. `scan_artifact_json` body wrapped in
    `std::panic::catch_unwind`, mapping caught panics to
    `Status::GenericFailure`. CLI binary keeps abort.
  - **C-005 (stale local `.node`):** `pretest` runs
    `scripts/check-binding-fresh.mjs` which fails if `.node` is missing
    or older than `src/`. Verified: triggers correctly on a stale
    binary, passes after rebuild. Tests still green (cold ~1.1 ms,
    warm ~0.52 ms — no regression from the unwind profile).
  Deferred:
  - **C-004 (silent registry-load failure):** pre-existing in the Rust
    scanner's `PATTERN_CATALOGUE` LazyLock. Surfaced more sharply by
    the napi binding (long-running host) but not introduced by TSRET.
    Carved out as a separate item — file under SPG follow-up rather
    than TSRET.

  TSRET-003 prerequisites (the 14 majors — must address before
  consumer cutover):

  Supply chain (security):
  1. Pre-reserve all five `@eddacraft/anvil-checks-native-*`
     sub-package names on npm (publish 0.0.0 placeholders) before
     flipping `private:false`.
  2. Enable npm provenance in the same PR that flips `private:false`
     (`id-token: write` + `--provenance`).
  3. Replace the long-lived classic `NPM_TOKEN` with a granular
     access token (or OIDC trusted publishing); wrap publish job in a
     GH environment with required-reviewer protection.
  4. Add manual approval gate / environment protection to the publish
     job; the current tag-trigger has no human checkpoint.

  Test/parity:
  5. Add a golden-snapshot diff against CLI output; current parity
     test asserts only id membership, not field values or counts.
  6. Correct the AP-001 opt-in misconception in the test comment;
     assert AP-001 fires on the fixture and pin an exact warning
     count.
  7. Re-measure timings on a representative ~500-line TS fixture; the
     9-line spike fixture is not actionable evidence for the VSCode
     hot-path budget.
  8. Exercise `includeOptIn` round-trip without a `patterns` filter
     (current test passes both, which short-circuits the opt-in
     branch).

  Workflow operations:
  9. Verify `pnpm install --frozen-lockfile` against the
     optionalDependencies 404 in a clean checkout; loosen if needed
     until the placeholder packages are published.
  10. Add `timeout-minutes` to every job (build 30, test 15, publish
      30); default is 6 hours.
  11. Scope `concurrency` per-job: keep `cancel-in-progress: true`
      for build/test, set false for publish (a partially-uploaded
      release is non-recoverable).
  12. Split publish into per-target steps with `continue-on-error`
      and a final aggregation step; add a failure notification path;
      document the rollback story (npm unpublish within 72h).
  13. Right-size `paths:` filter — keep `crates/anvil-checks/**` on
      push to main/dev, scope `pull_request` to
      `crates/anvil-checks-napi/**` only (full 8-job matrix on every
      scanner PR is CI noise).
  14. (covered by C-002) README/test parity docs.

  Minors / nits documented in reviewer outputs (workflow `extra_args`
  shape hardening, FFI input size cap, `engines.node` upper bound,
  `prepublishOnly` semantics, `unsafe_code = "deny"` instead of
  `"allow"`, napi-cross glibc ABI, `registry.json` paths trigger gap,
  cache key sharing note, `patterns_checked` sort stability,
  `ScanResultOutput` mirror drift risk, `ScanOptionsInput`
  snake_case alias) — pick up opportunistically during TSRET-003.
