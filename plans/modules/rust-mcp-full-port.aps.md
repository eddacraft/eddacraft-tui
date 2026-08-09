# Rust MCP Full Port

| ID    | Owner | Status | Progress |
| ----- | ----- | ------ | -------- |
| RMCPF | —     | In Progress | 7/15     |

**Last reviewed:** 2026-08-09

> **Plan change (2026-04-29, [ADR-033](../decisions/033-park-ide-mcp-retire-ts-scanner.md)):**
> The TypeScript MCP server (`anvil-archive/anvil-mcp-server/`) is now
> **archived** — moved out of `packages/mcp-server/` and then moved to
> the sibling `eddacraft/anvil-archive` repository. RMCPF still owns full Rust-side
> parity, but its starting state changes:
>
> - Inventory work (RMCPF-001) reads the archived TS package as a
>   frozen reference, not a moving target.
> - Tool / resource / prompt ports (RMCPF-010..-020) replace
>   functionality nobody is shipping today (the launch path runs
>   through RMCP); the parity bar becomes "match the historical TS
>   contract" rather than "match the live TS server".
> - Compatibility harness (RMCPF-030) compares Rust output against
>   the archived TS server treated as a fixture, not against an
>   actively maintained sidecar.
> - Decision #4 below ("TS server is temporary compatibility
>   source") is amended: under ADR-033 the TS server is archived
>   reference material, not a parallel runtime carried through the
>   port.
> - RMCPF-031 ("Retire or archive TypeScript MCP server") is
>   partially executed by ADR-033 — the package is already in
>   `anvil-archive/anvil-mcp-server/`. RMCPF-031 closes out by deciding
>   whether the archive remains as historical reference or is
>   deleted once parity ships.

## Purpose

After the current release proves the narrow Rust MCP launch shim, port the
existing TypeScript MCP server functionality into the Rust `anvil` binary so MCP
is no longer split across a launch-critical Rust path and a legacy Node/TS
sidecar.

**Why:** RMCP deliberately does not port `anvil-archive/anvil-mcp-server`. It ships only
the A1 pre-write validation path. The next release can then do the slower,
parity-focused work: existing tools, resources, prompts, transports, tests, and
compatibility behaviour move behind the Rust binary without jeopardising the
launch demo. Per ADR-033 the TS server is **archived**, so RMCPF's parity work
runs against a frozen reference — not a moving sidecar.

## In Scope

- Inventory and compatibility matrix for all current `anvil-archive/anvil-mcp-server`
  tools, resources, prompts, and transports
- Rust implementations of existing MCP tool contracts
- Rust implementations of existing MCP resource contracts where still needed
- Rust implementations or documented retirement of existing prompts
- Streamable HTTP transport parity if still required by supported clients
- Compatibility tests comparing TS and Rust MCP responses on fixture workspaces
- Deprecation/retirement path for `anvil-archive/anvil-mcp-server` once parity is reached
- Migration documentation for users who previously launched the TS MCP server

## Out of Scope

- Blocking the current release on full MCP parity
- Re-opening RMCP's launch-shim scope
- Creating new graph-context tools beyond what GCTX explicitly owns
- Replacing the daemon or driver framework
- Adding non-MCP editor integrations
- MCP `2026-07-28` dual-era protocol host, discovery, modern result envelopes,
  and activation-probe changes — owned by
  [MCP26](mcp-dual-era-support.aps.md)

## Interfaces

**Depends on:**

- RMCP — current-release Rust MCP stdio launch path
- DRVR / ADR-030 — driver and daemon direction for integration surfaces
- GV2/GCTX where graph context tools are introduced
- `anvil-archive/anvil-mcp-server` — compatibility source of truth until retirement
- `crates/anvil-cli` — Rust command host

**Exposes:**

- Full Rust MCP server surface under `anvil mcp serve`
- Migration plan for retiring or archiving `anvil-archive/anvil-mcp-server`
- Compatibility report for existing MCP users

## Constraints

- UK English spelling in all plan text and user-facing docs
- Existing MCP contracts must either be preserved or explicitly retired with a
  migration note
- The Rust server must preserve the single-binary story established by RMCP
- Feature parity claims require fixture-backed tests, not manual spot checks
- Graph-context features depend on GV2/GCTX and must not become accidental scope
  in this module

## Work Items

### Phase 1 Readiness Checklist

RMCPF has started because RMCPF-001 and RMCPF-002 are complete. Before claiming
the implementation phases are ready to run, close the remaining client and
retirement decisions:

- [x] RMCP has shipped or reached Committed state
- [x] Existing TS MCP server inventory is complete
- [x] Supported-client matrix for Claude Code, Cursor, Continue, VSCode, and
  any remaining HTTP clients is confirmed
- [x] Decision recorded on whether Streamable HTTP remains required
- [x] Retirement criteria for `archive/anvil-mcp-server` agreed

---

### Phase 0 — Inventory and Compatibility

