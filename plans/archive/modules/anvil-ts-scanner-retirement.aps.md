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

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| TSRET | —     | Complete    | 3/3 active (3 superseded — terminal state reached on `chore/TSRET-005`) |

> **Post-close gap audit (2026-05-02):** TSRET-005 completed the archive, but
> several scanner-adjacent capabilities now need explicit active ownership or
> retirement decisions: stale `@eddacraft/anvil-core` subpath exports,
> `.anvil` compiler namespace ownership, drift snapshot/reporting, constraint
> export APIs, persistent suppression store/service semantics, AP-* explain
> content, and MCP resource/prompt return paths. These are tracked in
> [TSGAP](./scanner-adjacent-ts-retirement.aps.md); TSRET remains Complete.
>
> **Plan change (2026-04-29, [ADR-033](../../decisions/033-park-ide-mcp-retire-ts-scanner.md)):**
> The IDE/MCP surfaces this module's TS code exists for are
> **archived** — the VSCode extension is now at
> `archive/anvil-vscode-extension/` and the TS MCP server is at
> `archive/anvil-mcp-server/`. TSRET-005 (archive the TS scanner +
> retire parity harness) **unblocks immediately** under ADR-033 — its
> dependencies on DRVR-003/-004 are dissolved because no active
> package needs the TS scanner. TSRET-006 (transition-window
> engine-version diagnostics) is **superseded**: there is no
> transition window to bridge once both engines collapse to one.
> The napi crate (`crates/anvil-checks-napi/`) stays `"private":
> true` and remains in CI as a build canary; it is no longer the
> on-ramp for any active surface. With TSRET-005 executed, this
> module reaches its terminal state — RMCP/RMCPF and DRVR own the
> return path for surfaces.
>
> **Plan change (2026-04-23, ADR-030):** TSRET-003 and TSRET-004 are
> superseded by the `surface-drivers` module (**DRVR**). Both consumers
> cut over to drivers on the anvil-intercept daemon rather than
> embedding a napi binding in-process. TSRET-002's remaining residual
> is narrowed: the napi crate stays `"private": true`; no npm publish,
> no provenance decision, no OOB install-test requirement. TSRET-005
> is retained but now blocks on DRVR rather than on TSRET-003/-004.
> *(ADR-033 supersedes the "blocks on DRVR" half — see above.)*

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

**Implementation path (post-ADR-030):** The original plan retired the
TS scanner by binding the Rust engine into Node via `napi-rs` and
swapping the import in each consumer. ADR-030 supersedes that: both
consumers become drivers on the `anvil-intercept` daemon (module
DRVR), and the TS scanner is deleted once no surface imports it
(TSRET-005, rewired to depend on DRVR-003/-004). The napi crate
remains as an internal acceleration path for the CLI only; no npm
publication, no per-consumer embedding.

Authoritative ADRs: [ADR-026](../../decisions/026-rust-scanner-authoritative.md)
(Rust scanner authoritative) and
[ADR-030](../../decisions/030-surface-drivers-supersede-napi-cutover.md)
(surface drivers supersede napi cutover).

## Background

- The `packages/anvil/core/src/antipattern/` scanner is a full TS
  implementation of the registry-driven scan loop.
- The VSCode extension at `archive/anvil-vscode-extension/` imports it for
  in-editor diagnostics.
- The MCP server at `archive/anvil-mcp-server/` imports it for tool calls.
- `napi-rs` was the originally-planned bridge. TSRET-001 landed the
  spike and TSRET-002 landed the prebuild matrix; both are retained,
  but per ADR-030 the binding now serves the CLI only (not editor or
  MCP consumers). The `"private": true` crate flag stays and no npm
  publication is planned.
- Dependency: SPG must land first, or at minimum SPG-001..003, so the
  Rust engine actually runs every default rule before the TS path is
  removed. SPG is now Complete (6/6), so this dependency is satisfied.

## Scope

**In scope:**

- `napi-rs` binding crate exposing `scan_artifact_json` and pattern
  registry getters — *retained* as CLI-internal acceleration (TSRET-001
  Complete, TSRET-002 scope-reduced; see the per-work-item notes below).
- Deleting `packages/anvil/core/src/antipattern/scanner.ts`,
  `registry-loader.ts`, and supporting TS files (TSRET-005, now
  blocks on DRVR-003 / DRVR-004).
- Retiring the TS half of `tests/scanner-parity/`.
- Documentation updates reflecting the now-single-engine state.

**Moved to DRVR (per ADR-030):**

- Cutting VSCode and MCP over to a Rust-backed scanner. Those
  consumers become drivers on the intercept daemon; see
  `plans/modules/surface-drivers.aps.md`.

**Out of scope:**

- Further scanner feature work (lives in SPG or a successor module).
- Replacing other TS engine code beyond the antipattern scanner
  (e.g. gate runner, provenance) — separate modules.

