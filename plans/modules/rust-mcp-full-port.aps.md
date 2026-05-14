# Rust MCP Full Port

| ID    | Owner | Status | Progress |
| ----- | ----- | ------ | -------- |
| RMCPF | —     | In Progress | 3/10     |

**Last reviewed:** 2026-05-14

> **Plan change (2026-04-29, [ADR-033](../decisions/033-park-ide-mcp-retire-ts-scanner.md)):**
> The TypeScript MCP server (`archive/anvil-mcp-server/`) is now
> **archived** — moved out of `packages/mcp-server/` per the
> project's archive convention. RMCPF still owns full Rust-side
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
>   `archive/anvil-mcp-server/`. RMCPF-031 closes out by deciding
>   whether the archive remains as historical reference or is
>   deleted once parity ships.

## Purpose

After the current release proves the narrow Rust MCP launch shim, port the
existing TypeScript MCP server functionality into the Rust `anvil` binary so MCP
is no longer split across a launch-critical Rust path and a legacy Node/TS
sidecar.

**Why:** RMCP deliberately does not port `archive/anvil-mcp-server`. It ships only
the A1 pre-write validation path. The next release can then do the slower,
parity-focused work: existing tools, resources, prompts, transports, tests, and
compatibility behaviour move behind the Rust binary without jeopardising the
launch demo. Per ADR-033 the TS server is **archived**, so RMCPF's parity work
runs against a frozen reference — not a moving sidecar.

## In Scope

- Inventory and compatibility matrix for all current `archive/anvil-mcp-server`
  tools, resources, prompts, and transports
- Rust implementations of existing MCP tool contracts
- Rust implementations of existing MCP resource contracts where still needed
- Rust implementations or documented retirement of existing prompts
- Streamable HTTP transport parity if still required by supported clients
- Compatibility tests comparing TS and Rust MCP responses on fixture workspaces
- Deprecation/retirement path for `archive/anvil-mcp-server` once parity is reached
- Migration documentation for users who previously launched the TS MCP server

## Out of Scope

- Blocking the current release on full MCP parity
- Re-opening RMCP's launch-shim scope
- Creating new graph-context tools beyond what GCTX explicitly owns
- Replacing the daemon or driver framework
- Adding non-MCP editor integrations

## Interfaces

**Depends on:**

- RMCP — current-release Rust MCP stdio launch path
- DRVR / ADR-030 — driver and daemon direction for integration surfaces
- GV2/GCTX where graph context tools are introduced
- `archive/anvil-mcp-server` — compatibility source of truth until retirement
- `crates/anvil-cli` — Rust command host

**Exposes:**

- Full Rust MCP server surface under `anvil mcp serve`
- Migration plan for retiring or archiving `archive/anvil-mcp-server`
- Compatibility report for existing MCP users

## Constraints

- UK English spelling in all plan text and user-facing docs
- Existing MCP contracts must either be preserved or explicitly retired with a
  migration note
- The Rust server must preserve the single-binary story established by RMCP
- Feature parity claims require fixture-backed tests, not manual spot checks
- Graph-context features depend on GV2/GCTX and must not become accidental scope
  in this module

## Phase 1 Readiness Checklist

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

## Phase 0 — Inventory and Compatibility

### RMCPF-001: Existing MCP surface inventory

- **Status:** Complete
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

### RMCPF-002: Rust MCP parity architecture spec

- **Status:** Complete
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

### RMCPF-003: Phase 1 readiness decision closure

- **Status:** Complete
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

## Phase 1 — Tool Parity

### RMCPF-010: Port check/gate/status tools

- **Status:** In Progress
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
  passed before PR closeout.
- **Closeout evidence:** The Rust MCP tool registry now dispatches multiple
  tools without changing `tools/call` protocol handling. `anvil_status` is
  implemented as an unauthenticated read-only local status tool for this slice
  because no Rust daemon `status.query` MCP surface exists yet; its response
  keeps the archived TS field names (`status`, `workspaceRoot`,
  `availableChecks`, `config`, `hasBaseline`, `version`) while redacting path
  values to workspace-relative forms and adding explicit `backend: "local"` /
  `daemonStatus: "not-wired"` provenance for future daemon replacement. The
  workspace root is canonicalised and rejected when it resolves outside the MCP
  server root before any filesystem reads, and `hasBaseline` reads the archived
  architecture baseline source (`.anvil/architecture.json`).
- **Files:** `crates/anvil-cli/src/mcp/tools/`,
  `archive/anvil-mcp-server/src/tools/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** RMCPF-002

---

### RMCPF-011: Port fix/suppress/boundary tools

- **Status:** Draft
- **Intent:** Move mutation and architecture-query tools to Rust with safe
  validation and redaction boundaries.
- **Expected Outcome:** Rust MCP server exposes parity for `anvil_fix`,
  `anvil_suppress`, and `anvil_query_boundary` or documented successors.
- **Validation:** Compatibility tests cover success, failure, workspace escape,
  and dry-run cases
- **Files:** `crates/anvil-cli/src/mcp/tools/`,
  `archive/anvil-mcp-server/src/tools/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RMCPF-010

---

### RMCPF-012: Port or retire MCP prompts

- **Status:** Draft
- **Intent:** Decide whether existing TS MCP prompts should move to Rust or be
  retired in favour of docs and tool descriptions.
- **Expected Outcome:** Prompt parity exists where still useful; retired prompts
  have migration notes and tests updated accordingly.
- **Validation:** Prompt list from Rust matches the inventory disposition
- **Files:** `crates/anvil-cli/src/mcp/prompts/`,
  `archive/anvil-mcp-server/src/prompts/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RMCPF-001

---

## Phase 2 — Resources and Transports

### RMCPF-020: Port MCP resources

- **Status:** Draft
- **Intent:** Move existing MCP resources to Rust or retire those that no longer
  fit the post-daemon architecture.
- **Expected Outcome:** Baseline, boundaries, patterns, suppressions, config,
  constraints, drift, and file-warning resources have Rust equivalents or
  documented retirement. Rust MCP resources must not treat the archived TS
  `runtime-export` collector as the active contract: `anvil://constraints` is
  sourced from the Rust CLI constraint exporter
  (`crates/anvil-cli/src/commands/export.rs`), drift resources are sourced from
  the Rust drift command/data model (`crates/anvil-cli/src/commands/drift.rs`),
  and suppressions are sourced from the same active `.anvil/suppressions.json`
  readers used by Rust CLI surfaces. Archived TS resource shapes are fixture
  evidence for compatibility review only.
- **Validation:** Resource listing and read tests match the inventory matrix
- **Files:** `crates/anvil-cli/src/mcp/resources/`,
  `archive/anvil-mcp-server/src/resources/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RMCPF-002

---

### RMCPF-021: Transport parity decision and implementation

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

## Phase 3 — Cutover

### RMCPF-030: Compatibility test harness and migration docs

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

### RMCPF-031: Retire or archive TypeScript MCP server

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
| 1 — Tool Parity | 3 | RMCPF-010 In Progress |
| 2 — Resources and Transports | 2 | Draft |
| 3 — Cutover | 2 | Draft |
| **Total** | **10** | **3/10 done** |