#### RMCPF-001: Existing MCP surface inventory

- **Status:** Done
- **Intent:** Create the compatibility matrix for every current TS MCP tool,
  resource, prompt, and transport.
- **Expected Outcome:** Inventory records contract, implementation owner,
  current tests, supported clients, and disposition: port, retire, or defer.
- **Validation:** Inventory reviewed against `archive/anvil-mcp-server/src/`,
  package manifest, Vitest config, and archived test files.
- **Validation Evidence:** Validation passed: `pnpm docs:check` on 2026-05-13.
  The inventory covers archived tools, resources, prompts, transports, client
  config targets, tests, dispositions, and open follow-on decisions. Source
  review covered `archive/anvil-mcp-server/src/**`,
  `archive/anvil-mcp-server/package.json`, and
  `archive/anvil-mcp-server/vitest.config.ts`.
- **Files:** `plans/specs/rust-mcp-full-port-inventory.md`,
  `archive/anvil-mcp-server/src/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** RMCP

---

#### RMCPF-002: Rust MCP parity architecture spec

- **Status:** Done
- **Intent:** Define the post-launch Rust MCP architecture and how it relates to
  the daemon, driver framework, Graph v2, and legacy TS server.
- **Expected Outcome:** Spec defines command layout, protocol support,
  validation paths, resource serving, prompt strategy, transport support, and
  retirement gates for the TS package. The spec adopts the DRVR-006 resolution
  (see `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
  §4.3, recorded 2026-05-06 in A2 Wave 1) with one recorded RMCPF Phase 1
  amendment: `anvil_status` starts as MCP-driver-local workspace-health
  composition because no approved daemon `status.query` surface exists. The
  retained daemon-RPC translator set is `anvil_check` and `anvil_suppress`; the
  MCP-driver-local composition set is `anvil_status`, `anvil_fix`,
  `anvil_gate`, and `anvil_query_boundary`. The architecture spec MUST NOT
  introduce new daemon RPCs whose only consumer is parity prose; if RMCPF needs
  additional daemon authority it MUST file new INTD work items rather than
  expanding the RPC surface implicitly.
- **Validation:** Council review confirms the spec does not regress the RMCP
  single-binary launch path, and confirms each tool's class matches the
  DRVR-006 table above (or records a deliberate amendment with rationale).
- **Validation Evidence:** Validation passed: Quick Council review completed on
  2026-05-14 with no findings after the dependency, protocol-method,
  architecture-index, and review-date fixes.
- **Files:** `docs/architecture/rust-mcp-server-spec.md`
- **Closeout evidence:** Spec added in
  `docs/architecture/rust-mcp-server-spec.md` on 2026-05-14. It preserves the
  RMCP `anvil_validate_write` path, adopts DRVR-006 option (b) classifications
  with the RMCPF Phase 1 `anvil_status` local-composition amendment, names
  DRVR-007 redaction as transport-wide, and records retirement gates for the
  archived TypeScript MCP package.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** DRVR-006, DRVR-007, ADR-033, RMCP,
  `archive/anvil-mcp-server/src/` frozen reference. RMCPF-001 remains the
  detailed compatibility inventory input for implementation items, but RMCPF-002
  is intentionally closed first to give that inventory a stable architecture
  taxonomy.

---

#### RMCPF-003: Phase 1 readiness decision closure

- **Status:** Done
- **Intent:** Close the remaining client, transport, and TypeScript archive
  retirement decisions before RMCPF Phase 1 tool parity starts.
- **Expected Outcome:** Supported-client matrix is confirmed; Streamable HTTP is
  retained or retired with rationale; retirement criteria for
  `archive/anvil-mcp-server` are agreed and reflected in the RMCPF readiness
  checklist.
- **Validation:** `pnpm docs:check`
- **Validation Evidence:** Validation passed: `pnpm docs:check` on 2026-05-14
  with 7/7 documentation surfaces passing and no failed checks.
- **Files:** `plans/modules/rust-mcp-full-port.aps.md`,
  `plans/specs/rust-mcp-full-port-inventory.md`,
  `docs/architecture/rust-mcp-server-spec.md`, `RELEASE-PLAN.md`
- **Closeout evidence:** Phase 0 decision closure recorded on 2026-05-14. Claude
  Code and Cursor are the first supported parity clients because both have
  archived TS config evidence, active Rust installer/config tests, and current
  public docs. Continue, VS Code, and Windsurf are deferred until fresh support
  evidence exists. Stdio is the only required Phase 1 transport; Streamable HTTP
  stays deferred under RMCPF-021 and should be retired unless a supported-client
  requirement appears before implementation. `archive/anvil-mcp-server` remains
  frozen reference material until RMCPF-031 and can retire only after retained
  surfaces have Rust parity or documented retirements, compatibility evidence,
  generated-config cutover, and migration docs.
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** RMCPF-001, RMCPF-002

---

