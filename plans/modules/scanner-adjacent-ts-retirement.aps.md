<!--
APS Module: Scanner-Adjacent TS Surface Remediation
===================================================
Close the active capability gaps left after TSRET-005 archived scanner-adjacent
surfaces. This is a cross-cutting module per
plans/aps-rules.md#cross-cutting-modules: it coordinates core, runtime, CLI,
export, drift, explain, and MCP return-path ownership without replacing the
owning modules for those surfaces.
-->

# Scanner-Adjacent TS Surface Remediation

| ID    | Owner | Status | Progress |
| ----- | ----- | ------ | -------- |
| TSGAP | -     | Ready  | 0/9      |

## Purpose

Restore, rehome, or explicitly retire the scanner-adjacent capabilities that
were archived with TSRET-005 but do not yet have complete active replacements:
drift snapshots, constraint export APIs, persistent suppressions, anti-pattern
explanations, stale package exports, and MCP resource/prompt return paths.

TSRET-005 is complete. This module is the post-archive remediation plan that
turns the archive from “source removed” into “capability ownership settled”.

## Background

ADR-033 archived the IDE/MCP TypeScript surfaces and enabled immediate
retirement of the TS scanner stack. The executed TSRET-005 archive went beyond
the scanner runtime: it also moved scanner-adjacent TS surfaces into
`archive/anvil-ts-scanner/`, including runtime gate code, runtime export
formatters, suppression store/service, drift snapshot/compare/reporting, and
the anti-pattern explainer.

The archive removed active imports, but several capabilities are now partial or
explicitly absent:

- `packages/anvil/core/package.json` still advertises `./antipattern`,
  `./suppression`, and `./drift` subpath exports even though those active source
  trees are gone.
- `.anvil` compiler code still lives under `packages/anvil/core/src/antipattern/format/`,
  which is semantically not scanner runtime but is still path-coupled to the
  archived namespace.
- Drift snapshot/compare/reporting has a Rust CLI implementation, but no active
  package API replacement and known lower-fidelity areas.
- Runtime export collector/formatters were archived; Rust CLI export exists, but
  there is no active TS package/API replacement.
- Persistent suppression store/service were archived; Rust has parser authority
  and local readers, not a reusable store/service owner.
- The TS anti-pattern explainer was archived at
  `archive/anvil-ts-scanner/core-explain-antipattern.ts`; AP-* explanation
  capability has not been reimplemented.
- MCP resources/prompts that previously depended on TS export/context shapes
  need an explicit Rust/RMCPF return path.

## Scope

In scope:

- Clean active package exports so they match shipped source.
- Decouple the `.anvil` compiler from the archived scanner namespace.
- Decide and implement active ownership for drift snapshots and reports.
- Decide and implement active ownership for constraint export APIs and formats.
- Decide and implement active ownership for persistent suppressions.
- Rehome or explicitly retire anti-pattern explanation capability.
- Align MCP resource/prompt return paths with Rust-owned contracts.
- Update plans/docs so completed TSRET does not imply missing capabilities are
  complete.

Out of scope:

- Reopening archived IDE/MCP TypeScript packages.
- Reopening completed DRIFT, LLMS, or TSRET work as executable modules.
- Moving active surface work back to the private napi package.
- Adding new scanner rules beyond preserving current behaviour.

## Interfaces

Depends on:

- `TSRET-005` completion as the archive baseline.
- `RCLI2-003` for Rust drift command ownership.
- `RCLI3-015` for explain command ownership.
- `RMCPF-020` for future MCP resource parity.
- `crates/anvil-checks` for Rust scanner and suppression parser authority.

Exposes:

- A post-TSRET capability remediation backlog.
- Explicit retain, port, or retire decisions for each archived scanner-adjacent
  surface.
- Validation gates that prevent stale exports and docs from claiming missing
  capabilities are active.

Coordinates with:

- `plans/modules/anvil-ts-scanner-retirement.aps.md` - TSRET is historical
  closeout context; this module owns post-close capability remediation.
