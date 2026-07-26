<!--
APS Module: MCP 2026-07-28 Dual-Era Support
==========================================
Dual-era stdio MCP server: modern 2026-07-28 clients via server/discover and
per-request _meta, plus retained initialise-era legacy clients. Prefer official
rmcp SDK once stable; anvil keeps tool/resource/security handlers.
Design: plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md
See: plans/aps-rules.md
-->

# MCP 2026-07-28 Dual-Era Support

| ID    | Owner | Status | Progress |
| ----- | ----- | ------ | -------- |
| MCP26 | —     | Draft  | 0/11     |

**Last reviewed:** 2026-07-27 — rewritten from a design-shaped document into a
canonical APS module; design assessment lives in
[`plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md`](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md).

## Purpose

Support MCP `2026-07-28` as a **dual-era stdio server** without dropping
supported legacy clients:

- Modern clients use `server/discover` and carry protocol version, client
  capabilities, and optional client identity in `_meta` on every request.
- Legacy clients continue to use `initialize` / `notifications/initialized`
  inside their stdio process.
- One set of anvil-owned tool and resource handlers serves both eras; protocol
  negotiation and result envelopes stay at the protocol adapter boundary.
- Prefer the official Rust SDK (`rmcp` v3) once stable and conformant; if not
  available at ratification, a short-lived typed internal adapter is allowed
  under ADR (MCP26-001) — not more ad hoc `serde_json::Value` branching in
  `commands/mcp.rs`.

## In Scope

- Dual-era stdio protocol host (modern + sealed legacy version matrix)
- `server/discover`, modern request metadata validation, modern result envelopes
  (`resultType`, server identity `_meta`, cache fields)
- Domain-handler extraction so tool/resource code is era-neutral
- Graph warm-up and process-local egress budget terminology (not "session")
- Activation verification probe: modern discovery first, legacy initialise
  fallback on a fresh child process
- JSON Schema 2020-12 verification for published tool descriptors
- Trace-context propagation from request `_meta` (observability only)
- Conformance + client matrix (official suites + real supported clients)
- Architecture/public docs and release notes for the dual-era posture

## Out of Scope

- Streamable HTTP / deprecated HTTP+SSE
- OAuth / OpenID Connect MCP authorisation
- MCP Apps, Tasks, Multi Round-Trip Requests
- `subscriptions/listen` while anvil advertises no subscription capability
- Reintroducing Roots, Sampling, Logging, or prompts
- Changing tool names, tool semantics, resource URIs, or editor config shapes
- Full TS MCP parity (owned by RMCPF) and client-matrix expansion (MCPX)

## Interfaces

**Depends on:**

- [rust-mcp-full-port](./rust-mcp-full-port.aps.md) (RMCPF) — current Rust MCP
  tool/resource surface and handler inventory
- [rust-mcp-launch-shim](../archive/modules/rust-mcp-launch-shim.aps.md)
  (RMCP, Complete) — A1 stdio launch path
- [mcp-client-expansion](./mcp-client-expansion.aps.md) (MCPX) — supported
  client matrix for legacy retention claims
- [activation-mcp-optional](./activation-mcp-optional.aps.md) (ACTMO) —
  activation MCP probe / install path
- Design:
  [2026-07-27-mcp-2026-07-28-dual-era-support](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md)

**Exposes:**

- Dual-era `anvil mcp serve --stdio` wire behaviour
- Modern discovery-based activation verification with legacy fallback
- Conformance evidence for modern and selected legacy protocol versions

**Coordinates with:**

- RMCPF remaining items if handler extraction touches shared MCP layout
- Docs / release notes for the first post-ratification ship window

## Design

Authoritative design assessment (upstream facts, current repo assessment,
compatibility model, SDK adoption gate, risks, definition of done):

- [plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md)

**Executive decision (summary):** dual-era server; modern and legacy responses
rendered by protocol version; tool/resource/security logic stays anvil-owned
behind an SDK (or typed temporary adapter) boundary.

## Ready Checklist

Change status to **Ready** when:

- [ ] MCP `2026-07-28` is ratified (or MCP26-001 explicitly authorises RC-locked
      implementation with a final-diff gate before merge)
- [ ] MCP26-001 ADR records SDK adoption vs temporary typed adapter
- [ ] Legacy version set sealed against the supported client matrix (MCPX)
- [ ] Work items MCP26-002..011 have enough detail to execute without reopening
      the dual-era product decision

## Work Items

### MCP26-001: Ratification and SDK readiness gate

- **Status:** Proposed
- **Intent:** Seal the final upstream contract and choose SDK vs typed adapter.
- **Expected Outcome:** Final schema/changelog diffed against the locked RC;
  modern + legacy version matrix confirmed; ADR records `rmcp` v3 adoption or
  temporary typed adapter with removal condition; conformance-suite and SDK
  versions pinned.
- **Validation:** `pnpm adr:check`; ADR present under `plans/decisions/` and
  linked from DECISION-LOG; module Ready Checklist items for ratification and
  ADR ticked