### Phase 1 — Tool Parity

#### RMCPF-010: Port check/gate/status tools

- **Status:** Done
- **Intent:** Move the core read-only validation tools from the TS MCP server to
  Rust while preserving response contracts or documenting intentional changes.
- **Expected Outcome:** Rust MCP server exposes parity for `anvil_check`,
  `anvil_gate`, and `anvil_status` or their explicitly versioned successors.
  Per the DRVR-006 resolution
  (`plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md` §4.3)
  plus the RMCPF Phase 1 status amendment recorded in RMCPF-002:
  - `anvil_check` is a **daemon-RPC translator**. Its handler calls the daemon's
    `scan.files` / `scan_buffer` surfaces (with the embedded fallback when the
    daemon is unavailable, as RMCP-005 already wires).
  - `anvil_status` is **MCP-driver-local composition** for Phase 1 because no
    approved daemon `status.query` surface exists. It reads only validated
    workspace-local data and returns explicit local/no-daemon provenance.
  - `anvil_gate` is **MCP-driver-local composition**: the handler shells to
    `anvil gate` (or invokes the equivalent in-process gate path) because
    `GateRunner` runs `npm audit`, OPA, and coverage JSON reads that the
    daemon deliberately does not do.
  - All response payloads pass through the redaction contract recorded in
    §4 of the design spec (DRVR-007) before leaving the MCP transport.
- **Validation:** Compatibility tests compare TS and Rust responses on fixture
  workspaces; tests assert the DRVR-006 classification (e.g. `anvil_gate`
  results are reachable when the daemon is offline, `anvil_check` falls back
  to embedded scan). Initial registry/status slice validates with
  `cargo test -p eddacraft-anvil --test mcp_serve_stdio` and preserves the
  existing `anvil_validate_write` observable behaviour while adding
  `anvil_status` list/call coverage.
- **Validation Evidence:** Registry and first parity-tool slice validated on
  2026-05-14 with targeted green tests:
  `cargo test -p eddacraft-anvil --test mcp_serve_stdio mcp_serve_stdio_tools_call_status_returns_workspace_health_summary`,
  `cargo test -p eddacraft-anvil --test mcp_serve_stdio mcp_serve_stdio_tools_list_returns_registered_tools`,
  `cargo test -p eddacraft-anvil --test mcp_serve_stdio mcp_serve_stdio_tools_call_status_rejects_workspace_outside_server_root`,
  `cargo test -p eddacraft-anvil mcp::tools::status::tests`, and
  `cargo test -p eddacraft-anvil mcp::tools::registry::tests`. The full
  `cargo test -p eddacraft-anvil --test mcp_serve_stdio` integration suite also
  passed before PR closeout. The check/gate slice was validated on 2026-05-14
  with `cargo test -p eddacraft-anvil --bin anvil mcp::tools::check::tests`,
  `cargo test -p eddacraft-anvil --bin anvil mcp::tools::gate::tests`, an
  updated `cargo test -p eddacraft-anvil --bin anvil mcp::tools::registry::tests`
  (four tools registered), and the full
  `cargo test -p eddacraft-anvil --test mcp_serve_stdio` integration suite
  (19/19 green including the new
  `mcp_serve_stdio_tools_call_check_returns_clean_payload_for_clean_files`,
  `mcp_serve_stdio_tools_call_check_rejects_workspace_outside_server_root`, and
  `mcp_serve_stdio_tools_call_gate_planless_mode_scans_target_files` cases).
  `cargo fmt --all -- --check` and
  `cargo clippy -p eddacraft-anvil --all-targets` both ran clean after the
  slice.