- `plans/modules/rust-cli-tier2.aps.md` - RCLI2-003 owns active Rust CLI drift
  behaviour.
- `plans/modules/rust-cli-tier3.aps.md` - RCLI3-015 owns the future Rust
  explain command.
- `plans/modules/rust-mcp-full-port.aps.md` - RMCPF owns future MCP resources
  and prompts sourced from Rust contracts.

## Tasks

### TSGAP-001: Ratify post-TSRET gap scope

- **Intent:** Make the scanner-adjacent remediation backlog visible now that
  TSRET is marked Complete.
- **Expected Outcome:** APS index and TSRET closeout text point to TSGAP as the
  owner for drift/export/suppression/explain/export-contract follow-up work.
- **Files:** `plans/index.aps.md`,
  `plans/modules/anvil-ts-scanner-retirement.aps.md`,
  `plans/modules/scanner-adjacent-ts-retirement.aps.md`
- **Validation:** `grep -R "TSGAP" plans/index.aps.md plans/modules/anvil-ts-scanner-retirement.aps.md plans/modules/scanner-adjacent-ts-retirement.aps.md`
- **Confidence:** high
- **Priority:** High
- **Status:** Ready

---

### TSGAP-002: Clean core package exports

- **Intent:** Remove stale public subpath exports for archived source trees or
  replace them with active compatibility stubs.
- **Expected Outcome:** `@eddacraft/anvil-core` package exports match active
  source and build output; `./antipattern`, `./suppression`, and `./drift` do
  not resolve to missing files unless intentional compatibility modules exist.
- **Files:** `packages/anvil/core/package.json`,
  `packages/anvil/core/src/index.ts`, `packages/anvil/core/src/warnings/`
- **Validation:** `pnpm --filter @eddacraft/anvil-core build && pnpm --filter @eddacraft/anvil-core test`
- **Confidence:** high
- **Priority:** High
- **Status:** Ready

---

### TSGAP-003: Decouple `.anvil` compiler namespace

- **Intent:** Keep the `.anvil` compiler active without keeping it under the
  archived scanner namespace.
- **Expected Outcome:** `patterns:compile` and `patterns:check` use an active
  compiler path whose ownership is documented separately from TS scanner
  runtime.
- **Files:** `packages/anvil/core/src/antipattern/format/`,
  `packages/anvil/core/scripts/compile-patterns.ts`,
  `packages/anvil/core/package.json`, `docs/guides/anvil-rule-authoring.md`
- **Validation:** `pnpm --filter @eddacraft/anvil-core patterns:check`
- **Confidence:** high
- **Priority:** High
- **Status:** Ready

---

### TSGAP-004: Settle drift capability ownership

- **Intent:** Give drift snapshot/compare/reporting an active owner after the TS
  core drift API archive.
- **Expected Outcome:** Drift is either fully owned by Rust CLI/daemon APIs, or
  docs and package exports explicitly state the TS API is retired with no active
  replacement.
- **Files:** `crates/anvil-cli/src/commands/drift.rs`,
  `packages/anvil/core/src/index.ts`, `packages/anvil/core/package.json`,
  `archive/anvil-ts-scanner/core-drift/`, `docs/`
- **Validation:** `cargo test -p anvil-cli -- drift`
- **Confidence:** medium
- **Priority:** High
- **Status:** Ready

---

### TSGAP-005: Settle constraint export ownership

- **Intent:** Keep `llms.txt`, MCP-resource, and prompt-fragment export
  behaviour available without the archived TS runtime export pipeline.
- **Expected Outcome:** Constraint export is either CLI-only with documented
  Rust ownership, or a new active API replaces the archived TS collector and
  formatters.
- **Files:** `crates/anvil-cli/src/commands/export.rs`,
  `archive/anvil-ts-scanner/runtime-export/`,
  `plans/archive/modules/llms-txt-export.aps.md`, `docs/`