## Interfaces

**Depends on:**

- `anvil-scanner-parity-gaps` (SPG, Complete — dependency satisfied).
- `crates/anvil-checks/` — Rust scanner surface used by the napi
  binding and by the daemon.
- ~~**DRVR-003 and DRVR-004** — TSRET-005 (delete TS scanner) waits for
  these before executing, per ADR-030.~~ *(Dependency dissolved by
  ADR-033 — IDE/MCP surfaces are archived; TSRET-005 executes against
  the archived-surfaces state.)*
- `packages/anvil/core/src/antipattern/` — being retired.
- `packages/anvil/core/src/suppression/parser.ts` — being retired
  with the scanner under ADR-033.

**Exposes:**

- A single authoritative scanner, reached by every surface — the
  intercept daemon hosts it per ADR-030 (not a published binding).
- `crates/anvil-checks-napi/` — internal CLI-acceleration binding.
  Retained private (`"private": true` in its `package.json`); not
  published to npm. See ADR-030 and council review S4.

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
- **Status:** Complete (2026-04-23, scope-reduced per ADR-030)
- **Scope after ADR-030:** The daemon is the runtime bundling point;
  npm publication of `@eddacraft/anvil-checks-native` is not required
  to complete this module. The CI matrix + test-without-Rust-toolchain
  path remains valuable as a canary on the binding, so the workflow
  stays. The following are **not planned**:
  - `"private": true` stays; no `NPM_TOKEN` wiring needed.
  - OOB install smoke tests for `aarch64-unknown-linux-gnu` and
    `x86_64-apple-darwin` are not planned — the binding is
    internal-only.
  - `--provenance` / `id-token: write` decision is not planned —
    there is no publish to provenance.
- **Closeout evidence:** `napi.yml` workflow ran green on every
  commit of PR #1060 (run IDs 24836269971, 24838233352, 24838796988
  — last one 9m15s on the final head, all five build targets plus
  three test runners passing without a Rust toolchain installed).
  Plan-change ADR-030 merged as commit `0be1bcf7`. Follow-up
  maintenance of the CI matrix (keeping it green on dependency
  bumps) is ongoing hygiene, not TSRET-002 work.

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

### TSRET-005: Archive the TS scanner and retire the parity harness

- **Intent:** One engine. One implementation. One place to change.
- **Expected Outcome:** Following the project's archive convention
  (precedent: `archive/anvil-cli-node/` from ADR-012,
  `archive/anvil-tui-ink/` from ADR-011a), the TS engine code moves
  to `archive/anvil-ts-scanner/`:
  - `packages/anvil/core/src/antipattern/` →
    `archive/anvil-ts-scanner/antipattern/`
  - `packages/anvil/core/src/suppression/parser.ts` →
    `archive/anvil-ts-scanner/suppression/parser.ts`
  - `tests/scanner-parity/` (TS side) →
    `archive/anvil-ts-scanner/scanner-parity/`

  The Rust-side parity test
  (`crates/anvil-checks/tests/scanner_parity.rs`) and the root
  `test:scanner-parity` script are **deleted** — there is no
  second engine to compare against, so the test has no meaning.
  Inbound imports from active packages
  (`@eddacraft/anvil-core/antipattern`,
  `@eddacraft/anvil-core/suppression`) are removed; any lingering
  type-only exports that other code depends on are moved to
  `@eddacraft/anvil-contracts`. `docs/archive/architecture/rust-architecture-endstate.md`
  and `docs/guides/anvil-rule-authoring.md` updated to describe
  the single-engine state.
- **Scope:** `packages/anvil/core/src/antipattern/`,
  `packages/anvil/core/src/suppression/parser.ts`,
  `crates/anvil-checks/tests/scanner_parity.rs`,
  `tests/scanner-parity/`, `archive/anvil-ts-scanner/` (new),
  inbound imports across `packages/`, `package.json`, docs
- **Dependencies:** ~~DRVR-003, DRVR-004 (per ADR-030 supersession)~~
  — dependencies dissolved by ADR-033; SPG-003 (already Complete) is
  the only remaining prerequisite, and it is satisfied
- **Validation:** `pnpm test` passes with scanner-parity suite
  removed; `grep -r "scanner-parity" docs/` returns nothing
  load-bearing; VSCode-extension and mcp-server packages still build
  *(if not yet archived)* or are excluded from the test sweep *(if
  archived per ADR-033)*