- **Closeout evidence:** The Rust MCP tool registry now dispatches four tools
  (`anvil_validate_write`, `anvil_status`, `anvil_check`, `anvil_gate`) without
  changing `tools/call` protocol handling. `anvil_status` is implemented as an
  unauthenticated read-only local status tool for this slice because no Rust
  daemon `status.query` MCP surface exists yet; its response keeps the archived
  TS field names (`status`, `workspaceRoot`, `availableChecks`, `config`,
  `hasBaseline`, `version`) while redacting path values to workspace-relative
  forms and adding explicit `backend: "local"` / `daemonStatus: "not-wired"`
  provenance for future daemon replacement. The workspace root is canonicalised
  and rejected when it resolves outside the MCP server root before any
  filesystem reads, and `hasBaseline` reads the archived architecture baseline
  source (`.anvil/architecture.json`).
  `anvil_check` is implemented as the daemon-RPC translator's
  correctness-equivalent embedded fallback because no daemon `scan.files`
  surface exists yet: it validates workspaceRoot containment under the MCP
  server root, rejects absolute and `..`-escaping file entries, canonicalises
  every workspace-relative path and re-verifies it stays inside the workspace
  before reading (per Council review — closes the symlink-escape vector where a
  workspace-relative entry pointing at a symlink targeting `/etc/passwd` would
  otherwise be read), runs
  `anvil_checks::antipattern::run_antipattern_check` in process against
  workspace-relative paths, and returns the parity payload (`warnings`,
  `summary`, `executionTimeMs`, `checksRun: ["antipattern"]`,
  `hasBlockingWarnings`, redacted `workspaceRoot`, `backend: "local"`,
  `daemonStatus: "not-wired"`). When INTD lands `scan.files`, this handler
  flips to the daemon-RPC translator path without an MCP contract change.
  `anvil_gate` is MCP-driver-local composition: planless mode (`targetFiles`
  supplied) runs the antipattern scanner in process with the same redaction,
  symlink-resolution, and provenance contract as `anvil_check`; full mode (no
  `targetFiles`) shells `current_exe --no-tui --json gate` from the workspace
  root with a 2-minute timeout, optional `--skip-checks` and `--fail-fast`
  flags, parses the gate JSON envelope, and returns a reshaped payload
  (`mode`, `overall`, `score`, `checks`, `executionTimeMs`, `exitCode`,
  redacted `workspaceRoot`, `backend: "local"`, `daemonStatus: "not-wired"`).
  Both tools reject workspace roots outside the MCP server root before any
  filesystem or subprocess work, cap file-array length at 10 000 entries,
  reject comma-bearing or NUL-byte-bearing skip-check entries before any
  subprocess spawn, and cap subprocess stdout/stderr capture at 16 MiB so a
  pathological gate plugin cannot exhaust the MCP server's memory before the
  timeout fires. Shared workspace/redaction/warning helpers live in
  `crates/anvil-cli/src/mcp/tools/shared.rs`; `status`, `check`, and `gate`
  all consume the same canonicalisation and containment logic so a future
  hardening tweak lands once, not three times.
- **Files:** `crates/anvil-cli/src/mcp/tools/`,
  `archive/anvil-mcp-server/src/tools/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** RMCPF-002

---

#### RMCPF-011: Port fix/suppress/boundary tools

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Intent:** Move mutation and architecture-query tools to Rust with safe
  validation and redaction boundaries.
- **Expected Outcome:** Rust MCP server exposes parity for `anvil_fix`,
  `anvil_suppress`, and `anvil_query_boundary` or documented successors.
- **Validation:** Compatibility tests cover success, failure, workspace escape,
  and dry-run cases. The new tools sit beside `anvil_check`/`anvil_gate` in
  the Rust registry; `cargo test -p eddacraft-anvil --bin anvil mcp::tools`
  and `cargo test -p eddacraft-anvil --test mcp_serve_stdio` pass with all
  three new tools wired in. Unit tests cover happy path, missing fields,
  workspace-outside-server-root, parent-dir escape, absolute-path rejection,
  blank/over-limit reasons, expiryDays clamp, symlink-target containment, and
  line-out-of-range. Integration tests cover `tools/list` exposing all seven
  tools, `tools/call` on each new tool, and the prompts capability negation
  (RMCPF-012). Closeout evidence is reproduced under §RMCPF-011 closeout
  evidence below.
- **Files:** `crates/anvil-cli/src/mcp/tools/query_boundary.rs`,
  `crates/anvil-cli/src/mcp/tools/suppress.rs`,
  `crates/anvil-cli/src/mcp/tools/fix.rs`,
  `crates/anvil-cli/src/mcp/tools/registry.rs`,
  `crates/anvil-cli/tests/mcp_serve_stdio.rs`,
  `crates/anvil-architecture/src/lib.rs` (re-export `assign_layers` +
  `BoundarySeverity` for the MCP query layer),
  `archive/anvil-mcp-server/src/tools/` (frozen reference).
- **Closeout evidence:** Rust MCP registry now dispatches seven tools
  (`anvil_validate_write`, `anvil_status`, `anvil_check`, `anvil_gate`,
  `anvil_query_boundary`, `anvil_suppress`, `anvil_fix`). All three RMCPF-011
  tools reuse the shared workspace-containment / redaction helpers from
  `crates/anvil-cli/src/mcp/tools/shared.rs` so a future hardening tweak lands
  in one place. `anvil_query_boundary` is MCP-driver-local composition: it
  reads `.anvil/architecture.json` through `anvil_architecture::load_baseline`
  and matches layers through `assign_layers` so the verdict aligns with
  `anvil check` for the same file pair. The handler covers the
  archived-TS reason set (`no-baseline`, `baseline-load-failed`,
  `unassigned-layer`, `same-layer`, `boundary-ok`, `boundary-violation`) and
  only treats Error-severity boundaries as blocking, so an explicit
  warning-level rule downgrades a default deny to `boundary-ok` rather than
  blocking. `anvil_suppress` ships as the daemon-RPC translator's
  correctness-equivalent embedded fallback because no INTD-owned
  `suppression.apply` exists yet; it validates workspace-relative
  containment, canonicalises symlink targets, sanitises CR/LF from `reason`,
  enforces a 512-byte reason cap and a 1–365 day `expiryDays` range, holds
  a same-process exclusive lock around the read-modify-write, and inserts
  `// @anvil-ignore-until YYYY-MM-DD <warningId>: <reason>` above the target
  line preserving its indent. The response carries
  `backend: "embedded"` / `daemonStatus: "not-wired"` so a future flip to
  daemon-authorised mutation does not change the wire shape. `anvil_fix` is
  MCP-driver-local composition: AP-001 / AP-003 / AP-004 are line-by-line
  deterministic transforms with the same string-literal/comment-aware
  character walker the archived TS tool used. Unknown warning IDs return a
  `fixed: false` payload with the supported-pattern list rather than an
  error so the LLM can fall back to manual editing. Both mutating tools
  reuse the embedded-fallback `backend` field so the MCP correlation envelope
  matches RMCPF-010.
