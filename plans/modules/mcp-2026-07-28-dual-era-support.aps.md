# MCP 2026-07-28 Dual-Era Support

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| MCP26 | —     | In Progress | 0/11     |

**Last reviewed:** 2026-07-27 — module filed and MCP26-001 started on
`feat/mcp26-dual-era-support` (pre-ratification branch). Final MCP
`2026-07-28` publication is scheduled 2026-07-28; no dual-era wire work
merges until MCP26-001 seals the final schema and SDK/adapter ADR.

## Purpose

Make `anvil mcp serve --stdio` a dual-era MCP server: modern clients use
`server/discover` and per-request `_meta` under protocol `2026-07-28`, while
supported legacy initialise-era clients continue to work unchanged. Prefer the
official Rust SDK (`rmcp` v3) for protocol framing once it passes the adoption
gate; keep anvil-owned tool, resource, validation, redaction, auth, and
workspace behaviour behind that boundary.

## Design source

- Spec:
  [`plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md`](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md)
- Gate evidence:
  [`plans/audits/2026-07-27-mcp26-001-ratification-gate.md`](../audits/2026-07-27-mcp26-001-ratification-gate.md)
- ADR (Proposed):
  [`plans/decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md`](../decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md)
- Upstream RC:
  [MCP 2026-07-28 release candidate](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
- [Draft changelog](https://modelcontextprotocol.io/specification/draft/changelog)
- [Official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)

## In Scope

- Dual-era stdio protocol host for modern `2026-07-28` and a sealed legacy
  version matrix
- `server/discover`, modern request `_meta` validation, modern result envelopes
  (`resultType`, server identity, cache fields)
- Typed separation of era-neutral domain handlers from protocol rendering
- Official `rmcp` v3 adoption when the adoption gate passes; otherwise a
  temporary typed internal adapter with an explicit removal condition
- Graph warm-up relocation off modern-only dependence on `initialize`
- Process-local MCP egress budget terminology (not protocol sessions)
- Activation verification via modern discovery with legacy fallback
- Tool `inputSchema` validation under JSON Schema 2020-12
- W3C Trace Context correlation from request `_meta`
- Official conformance suites, dual-era fixtures, client/platform matrix
- Architecture, public MCP, activation, and release documentation for the
  dual-era posture

## Out of Scope

- Streamable HTTP or deprecated HTTP+SSE — remains RMCPF-021 if ever required
- OAuth / OpenID Connect MCP authorisation; existing stdio credential gate is
  unchanged
- MCP Apps, Tasks, Multi Round-Trip Requests
- `subscriptions/listen` while anvil advertises no list-change or resource
  subscriptions
- Reintroducing Roots, Sampling, Logging, or prompts
- Changing tool names, tool semantics, resource URIs, or editor config shapes
  (MCPX / SKPKG client install paths stay stable)
- New graph-context tools or resources (GCTX)
- Client expansion beyond the existing MCPX matrix (MCPX)

## Interfaces

**Depends on:**

- [rust-mcp-full-port](rust-mcp-full-port.aps.md) — shipped Rust MCP tool and
  resource surface under `anvil mcp serve`
- [mcp-client-expansion](mcp-client-expansion.aps.md) — supported client matrix
  that must keep working on the sealed legacy set
- [activation-mcp-optional](activation-mcp-optional.aps.md) — activation probe
  path updated by MCP26-007
- `crates/anvil-cli/src/commands/mcp.rs` — current hand-written stdio host
- `crates/anvil-cli/src/mcp/` — tools, resources, validation, redaction
- `crates/anvil-cli/src/activation/mcp_client.rs` — current legacy initialise
  probe
- Final MCP `2026-07-28` schema and changelog (MCP26-001 gate)
- Stable `rmcp` v3 targeting final `2026-07-28`, or the temporary adapter path
  in ADR-113

**Exposes:**

- Dual-era stdio MCP server binary behaviour for modern and legacy clients
- Sealed supported protocol version matrix and ADR for SDK vs adapter
- Modern activation verification with clean legacy fallback
- Golden dual-era protocol fixtures and conformance evidence
- Authoritative docs describing dual-era support without requiring modern
  clients to `initialize`

## Constraints

- UK English in plan text and user-facing docs
- Never echo an arbitrary requested protocol version
- One set of anvil-owned handlers for both eras; protocol adapter owns envelopes
- Do not ship modern-only support while the generated client support matrix
  still contains legacy-only clients
- Preserve four MiB frame ceiling, stdout protocol purity, workspace
  containment, redaction, auth, deterministic catalogue ordering, and
  process-local graph egress accounting
- Graph warm-up must not delay `server/discover`
- Client identity metadata is never trusted for auth, workspace selection, or
  policy
- Temporary internal adapter (if needed) uses typed DTOs and the same
  conformance fixtures; no further ad hoc `serde_json::Value` branching in
  `commands/mcp.rs`
- Keep this work on a feature branch until the final specification is ratified
  and MCP26-001 closes

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Final schema diverges from the locked RC | Low | High | MCP26-001 final schema/changelog diff gate |
| `rmcp` v3 late or incomplete | Medium | Medium | Temporary typed adapter + removal condition in ADR-113 |
| SDK transport bypasses four MiB frame limit | Medium | High | Adoption spike and oversize-frame tests before pin |
| Legacy clients reject additive modern fields | Medium | High | Era-specific rendering; real-client matrix |
| Modern probe leaves orphan children or kills legacy path | Medium | Medium | Disposable modern probe, fresh legacy child |
| Cache policy leaks workspace data | Low | High | `private` scope; zero-TTL resource reads |
| Protocol refactor weakens security controls | Low | High | Domain-handler extraction + security regression suite |
| SDK raises startup or resident memory cost | Medium | Medium | Benchmark gate before adoption |

## Ready Checklist

Change module status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies and non-goals identified
- [x] Work items MCP26-001..011 defined with intent, outcome, validation
- [ ] MCP26-001 seals the final schema, legacy matrix, and SDK/adapter ADR
- [ ] No dual-era implementation PR merges before MCP26-001 closes

## Sequencing

| Wave | Items | Gate |
| ---- | ----- | ---- |
| 0 | MCP26-001 | Final schema + SDK/adapter ADR |
| 1 | MCP26-002 | Era-neutral domain handlers and golden fixtures |
| 2 | MCP26-003, MCP26-004, MCP26-005 | Dual-era wire with modern and legacy golden fixtures |
| 3 | MCP26-006, MCP26-007 | Lifecycle, warm-up, activation probe |
| 4 | MCP26-008, MCP26-009 | Schema 2020-12 and trace correlation |
| 5 | MCP26-010 | Official conformance + client/platform matrix |
| 6 | MCP26-011 | Docs and release notes |

MCP26-003..005 may develop together but must not merge without both modern and
legacy golden fixtures.

## Work Items

### MCP26-001: Ratification and SDK readiness gate

- **Status:** In Progress 2026-07-27 on `feat/mcp26-dual-era-support`
- **Intent:** Seal the final upstream contract and choose the protocol
  implementation path before any dual-era wire lands.
- **Expected Outcome:** Final `2026-07-28` schema and changelog are diffed
  against the locked RC; the modern and legacy version matrix is sealed against
  the MCPX/support-reference clients; stable `rmcp` v3 is evaluated against the
  adoption gate (or the temporary typed adapter is authorised); an ADR records
  the choice, pin versions, and the adapter removal condition if used.
- **Files:**
  - `plans/decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md`
  - `plans/decisions/DECISION-LOG.md`
  - `plans/audits/2026-07-27-mcp26-001-ratification-gate.md`
  - `plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md`
  - `Cargo.toml` / `crates/anvil-cli/Cargo.toml` (pins only if SDK adopted at close)
- **Dependencies:** Final MCP `2026-07-28` publication; crates.io `rmcp` v3
  evaluation evidence
- **Validation:** ADR accepted and linked from this module; sealed version
  matrix recorded; no implementation PR merges citing only the prerelease schema
- **Evidence (pre-ratification, 2026-07-27):** RC lock, draft-changelog
  inventory, provisional legacy matrix, and `rmcp` status recorded in
  `plans/audits/2026-07-27-mcp26-001-ratification-gate.md`. ADR-113 Proposed.
  Still open: final schema publish, final-vs-RC diff, stable `rmcp` 3.0.0 pin
  or accepted temporary-adapter closeout.
- **Confidence:** medium

### MCP26-002: Extract anvil MCP domain handlers

- **Status:** Proposed
- **Intent:** Separate protocol concerns from anvil behaviour before changing
  the wire format.
- **Expected Outcome:** Domain dispatch and invocation live outside the thin
  `commands/mcp.rs` host; tools, resources, and errors return typed era-neutral
  results; tool and resource implementations keep current behaviour; golden
  domain-result fixtures exist independently of protocol era.
- **Files:**
  - `crates/anvil-cli/src/commands/mcp.rs`
  - `crates/anvil-cli/src/mcp/`
  - domain-result fixtures under `crates/anvil-cli/tests/` or adjacent fixture
    paths chosen during implementation
- **Dependencies:** MCP26-001
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_serve_stdio` plus
  new domain-result fixture tests; handlers do not observe negotiated protocol
  version
- **Confidence:** high

### MCP26-003: Dual-era stdio protocol host

- **Status:** Proposed
- **Intent:** Serve modern and legacy clients from one binary without mixing
  lifecycle rules.
- **Expected Outcome:** Modern per-request `_meta` is parsed and validated;
  unsupported modern versions return `-32022` with `supported` and `requested`;
  legacy initialisation works for the sealed legacy set; legacy-only lifecycle
  methods (`ping`, `shutdown`, `exit`, `notifications/initialized`) are gated to
  the legacy path; four MiB frame ceiling and stdout discipline remain.
- **Files:**
  - `crates/anvil-cli/src/commands/mcp.rs`
  - `crates/anvil-cli/src/mcp/` (protocol adapter / server host)
  - `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Dependencies:** MCP26-001, MCP26-002
- **Validation:** Modern tool call without `initialize` succeeds; legacy
  initialise + tool flow succeeds; unknown modern version yields `-32022`;
  modern `exit` does not terminate the server
- **Confidence:** medium

### MCP26-004: Discovery and capability declaration

- **Status:** Proposed
- **Intent:** Implement mandatory modern `server/discover` with honest
  capability claims.
- **Expected Outcome:** `server/discover` returns supported modern versions,
  tools/resources capabilities only, current instructions, server identity,
  cache fields, and no prompts/Tasks/Apps/subscriptions claims; extension map
  is absent or empty; response matches the final schema golden fixture.
- **Files:**
  - `crates/anvil-cli/src/mcp/`
  - `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Dependencies:** MCP26-001, MCP26-002
- **Validation:** Discovery golden fixture matches final schema; advertised
  capabilities exactly match implemented methods
- **Confidence:** high

### MCP26-005: Modern result envelopes and caching

- **Status:** Proposed
- **Intent:** Make every modern successful result conforming and cache-safe.
- **Expected Outcome:** Modern successes include `resultType: "complete"` and
  server identity in result `_meta`; discovery, list, and resource-read results
  carry the sealed cache policy (`discover`/`tools.list`/`resources.list`
  ttlMs 3600000 private; `resources.read` ttlMs 0 private); tool and resource
  ordering stay deterministic; legacy wire fixtures remain unchanged unless
  proven harmless.
- **Files:**
  - `crates/anvil-cli/src/mcp/`
  - `crates/anvil-cli/src/mcp/resources/`
  - `crates/anvil-cli/src/mcp/tools/registry.rs`
  - `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Dependencies:** MCP26-001, MCP26-002
- **Validation:** Modern `tools/list`, `tools/call`, `resources/list`, and
  `resources/read` validate against the final schema; workspace resource reads
  are private and immediately stale
- **Confidence:** high

### MCP26-006: Lifecycle, warm-up and state terminology

- **Status:** Proposed
- **Intent:** Remove modern dependence on initialise-era lifecycle and session
  wording.
- **Expected Outcome:** Graph warm-up moves to server start or one-time lazy
  action before the first workspace-backed request without delaying discovery;
  warm-up behaviour is available to both eras; user-facing errors and docs say
  process-local MCP egress budget rather than protocol session; no modern path
  requires an initialisation side effect.
- **Files:**
  - `crates/anvil-cli/src/commands/mcp.rs`
  - `crates/anvil-cli/src/mcp/resources/mod.rs`
  - related error strings and architecture docs touched by terminology
- **Dependencies:** MCP26-003, MCP26-004
- **Validation:** First modern workspace call receives warm-up behaviour;
  modern discovery does not require prior `initialize`; errors no longer tell
  modern clients to reconnect to reset a protocol session
- **Confidence:** high

### MCP26-007: Modern activation verification

- **Status:** Proposed
- **Intent:** Verify the installed anvil MCP entry without assuming a legacy
  handshake.
- **Expected Outcome:** Activation probes `server/discover` on a disposable
  child; recognises valid modern discovery and modern protocol errors; on
  non-modern failure, reaps the probe child and uses a fresh child for legacy
  `initialize`; modern identity is read from result `_meta`; diagnostics include
  `protocolEra`, `protocolVersion`, and `verificationMethod` without renaming
  the public tier label until a schema version change.
- **Files:**
  - `crates/anvil-cli/src/activation/mcp_client.rs`
  - activation unit/integration tests covering timeout, early exit, and
    malformed frames on Linux, macOS, and Windows where CI provides them
- **Dependencies:** MCP26-003, MCP26-004
- **Validation:** New anvil verifies via discovery; older binary verifies via
  legacy fallback; failed probe leaves no child process; platform coverage for
  timeout, early exit, and malformed frames
- **Confidence:** medium

### MCP26-008: JSON Schema 2020-12 verification

- **Status:** Proposed
- **Intent:** Confirm every published tool descriptor is valid under the modern
  schema contract.
- **Expected Outcome:** All published tool `inputSchema` values validate as
  JSON Schema 2020-12 with a required object root; schema depth and validation
  time are bounded; external `$ref` values are not dereferenced; any future
  `outputSchema` / `structuredContent` pairing is verified when introduced.
- **Files:**
  - `crates/anvil-cli/src/mcp/tools/registry.rs`
  - catalogue validation tests
- **Dependencies:** MCP26-001, MCP26-002
- **Validation:** Catalogue validation passes for every tool; official
  conformance scenario passes or a blocking upstream SDK issue is recorded
- **Confidence:** medium

### MCP26-009: Trace context and observability

- **Status:** Proposed
- **Intent:** Correlate MCP requests without treating client metadata as
  trusted.
- **Expected Outcome:** Valid W3C `traceparent` / `tracestate` / `baggage` from
  request `_meta` parent or link the MCP request span per existing policy;
  spans record protocol era, protocol version, and method; tool arguments,
  proposed source content, credentials, and unredacted resource bodies are never
  recorded; invalid metadata cannot panic or alter authorisation.
- **Files:**
  - `crates/anvil-cli/src/mcp/`
  - observability/tracing integration points already used by the CLI
- **Dependencies:** MCP26-003
- **Validation:** Valid `traceparent` joins the expected trace; invalid
  metadata is ignored or recorded as a validation event without panic or auth
  change
- **Confidence:** medium

### MCP26-010: Conformance and client matrix

- **Status:** Proposed
- **Intent:** Prove protocol correctness and real-client compatibility before
  shipping.
- **Expected Outcome:** Applicable official `2026-07-28` server scenarios and
  the selected legacy suite pass for advertised capabilities; repository
  fixtures cover discovery, direct modern list/call/read, missing metadata,
  unsupported version, modern result metadata, cache fields, legacy lifecycle,
  and malformed/oversized frames; benchmark drivers and E2E smoke that currently
  initialise first are updated; supported real clients are exercised where
  version selection exists; Linux, macOS, and Windows evidence is recorded;
  startup/first-response/memory budgets do not regress beyond an approved
  threshold.
- **Files:**
  - `crates/anvil-cli/tests/mcp_serve_stdio.rs`
  - `crates/anvil-bench/benches/`
  - `apps/e2e/src/smoke/smoke.e2e.test.ts`
  - conformance harness wiring as required
- **Dependencies:** MCP26-003, MCP26-004, MCP26-005, MCP26-006, MCP26-007,
  MCP26-008, MCP26-009
- **Validation:** Official applicable server scenarios pass; no open supported
  legacy client regression; platform CI green; budget gate recorded
- **Confidence:** medium

### MCP26-011: Documentation and release

- **Status:** Proposed
- **Intent:** Make the dual-era posture clear without over-exposing protocol
  detail to ordinary users.
- **Expected Outcome:** Architecture as-built and Rust MCP server specs describe
  dual-era behaviour; public MCP integration docs cover version support and
  verification troubleshooting; activation and support-reference docs match the
  modern probe; release notes and changelog state that existing client
  configurations continue to work; no authoritative current document claims
  modern MCP requires `initialize`.
- **Files:**
  - `docs/architecture/mcp-shim-as-built.md`
  - `docs/architecture/rust-mcp-server-spec.md`
  - `docs/public/anvil/integrations/mcp.md`
  - activation / support-reference docs touched by verification wording
  - release notes / changelog for the shipping release
  - this module and related APS prose if handshake-required claims remain
- **Dependencies:** MCP26-010
- **Validation:** `pnpm docs:check`; greps confirm no authoritative current
  docs claim modern MCP requires `initialize`; release notes include the
  compatibility claim
- **Confidence:** high

## Decisions

1. **Dual-era, not modern-only** — retain initialise-era stdio for the sealed
   legacy client set until the support matrix no longer needs it.
2. **SDK-first protocol layer** — prefer stable `rmcp` v3; temporary typed
   adapter only under MCP26-001 / ADR-113 with an explicit removal condition.
3. **Domain handlers stay anvil-owned** — tools, resources, auth, redaction,
   containment, and egress accounting never move into the protocol SDK.
4. **Era-neutral results, era-specific envelopes** — handlers must not inject
   protocol fields; the adapter owns `resultType`, server identity, and cache
   metadata.
5. **Conservative private caching** — fixed catalogues may cache for one hour
   privately; workspace resource reads are immediately stale.
6. **Process-local egress, not protocol sessions** — modern MCP has no session
   concept; existing byte budgets remain process-local security accounting.
7. **Activation probes modern first** — discovery on a disposable child, then a
   fresh legacy child; never leave probe processes behind.
8. **Branch until ratification** — keep MCP26 implementation and plan closeout
   on a feature branch until the final specification is ratified and MCP26-001
   completes.

## Notes

- Spec non-goals (HTTP, Apps, Tasks, MRTR, subscriptions) remain follow-on
  opportunities outside MCP26.
- RMCPF still owns remaining full-port leftovers (RMCPF-021 transport decision,
  RMCPF-030 compatibility harness vs archived TS, RMCPF-031 archive closeout).
  MCP26 owns the protocol-version dual-era cut, not tool/resource parity.
- Stored progress `0/11` is advisory (ADR-053). Item `Status:` lines are
  authoritative.
