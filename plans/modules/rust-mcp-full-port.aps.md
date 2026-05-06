# Rust MCP Full Port

| ID    | Owner | Status | Progress |
| ----- | ----- | ------ | -------- |
| RMCPF | —     | Draft  | 0/9      |

**Last reviewed:** 2026-04-28

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

## Ready Checklist

Change status to **Ready** when:

- [ ] RMCP has shipped or reached Committed state
- [ ] Existing TS MCP server inventory is complete
- [ ] Supported-client matrix for Claude Code, Cursor, Continue, VSCode, and
  any remaining HTTP clients is confirmed
- [ ] Decision recorded on whether Streamable HTTP remains required
- [ ] Retirement criteria for `archive/anvil-mcp-server` agreed

---

## Phase 0 — Inventory and Compatibility

### RMCPF-001: Existing MCP surface inventory

- **Status:** Draft
- **Intent:** Create the compatibility matrix for every current TS MCP tool,
  resource, prompt, and transport.
- **Expected Outcome:** Inventory records contract, implementation owner,
  current tests, supported clients, and disposition: port, retire, or defer.
- **Validation:** Inventory reviewed against `archive/anvil-mcp-server/src/` and the
  current package test suite
- **Files:** `plans/specs/rust-mcp-full-port-inventory.md`,
  `archive/anvil-mcp-server/src/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** RMCP

---

### RMCPF-002: Rust MCP parity architecture spec

- **Status:** Draft
- **Intent:** Define the post-launch Rust MCP architecture and how it relates to
  the daemon, driver framework, Graph v2, and legacy TS server.
- **Expected Outcome:** Spec defines command layout, protocol support,
  validation paths, resource serving, prompt strategy, transport support, and
  retirement gates for the TS package. The spec MUST adopt the DRVR-006
  resolution (see
  `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md` §4.3,
  recorded 2026-05-06 in A2 Wave 1): each ported tool is classified as either
  a **daemon-RPC translator** (`anvil_check`, `anvil_status`, `anvil_suppress`)
  or **MCP-driver-local composition** (`anvil_fix`, `anvil_gate`,
  `anvil_query_boundary`). The architecture spec MUST NOT introduce new
  daemon RPCs whose only consumer is parity prose; if RMCPF needs additional
  daemon authority it MUST file new INTD work items rather than expanding the
  RPC surface implicitly.
- **Validation:** Council review confirms the spec does not regress the RMCP
  single-binary launch path, and confirms each tool's class matches the
  DRVR-006 table above (or records a deliberate amendment with rationale).
- **Files:** `docs/architecture/rust-mcp-server-spec.md`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** RMCPF-001

---

## Phase 1 — Tool Parity

### RMCPF-010: Port check/gate/status tools

- **Status:** Draft
- **Intent:** Move the core read-only validation tools from the TS MCP server to
  Rust while preserving response contracts or documenting intentional changes.
- **Expected Outcome:** Rust MCP server exposes parity for `anvil_check`,
  `anvil_gate`, and `anvil_status` or their explicitly versioned successors.
  Per the DRVR-006 resolution
  (`plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md` §4.3):
  - `anvil_check` and `anvil_status` are **daemon-RPC translators**. Their
    handlers call the daemon's `scan.files` / `scan_buffer` and `status.query`
    surfaces (with the embedded fallback when the daemon is unavailable, as
    RMCP-005 already wires).
  - `anvil_gate` is **MCP-driver-local composition**: the handler shells to
    `anvil gate` (or invokes the equivalent in-process gate path) because
    `GateRunner` runs `npm audit`, OPA, and coverage JSON reads that the
    daemon deliberately does not do.
  - All response payloads pass through the redaction contract recorded in
    §4 of the design spec (DRVR-007) before leaving the MCP transport.
- **Validation:** Compatibility tests compare TS and Rust responses on fixture
  workspaces; tests assert the DRVR-006 classification (e.g. `anvil_gate`
  results are reachable when the daemon is offline, `anvil_check` falls back
  to embedded scan).
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
  documented retirement.
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
  and known retirements.
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
| 0 — Inventory and Compatibility | 2 | Draft |
| 1 — Tool Parity | 3 | Draft |
| 2 — Resources and Transports | 2 | Draft |
| 3 — Cutover | 2 | Draft |
| **Total** | **9** | **0/9 done** |