- **Validation Evidence:** Validated on 2026-05-14 with
  `cargo test -p eddacraft-anvil --bin anvil mcp::tools` (95 unit tests
  green, including the 23 new tests across `query_boundary`, `suppress`, and
  `fix`) and
  `cargo test -p eddacraft-anvil --test mcp_serve_stdio` (24 integration
  tests green, including
  `mcp_serve_stdio_tools_list_returns_registered_tools` updated for the
  seven-tool registry,
  `mcp_serve_stdio_tools_call_query_boundary_returns_no_baseline_for_clean_workspace`,
  `mcp_serve_stdio_tools_call_suppress_inserts_comment_in_workspace_file`,
  and `mcp_serve_stdio_tools_call_fix_replaces_any_with_unknown`).
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RMCPF-010

---

#### RMCPF-012: Port or retire MCP prompts

- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Intent:** Decide whether existing TS MCP prompts should move to Rust or be
  retired in favour of docs and tool descriptions.
- **Expected Outcome:** Prompt parity exists where still useful; retired prompts
  have migration notes and tests updated accordingly.
- **Validation:** Prompt list from Rust matches the inventory disposition.
  Integration tests pin both the capability omission and the JSON-RPC
  `Method not found` for `prompts/list`.
- **Files:** `plans/specs/rust-mcp-full-port-inventory.md` (per-prompt
  retirement matrix and migration text),
  `crates/anvil-cli/tests/mcp_serve_stdio.rs` (capability + method assertions),
  `archive/anvil-mcp-server/src/prompts/` (frozen reference).
- **Closeout decision (2026-05-14):** All four archived prompts —
  `fix-violation`, `suppress-violation`, `architecture-review`,
  `pre-generation` — are **retired**. Rationale:
  - Phase 0 (RMCPF-003) confirmed Phase 1 clients (Claude Code, Cursor) do
    not depend on the archived prompts to call `anvil_check` /
    `anvil_gate` / `anvil_suppress`.
  - `docs/architecture/rust-mcp-server-spec.md` §"Prompt Strategy"
    explicitly warns that prompt content must not become a hidden
    architecture policy surface. Re-porting the prompts would re-introduce
    that hidden surface.
  - The new RMCPF-010 / RMCPF-011 tool descriptions already carry the
    actionable guidance (limitations, expected next call), so clients
    surface the same hints without a `prompts/get` round-trip.

  Enforcement landed in two places:
  - `initialize` `capabilities` omits `prompts`, so MCP clients negotiate
    without it. Pinned by
    `mcp_serve_stdio_initialize_does_not_advertise_prompts_capability`.
  - `prompts/list` returns JSON-RPC error `-32601 Method not found` rather
    than an empty list. Pinned by
    `mcp_serve_stdio_prompts_list_returns_method_not_found`.

  Per-prompt migration notes live in
  `plans/specs/rust-mcp-full-port-inventory.md` §Prompts and §"Prompts —
  RMCPF-012 disposition". RMCPF-030 must include the retirement in its
  compatibility matrix and migration docs. The archived TS prompts stay
  frozen under `archive/anvil-mcp-server/src/prompts/` until RMCPF-031.
- **Validation Evidence:** Validated on 2026-05-14 with
  `cargo test -p eddacraft-anvil --test mcp_serve_stdio` — the
  RMCPF-012 cases
  `mcp_serve_stdio_initialize_does_not_advertise_prompts_capability` and
  `mcp_serve_stdio_prompts_list_returns_method_not_found` are green
  (24/24 integration tests pass).
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RMCPF-001

---

### Phase 2 — Resources and Transports

#### RMCPF-020: Port MCP resources

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-19 via PR #2809 (readiness pass + GCTX-030
  reconciliation in the Readiness note below; seven `anvil://` resources ported +
  `file/{path}/warnings` retired; closeout and validation evidence below).