- **Confidence:** medium
- **Priority:** High
- **Status:** Complete (2026-04-29 on `chore/TSRET-005`)
- **Closeout (2026-04-29):** Executed under ADR-033 alongside the
  IDE/MCP archive. The cascade through inbound consumers landed
  larger than the literal ADR-033 scope: drift detection
  (`packages/anvil/core/src/drift/`) and the antipattern explainer
  (`packages/anvil/core/src/explain/antipattern-explainer.ts`)
  archived because they were end-to-end coupled to the archived
  engine code. The TS gate runner
  (`packages/anvil/runtime/src/gate/`), constraint collector, and
  formatters archived for the same reason. A minimal `Warning`
  type was extracted to `packages/anvil/core/src/warnings/types.ts`
  so active consumers (`warnings/warning-id`,
  `explain/explain-service` boundary surface) keep a typed handle.
  The Rust-side parity test
  (`crates/anvil-checks/tests/scanner_parity.rs`) and the root
  `test:scanner-parity` script were deleted. ADR-033's "Code
  archived" section was updated to record the full cascade.

---

### TSRET-006: Engine-version diagnostics + transition-window divergence canary — **Superseded**

- **Status:** Superseded by ADR-033 (2026-04-28).
- **Why:** This work item existed to monitor a TS-to-daemon
  transition window — the period where rules added to the compiled
  registry after TSRET-002 but before DRVR-003 would be invisible to
  TS-scanner surfaces (VSCode + MCP via `GateRunner`). Under
  ADR-033 those surfaces are archived and the TS scanner is retired
  in TSRET-005, so the window collapses to zero and the canary has
  no second engine to compare against. No follow-up is required.
- **Original Intent:** Close the monitoring gap in the TS-to-daemon transition
  window. Rules added to the compiled registry after TSRET-002 but
  before DRVR-003 are invisible to TS-scanner surfaces
  (VSCode + MCP via `GateRunner`) with no attribution path. A user
  reporting "Anvil flagged this wrong at 14:22" currently cannot be
  told which engine (TS or Rust) served the warning.
- **Expected Outcome:** Every diagnostic / scan result carries an
  `engineVersion` field derived from the compiled registry's
  `compiled_at` timestamp (or equivalent). Both the TS scanner path
  (in `packages/anvil/core/src/antipattern/`) and the Rust scanner
  path (via the napi binding and eventually the daemon) populate
  the field. A CI canary job runs a small repo-hosted fixture set
  through both engines and fails if warning counts or ids diverge
  beyond a recorded tolerance — turning silent drift into a CI
  signal. Fixture set uses the repo's own `.anvil` sources so new
  rules are covered automatically.
- **Scope:** `packages/anvil/core/src/antipattern/` (add the field,
  with a fallback when registry metadata is absent),
  `crates/anvil-checks/src/antipattern/` (same on Rust side),
  `crates/anvil-checks-napi/src/lib.rs` (surface the field),
  `tests/scanner-parity/` (extend fixture set + canary script),
  `.github/workflows/ci.yml` (wire the canary into PR CI)
- **Dependencies:** SPG-003 (Complete), TSRET-001 (Complete)
- **Validation:** Canary fixture run returns structured output
  containing `engineVersion` on both sides; deliberate rule-divergence
  test (add a rule to Rust registry without the TS loader mirror)
  fails the canary loudly.
- **Source:** 2026-04-24 council review M14 (operations-reviewer) —
  tracked in PR #1063.
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Proposed

## Risks

- **Transitive consumers of the TS module.** Any code that imports
  `@eddacraft/anvil-core/antipattern` outside the extension/MCP must
  be inventoried before TSRET-005 runs; some may need a TS-side
  adapter rather than a straight import swap.

  *(Vacated risks — napi-rs overhead on the VSCode hot path, and
  Windows arm64 / niche-target prebuilds — were removed 2026-04-24
  per ADR-030. VSCode no longer goes through napi; the napi crate is
  internal-only and does not ship to npm. See council review S2.)*

## Milestones

- **M1 (TSRET-001, TSRET-002 — Complete):** napi binding exists and
  installs without a Rust toolchain on every target platform; CI
  matrix green. Under ADR-030 the binding is an internal CLI
  acceleration path, not a published consumer surface; under
  ADR-033 it stays as a build canary with no active consumer
  during the IDE/MCP pause.
- ~~**M2 (TSRET-006):** transition-window divergence canary green~~
  *(Superseded by ADR-033 — no transition window remains.)*
- **M2 / Terminal (TSRET-005, under ADR-033):** TS scanner +
  TS suppression parser deleted; parity harness retired
  (both sides); single-engine state documented in
  `docs/archive/architecture/rust-architecture-endstate.md` and
  `docs/guides/anvil-rule-authoring.md`.

  *(Pre-ADR-033 wording: "M3 (TSRET-005, via DRVR-003 / DRVR-004)" —
  the DRVR dependency was dissolved by ADR-033.)*

  *(The old M2 — "VSCode + MCP run on the Rust engine via napi" —
  was superseded by ADR-030. See council review S3.)*

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
