<!--
APS Module: Anvil TS Scanner Retirement
========================================
Retire the TypeScript antipattern scanner now that the Rust engine is
authoritative. Binds the Rust engine to Node surfaces via napi-rs (or
WASM fallback) so VSCode + MCP stop running a second implementation.
Follows ADR-026. See: plans/aps-rules.md
-->

# Anvil TS Scanner Retirement

| ID    | Owner | Status   |
| ----- | ----- | -------- |
| TSRET | —     | Proposed |

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
- **Status:** Proposed

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
- **Status:** Proposed

---

### TSRET-003: VSCode extension cuts over to the napi scanner

- **Intent:** VSCode users run the authoritative scanner.
- **Expected Outcome:** The VSCode extension imports
  `@eddacraft/anvil-checks-native` instead of
  `@eddacraft/anvil-core/antipattern`. Diagnostics in VSCode match CLI
  output on the same file. Existing extension tests pass; add one that
  asserts a lookaround-affected rule fires (or explicitly doesn't, if
  TSRET is gated on SPG-003).
- **Scope:** `packages/vscode-extension/`
- **Dependencies:** TSRET-002
- **Validation:** `pnpm --filter anvil-vscode test` passes;
  manual scan in VSCode matches `anvil check` output
- **Confidence:** high
- **Priority:** High
- **Status:** Proposed

---

### TSRET-004: MCP server cuts over to the napi scanner

- **Intent:** MCP tool calls run the authoritative scanner.
- **Expected Outcome:** The MCP server imports the napi binding. The
  `scan_artifact` MCP tool returns Rust-scanner output.
- **Scope:** `packages/mcp-server/`
- **Dependencies:** TSRET-002
- **Validation:** `pnpm --filter @eddacraft/anvil-mcp test`; e2e
  MCP call through the harness
- **Confidence:** high
- **Priority:** High
- **Status:** Proposed

---

### TSRET-005: Delete the TS scanner and retire the parity harness

- **Intent:** One engine. One implementation. One place to change.
- **Expected Outcome:** `packages/anvil/core/src/antipattern/scanner.ts`,
  `registry-loader.ts`, `prepare.ts`, and
  `scanner-parity.test.ts` are deleted. Any lingering type-only exports
  from that module that other code depends on are preserved as a thin
  shim re-exporting from the napi binding (or moved to
  `@eddacraft/anvil-contracts` if they're pure types). The Rust side of
  `tests/scanner-parity/` and the root `test:scanner-parity` script are
  retired. `docs/architecture/rust-architecture-endstate.md` and
  `docs/guides/anvil-rule-authoring.md` updated to describe the
  single-engine state.
- **Scope:** `packages/anvil/core/src/antipattern/`,
  `crates/anvil-checks/tests/scanner_parity.rs`,
  `tests/scanner-parity/`, `package.json`, docs
- **Dependencies:** TSRET-003, TSRET-004, SPG-003 (so no user-visible
  regression from losing TS-only rules)
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