- **Validation:** `cargo test -p anvil-cli -- export`
- **Confidence:** medium
- **Priority:** High
- **Status:** Ready

---

### TSGAP-006: Settle persistent suppression ownership

- **Intent:** Decide where `.anvil/suppressions.json` store/service semantics
  live after the TS suppression stack archive.
- **Expected Outcome:** Active surfaces have a documented and tested path for
  reading active suppressions; expired suppression handling is explicit.
- **Files:** `crates/anvil-cli/src/commands/export.rs`,
  `crates/anvil-checks/src/antipattern/scanner.rs`,
  `archive/anvil-ts-scanner/core-suppression/`, `docs/`
- **Validation:** `cargo test -p anvil-cli -- export`
- **Confidence:** medium
- **Priority:** High
- **Status:** Ready

---

### TSGAP-007: Restore anti-pattern explanations

- **Intent:** Reintroduce AP-* explanation capability from the Rust catalogue or
  explicitly retire it until RCLI3-015 lands.
- **Expected Outcome:** `anvil explain AP-001` has an active source of
  explanation content, or docs state that AP-* explanations are intentionally
  unavailable until the Rust explain command is implemented.
- **Files:** `packages/anvil/core/src/explain/`,
  `archive/anvil-ts-scanner/core-explain-antipattern.ts`,
  `crates/anvil-cli/src/commands/explain.rs`,
  `plans/modules/rust-cli-tier3.aps.md`, `docs/`
- **Validation:** `grep -R "core-explain-antipattern\|RCLI3-015" plans docs packages crates --include="*.md" --include="*.ts" --include="*.rs"`
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Ready

---

### TSGAP-008: Define MCP resource and prompt return path

- **Intent:** Ensure archived TS export/resource shapes do not become the hidden
  contract for future MCP parity work.
- **Expected Outcome:** RMCPF references Rust-owned constraint, drift,
  suppression, and warning contracts for resources/prompts, using the archive
  only as historical compatibility evidence.
- **Files:** `plans/modules/rust-mcp-full-port.aps.md`,
  `archive/anvil-ts-scanner/runtime-export/`,
  `crates/anvil-cli/src/commands/mcp.rs`, `docs/`
- **Validation:** `grep -R "runtime-export\|constraints" plans/modules/rust-mcp-full-port.aps.md docs --include="*.md"`
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Ready

---

### TSGAP-009: Execute final capability audit

- **Intent:** Prove the post-TSRET active surface is internally consistent.
- **Expected Outcome:** Active package exports, docs, CLI commands, and APS rows
  agree on which scanner-adjacent capabilities exist, which are CLI-only, and
  which are intentionally retired.
- **Files:** `packages/`, `apps/`, `crates/`, `docs/`, `plans/`,
  `archive/anvil-ts-scanner/`
- **Dependencies:** TSGAP-002, TSGAP-003, TSGAP-004, TSGAP-005, TSGAP-006,
  TSGAP-007, TSGAP-008
- **Validation:** `pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test && cargo test --workspace`
- **Confidence:** medium
- **Priority:** High
- **Status:** Ready

## Risks

- **Completed TSRET masks missing capabilities.** The archive is complete, but
  users may still expect drift/export/explain package APIs that no longer ship.
- **CLI-only replacement is not API parity.** Rust `drift` and `export` commands
  exist, but TS package APIs may still be expected by active code or downstream
  consumers.
- **Compiler namespace drift.** The active `.anvil` compiler still lives under
  `antipattern/format`, which can confuse ownership after scanner retirement.
- **MCP contract drift.** Future MCP resource work may accidentally treat
  archived TS formatters as active contracts instead of compatibility evidence.

## Milestones

- **M1:** TSGAP is registered as the post-TSRET remediation owner.
- **M2:** Core package exports and compiler ownership are consistent.
- **M3:** Drift, export, suppression, and explain capability decisions are
  implemented.
- **M4:** MCP return path and final audit confirm no hidden archive dependency.