- **Readiness (2026-06-19):** GCTX-030 (Merged 2026-06-18 via #2772) already
  shipped the MCP **resources substrate** this item assumed it would build:
  the `resources` capability is advertised at `initialize`, and `resources/list`
  / `resources/read` are routed in `crates/anvil-cli/src/commands/mcp.rs`
  (`:248`–`:249`, `:374`, `:386`) to `crate::mcp::resources::{list,read}` in
  `crates/anvil-cli/src/mcp/resources/mod.rs`. RMCPF-020's scope therefore
  **narrows from "create a resources surface" to "extend the existing
  dispatch"** — add the eight `anvil://` descriptors to `resources::list()` and
  the eight URI arms to `resources::read()`, coexisting with the GCTX `graph://`
  scheme already served there. Source readers are all confirmed present:
  baseline (`crates/anvil-cli/src/activation/baseline.rs` /
  `anvil-architecture` baseline model), config (`crates/anvil-config`),
  patterns (`crates/anvil-checks/src/antipattern/check.rs`), suppressions
  (`crates/anvil-cli/src/services/suppressions.rs`), constraints
  (`crates/anvil-cli/src/commands/export.rs`), drift
  (`crates/anvil-cli/src/commands/drift.rs`), file-warnings (the `anvil_check`
  path). No open dependency remains.
- **Intent:** Move existing MCP resources to Rust or retire those that no longer
  fit the post-daemon architecture.
- **Expected Outcome:** Baseline, boundaries, patterns, suppressions, config,
  constraints, drift, and file-warning resources have Rust equivalents or
  documented retirement. The new `anvil://` resources are added to the existing
  GCTX-030 `resources::list`/`resources::read` dispatch (they do not introduce a
  second resources surface). Rust MCP resources must not treat the archived TS
  `runtime-export` collector as the active contract: `anvil://constraints` is
  sourced from the Rust CLI constraint exporter
  (`crates/anvil-cli/src/commands/export.rs`), drift resources are sourced from
  the Rust drift command/data model (`crates/anvil-cli/src/commands/drift.rs`),
  and suppressions are sourced from the same active `.anvil/suppressions.json`
  readers used by Rust CLI surfaces. Archived TS resource shapes are fixture
  evidence for compatibility review only.
- **Validation:** Resource listing and read tests match the inventory matrix,
  extending the existing `crates/anvil-cli/src/mcp/resources/tests.rs` unit
  coverage and the `crates/anvil-cli/tests/mcp_serve_stdio.rs` integration suite
  (`resources/list` exposes the `anvil://` set beside `graph://`; `resources/read`
  returns each ported resource's parity payload). `anvil://file/{path}/warnings`
  reuses the `anvil_check` workspace-containment + redaction contract.
- **Files:** `crates/anvil-cli/src/mcp/resources/` (extend `mod.rs` dispatch +
  `tests.rs`), `crates/anvil-cli/src/commands/mcp.rs` (no new routing needed —
  GCTX-030 already wired `resources/list`/`resources/read`),
  `archive/anvil-mcp-server/src/resources/` (frozen reference)
- **Confidence:** medium (substrate now exists via GCTX-030, lowering risk; the
  remaining work is eight per-resource read handlers over already-present Rust
  source readers)
- **Priority:** High
- **Dependencies:** RMCPF-002, GCTX-030 (resources dispatch substrate — Merged)
- **Closeout evidence (2026-06-19):** Seven state resources **ported** into a new
  `crate::mcp::resources::anvil` submodule and advertised in `resources/list`
  beside the GCTX `graph://` trio — `anvil://baseline`, `anvil://boundaries`,
  `anvil://patterns`, `anvil://suppressions`, `anvil://config`,
  `anvil://constraints`, `anvil://drift` — each sourced from its canonical Rust
  reader (`anvil_architecture::baseline`, `anvil_checks::antipattern::patterns`,
  `services::suppressions`, `anvil_config`, the `anvil export constraints`
  aggregator, and the `commands::drift` snapshot readers). The local-state
  `anvil://` resources are kept architecturally separate from the daemon-
  forwarding, CE-5-gated `graph://` egress resources, but reuse the parent
  module's URI/error helpers and the MCP tools' session-pinned-cwd root contract.
  `anvil://file/{path}/warnings` is **retired** — folded into the shipped
  `anvil_check` tool (the inventory-sanctioned disposition), so all eight archived
  resources are dispositioned (7 ported + 1 retired). Three private readers were
  widened to `pub(crate)` to keep one source of truth (no behaviour change):
  `export::collect_constraints`/`ConstraintData`, `drift::compare_snapshots`/
  `ComparisonOutput`, and a new `suppressions::load_suppressions_report`
  (active set + total/active/expired summary; `load_suppressions` now delegates).
  Streamable HTTP transport stays out of scope (RMCPF-021).
- **Validation Evidence:** `cargo test -p eddacraft-anvil --bin anvil
  mcp::resources` (13 unit) + `cargo test -p eddacraft-anvil --test
  mcp_serve_stdio resources` (10 integration, incl. the 7 new `anvil://` cases
  and the pre-existing graph cases) green on 2026-06-19; widened-module
  regressions clean (`commands::drift::tests` 27, `commands::export::tests` 36,
  `services::suppressions::tests` 3). `cargo clippy --workspace --all-targets --
  -D warnings` and `cargo fmt --all --check` both clean. Parity deltas vs the
  archived TS shapes (Rust-owned source models) are recorded in
  `plans/specs/rust-mcp-full-port-inventory.md` for the RMCPF-030 harness.
- **Council (2026-06-19):** Batch review (kernel-maintainer + adversarial). No
  correctness bugs; the drift comparison ordering and the `load_suppressions`
  delegation were confirmed equivalent. Egress findings **fixed in-PR**: absolute
  workspace paths are now redacted from `config`/`baseline`/`drift` in-band error
  messages and from `constraints` `metadata.workspace_root` (mirroring the MCP
  tools' redaction); the drift corrupt-second-snapshot path now still returns
  `latest`; unknown-URI is reported before query validation. The full-config
  egress of `anvil://config` is a **conscious decision** (documented in code) —
  it mirrors the archived contract and the `anvil config` surface, and `.anvil.*`
  is architecture/check config, not a secret store.
  **Deferred follow-up — filed as CIB-084:** read size/count caps for the shared
  `load_baseline` / drift-snapshot readers (a hostile local workspace with a
  huge or very-numerous
  snapshot/baseline file is an unbounded read) — best fixed at the reader level so
  `anvil drift`/baseline loading benefit too; out of scope for the MCP-resource
  port and pre-existing in the CLI readers.

---

#### RMCPF-021: Transport parity decision and implementation

- **Status:** Draft
- **Intent:** Decide whether the Rust MCP server needs Streamable HTTP parity in
  addition to stdio, then implement the accepted transport set.
- **Expected Outcome:** Supported transport matrix is documented. If HTTP remains
  supported, `anvil mcp serve --http` reaches parity with the TS transport for
  supported clients.
- **Validation:** Transport integration tests cover stdio and any retained HTTP
  mode
- **Files:** `crates/anvil-cli/src/mcp/transports/`,
  `archive/anvil-mcp-server/src/transports/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RMCPF-002

---

### Phase 3 — Cutover

#### RMCPF-030: Compatibility test harness and migration docs

- **Status:** Draft
- **Intent:** Prove users can move from the TS MCP server to the Rust MCP server
  without contract surprises.
- **Expected Outcome:** Fixture-backed compatibility harness runs TS and Rust MCP
  servers side by side; migration docs explain command changes, client config,
  and known retirements. For resources/prompts, the migration docs must call out
  the Rust-owned source of truth for constraints, drift, and suppressions rather
  than pointing users at the archived TS export pipeline.
- **Validation:** Harness passes for all ported surfaces; docs smoke-tested with
  a clean Cursor and Claude Code config
- **Files:** `apps/e2e/src/`,
  `docs/guides/rust-mcp-migration.md`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RMCPF-010, RMCPF-011, RMCPF-020

---

#### RMCPF-031: Retire or archive TypeScript MCP server

- **Status:** Draft
- **Intent:** Remove the split MCP runtime once Rust parity is sufficient.
- **Expected Outcome:** `archive/anvil-mcp-server` is either archived with historical
  docs or retained only for compatibility tests with a clear sunset date.
- **Validation:** No release-critical docs or generated configs point at the TS
  MCP server; package retirement decision recorded
- **Files:** `archive/anvil-mcp-server/`, `docs/`, `plans/index.aps.md`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RMCPF-030

---

### Phase 4 — Agent-facing validate_write ergonomics

Design authority:
[`plans/specs/2026-08-09-agent-facing-validate-write-ergonomics.md`](../specs/2026-08-09-agent-facing-validate-write-ergonomics.md)
(Approved 2026-08-09). Layer A is wire lean (anvil ships). Layer B is
harness-agnostic display guidance only (docs; no vendor UI).

#### RMCPF-040: Lean validate_write detail control (A1)

- **Status:** In Progress
- **Intent:** Let callers request a minimal clean-allow response without
  changing validation quality; keep the full envelope as the default until
  drivers and skills are ready for the flip (A4).
- **Expected Outcome:** `anvil_validate_write` accepts optional
  `detail: "minimal" | "full"` and honours process env
  `ANVIL_MCP_VALIDATE_DETAIL` (request wins over env; default **full**). On
  clean `decision: allow` with `detail: minimal`, the JSON body is only
  `{ "schema": "anvil.mcp.validate-write.v1", "decision": "allow" }`. Warn,
  block/veto, and error paths keep actionable full payloads. Validation
  quality (secrets, policy, patch/post-image) is unchanged.
- **Validation:** `cargo test -p eddacraft-anvil --lib mcp::tools::validate_write` (or
  equivalent filter covering new detail tests) passes; existing allow tests
  still see full envelope under default detail.
- **Files:** `crates/anvil-cli/src/mcp/tools/validate_write.rs`,
  `plans/specs/2026-08-09-agent-facing-validate-write-ergonomics.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** none (design approved)
- **releaseScope:** patch

#### RMCPF-041: Skill and tool-description lean-call guidance (A2)

- **Status:** In Progress
- **Intent:** Steer agents to lean request shapes and lean allow interpretation.
- **Expected Outcome:** Tool description and `anvil-developer-functions` skill
  prefer `anvil_apply_patch` / patch-only validate_write over full
  `proposedContent`; state that `decision` alone is authoritative on allow;
  rank preview+hash as partial, not quality-default.
- **Validation:** Docs/skill review against the design decision table; no
  regression in MCP tool catalogue tests.
- **Files:** `crates/anvil-cli/src/mcp/tools/validate_write.rs` (descriptor),
  skill pack / developer-functions skill as distributed for this repo
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RMCPF-040
- **releaseScope:** patch

#### RMCPF-042: apply_patch response detail parity (A3)

- **Status:** In Progress
- **Intent:** Preferred edit path is not the verbose exception.
- **Expected Outcome:** `anvil_apply_patch` honours the same `detail` /
  env / decision-gated minimal allow rules as validate_write; as-built and
  CHANGELOG note the contract.
- **Validation:** Unit tests for apply_patch minimal/full allow; as-built
  section updated.
- **Files:** `crates/anvil-cli/src/mcp/tools/apply_patch.rs`,
  `docs/architecture/mcp-shim-as-built.md`, `CHANGELOG.md`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RMCPF-040
- **releaseScope:** patch

#### RMCPF-043: Flip default validate detail to minimal (A4)

- **Status:** Ready
- **Intent:** Make lean allow the default for all harnesses after consumers
  tolerate omitted empty fields.
- **Expected Outcome:** Default response detail is **minimal** for clean
  allow; `detail: "full"` and env restore the pre-A4 envelope. Driver fixtures
  and release notes updated.
- **Validation:** Default-allow tests expect minimal keys; full detail
  fixture still matches claim-bearing envelope; TS driver client tests green.
- **Files:** `crates/anvil-cli/src/mcp/tools/validate_write.rs`,
  `packages/anvil-driver-client/` as needed, release notes
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RMCPF-040, RMCPF-041, RMCPF-042
- **releaseScope:** minor (default wire behaviour change)

#### RMCPF-044: Harness-agnostic display summary contract (B1)

- **Status:** Ready
- **Intent:** Publish portable one-line summary guidance for every MCP client;
  no vendor UI code.
- **Expected Outcome:** Public/integration MCP docs state tool · path ·
  decision summary shape, display ≠ context, and never fold non-allow into
  allow-only groups; skill cross-link.
- **Validation:** `pnpm docs:check` (or narrow docs lint for touched paths).
- **Files:** `docs/public/anvil/integrations/mcp.md` (or current integration
  guide), skill cross-link
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RMCPF-040 (wire fields stable enough to document)
- **releaseScope:** patch


## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Full parity blocks release momentum | Medium | High | RMCP ships first; RMCPF is next-release work only |
| Existing clients depend on obscure TS server behaviours | Medium | Medium | RMCPF-001 inventory plus side-by-side compatibility harness |
| HTTP transport is kept without real demand | Medium | Medium | RMCPF-021 requires explicit transport decision before implementation |
| Graph context scope leaks into parity work | Medium | Medium | GCTX owns new graph context tools; RMCPF only ports or retires existing server behaviour |
| Retirement breaks package consumers | Medium | High | Migration docs, deprecation window, and generated config cutover before archive |

## Decisions

1. **Next release, not current release** — full MCP parity follows RMCP and does
   not block the launch shim.
2. **Parity before novelty** — existing server functionality is ported or
   retired before new graph-context tools are added.
3. **Rust binary remains canonical** — after cutover, generated client configs
   continue to launch `anvil mcp serve`.
4. **TS server is archived reference material** — under ADR-033 the TS MCP
   server is at `archive/anvil-mcp-server/`, not actively maintained. RMCPF
   uses it as a frozen contract source during the port; RMCPF-031 closes out
   by deciding whether the archive remains historical or is deleted once
   parity ships.

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Inventory and Compatibility | 3 | 3/3 done |
| 1 — Tool Parity | 3 | 3/3 done (RMCPF-010 Complete; RMCPF-011/-012 Merged via PR #1558) |
| 2 — Resources and Transports | 2 | 1/2 done (RMCPF-020 Merged via #2809; RMCPF-021 Draft) |
| 3 — Cutover | 2 | 0/2 (Draft) |
| 4 — Validate-write ergonomics | 5 | 0/5 (RMCPF-040 PR #3718; 041–042 In Progress on stack; 043–044 Ready) |
| **Total** | **15** | **7/15 done** |