- **Files:** `plans/decisions/`, `plans/decisions/DECISION-LOG.md`,
  `plans/modules/mcp-dual-era-support.aps.md`,
  `plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md`
- **Dependencies:** None
- **Confidence:** high
- **Risks:** Final schema changes after RC; `rmcp` v3 late or non-conformant
- **changeType:** internal
- **releaseIntent:** hold
- **releaseScope:** none

### MCP26-002: Extract anvil MCP domain handlers

- **Status:** Proposed
- **Intent:** Separate protocol concerns from anvil behaviour before changing
  the wire.
- **Expected Outcome:** Tool/resource domain invocation lives outside raw
  JSON-RPC dispatch; handlers return era-neutral typed results; golden
  domain-result fixtures exist independent of protocol era.
- **Validation:** `cargo test -p eddacraft-anvil -- mcp`; domain golden fixtures
  pass without requiring a negotiated protocol version in handlers
- **Files:** `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/src/mcp/**`
- **Dependencies:** MCP26-001
- **Confidence:** medium
- **Risks:** Refactor churn across the 14-tool / resource surface
- **changeType:** internal
- **releaseIntent:** hold
- **releaseScope:** none

### MCP26-003: Dual-era stdio protocol host

- **Status:** Proposed
- **Intent:** Serve modern and legacy clients from one binary.
- **Expected Outcome:** Modern per-request `_meta` validated; supported-version
  enforcement with `-32022`; sealed legacy initialise path retained;
  legacy-only lifecycle methods gated; four MiB frame ceiling and stdout
  discipline preserved.
- **Validation:** Integration fixtures: modern tool call without initialise;
  legacy initialise + tool flow; unsupported modern version → `-32022`; modern
  `exit` does not terminate the server
- **Files:** `crates/anvil-cli/src/mcp/**`,
  `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Dependencies:** MCP26-002
- **Confidence:** medium
- **Risks:** Era-selection edge cases; Windows process behaviour
- **changeType:** external
- **releaseIntent:** ship
- **releaseScope:** mcp-protocol

### MCP26-004: Discovery and capability declaration

- **Status:** Proposed
- **Intent:** Implement mandatory modern `server/discover`.
- **Expected Outcome:** Discovery returns supported modern versions,
  tools/resources capabilities, instructions, server identity, and cache
  fields; no prompts/Tasks/Apps/subscriptions claims.
- **Validation:** Discovery golden fixture matches final schema; capability
  claims match implemented methods only
- **Files:** `crates/anvil-cli/src/mcp/**`,
  `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Dependencies:** MCP26-003
- **Confidence:** high
- **Risks:** Schema drift on discovery result shape
- **changeType:** external
- **releaseIntent:** ship
- **releaseScope:** mcp-protocol

### MCP26-005: Modern result envelopes and caching

- **Status:** Proposed
- **Intent:** Make every modern successful result conforming and cache-safe.
- **Expected Outcome:** Modern results include `resultType: "complete"`, server
  identity in result `_meta`, and cache fields on discovery/lists/resource
  reads per the design cache policy; deterministic catalogue ordering
  preserved; legacy wire unchanged unless proven harmless.
- **Validation:** Modern `tools/list`, `tools/call`, `resources/list`,
  `resources/read` validate against the final schema; resource reads use
  private zero-TTL policy
- **Files:** `crates/anvil-cli/src/mcp/**`,
  `crates/anvil-cli/src/mcp/resources/**`
- **Dependencies:** MCP26-003, MCP26-004
- **Confidence:** medium
- **Risks:** Legacy clients reject additive fields if eras mix
- **changeType:** external
- **releaseIntent:** ship
- **releaseScope:** mcp-protocol

### MCP26-006: Lifecycle, warm-up and state terminology

- **Status:** Proposed
- **Intent:** Remove modern dependence on initialise-era lifecycle and
  session-shaped language.
- **Expected Outcome:** Graph warm-up moves off `initialize` (start or lazy
  before first workspace request, best-effort, does not delay discovery);
  egress accounting is process-local budget, not protocol session; user-facing
  errors no longer tell modern clients to reconnect to reset a session.
- **Validation:** First modern workspace call receives warm-up behaviour;
  modern path requires no initialise side effect; docs/errors use process-local
  terminology
- **Files:** `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/src/mcp/resources/**`
- **Dependencies:** MCP26-003
- **Confidence:** medium
- **Risks:** Hidden cross-call state remaining in handlers
- **changeType:** internal
- **releaseIntent:** ship
- **releaseScope:** mcp-protocol

### MCP26-007: Modern activation verification

- **Status:** Proposed
- **Intent:** Verify installed anvil MCP without assuming a legacy handshake.
- **Expected Outcome:** Activation probes `server/discover` on a disposable
  child; falls back to legacy `initialize` on a fresh child; modern identity
  read from result `_meta`; diagnostics include `protocolEra`,
  `protocolVersion`, `verificationMethod`; failed probe leaves no child
  process.
- **Validation:** `cargo test -p eddacraft-anvil activation::mcp`; coverage for
  timeout, early exit, malformed frames on Linux/macOS/Windows where CI allows
- **Files:** `crates/anvil-cli/src/activation/mcp_client.rs`, related tests
- **Dependencies:** MCP26-004
- **Confidence:** medium
- **Risks:** Probe process leaks; false modern/legacy classification
- **changeType:** external
- **releaseIntent:** ship
- **releaseScope:** activation

### MCP26-008: JSON Schema 2020-12 verification

- **Status:** Proposed
- **Intent:** Confirm every published tool descriptor meets the new schema
  contract.
- **Expected Outcome:** All tool `inputSchema` values validate as JSON Schema
  2020-12 with object roots; depth/time bounds enforced; no external `$ref`
  dereference; future `outputSchema`/`structuredContent` pairing verified if
  present.
- **Validation:** Catalogue validation passes for every tool; official
  conformance schema scenario passes or blocking upstream SDK issue recorded
- **Files:** `crates/anvil-cli/src/mcp/tools/registry.rs`, related tests
- **Dependencies:** MCP26-002
- **Confidence:** medium
- **Risks:** SDK still missing 2020-12 conformance
- **changeType:** internal
- **releaseIntent:** ship
- **releaseScope:** mcp-protocol

### MCP26-009: Trace context and observability

- **Status:** Proposed
- **Intent:** Correlate MCP requests without treating client metadata as
  trusted authority.
- **Expected Outcome:** Valid W3C `traceparent`/`tracestate`/`baggage` parent
  or link spans per existing observability policy; protocol era/version/method
  on spans; no tool args, source content, credentials, or unredacted bodies
  recorded.
- **Validation:** Valid `traceparent` joins expected trace in tests; invalid
  metadata neither panics nor alters authorisation
- **Files:** `crates/anvil-cli/src/mcp/**`
- **Dependencies:** MCP26-003
- **Confidence:** medium
- **Risks:** Over-logging sensitive payloads
- **changeType:** internal
- **releaseIntent:** hold
- **releaseScope:** none

### MCP26-010: Conformance and client matrix

- **Status:** Proposed
- **Intent:** Prove protocol correctness and real-client compatibility.
- **Expected Outcome:** Applicable official `2026-07-28` and selected legacy
  server scenarios pass; repo integration fixtures cover discovery, modern
  direct calls, missing metadata, unsupported version, result metadata, cache
  fields, legacy lifecycle, malformed/oversized frames; supported real clients
  exercised where version selection allows; no unapproved latency/memory
  regression.
- **Validation:** Official conformance suite (advertised capabilities);
  `cargo test -p eddacraft-anvil -- mcp`; E2E/smoke paths that currently
  initialise first updated; platform matrix evidence
- **Files:** `crates/anvil-cli/tests/mcp_serve_stdio.rs`,
  `crates/anvil-bench/benches/**`, `apps/e2e/src/smoke/**`
- **Dependencies:** MCP26-003, MCP26-004, MCP26-005, MCP26-006, MCP26-007,
  MCP26-008
- **Confidence:** medium
- **Risks:** Client matrix gaps; conformance suite version skew
- **changeType:** external
- **releaseIntent:** ship
- **releaseScope:** mcp-protocol

### MCP26-011: Documentation and release

- **Status:** Proposed
- **Intent:** Document dual-era support without dumping protocol detail on
  ordinary users.
- **Expected Outcome:** Architecture as-built and public MCP docs describe
  dual-era posture; no authoritative current doc claims modern MCP requires
  `initialize`; release notes state existing client configs continue to work.
- **Validation:** `pnpm docs:check`; manual grep of authoritative MCP docs for
  initialise-required claims on modern path
- **Files:** `docs/architecture/mcp-shim-as-built.md`,
  `docs/architecture/rust-mcp-server-spec.md`,
  `docs/public/anvil/integrations/mcp.md`, release notes / changelog
- **Dependencies:** MCP26-010
- **Confidence:** high
- **Risks:** Stale handshake prose in archived paths
- **changeType:** docs
- **releaseIntent:** ship
- **releaseScope:** docs

## Waves

Suggested implementation order (see design §11 for detail):

| Wave | Items | Notes |
| ---- | ----- | ----- |
| 0 | MCP26-001 | Gate only — no wire change |
| 1 | MCP26-002 | Handler extraction + domain goldens |
| 2 | MCP26-003, MCP26-004, MCP26-005 | Dual-era wire; merge only with both-era goldens |
| 3 | MCP26-006, MCP26-007 | Lifecycle + activation probe |
| 4 | MCP26-008, MCP26-009 | Schema + observability |
| 5 | MCP26-010, MCP26-011 | Conformance matrix + docs/release |

Do not ship modern-only support while the generated client support matrix still
contains legacy-only clients.

## Definition of Done

Module may advance to Complete when the design definition of done is met
(design §12) and MCP26-001..011 are terminal with release evidence.
