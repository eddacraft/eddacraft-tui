# MCP 2026-07-28 Dual-Era Support — Design

| Field | Value |
| --- | --- |
| Status | Draft — awaiting ratification gate (MCP26-001) |
| Modules | [mcp-dual-era-support](../modules/mcp-dual-era-support.aps.md) (MCP26) |
| Owner | anvil CLI / MCP |
| Target | First anvil release after MCP `2026-07-28` ratification |
| Upstream | MCP `2026-07-28` release candidate / final |
| Compatibility posture | Dual-era stdio server |


This document is the **design and assessment** for dual-era MCP support.
Execution authority lives in the APS module linked above; do not treat this
file as a work-item index.

## Problem

Anvil's Rust MCP server is still a legacy-only protocol shape (pinned near
`2024-11-05` / initialise-era handshake) while the MCP `2026-07-28` revision
removes that handshake for modern clients, requires `server/discover`,
per-request protocol metadata, `resultType`, and cache fields. Maintaining a
hand-written JSON dispatcher for that surface is no longer a good trade.

## Design

## 1. Executive decision

anvil should support MCP `2026-07-28` as a **dual-era server**:

- Modern clients use `server/discover` and carry protocol version, client
  capabilities and optional client identity in `_meta` on every request.
- Legacy clients continue to use `initialize` and
  `notifications/initialized` inside their stdio process.
- Modern and legacy responses are rendered according to the protocol version
  selected for that request or legacy process.
- Streamable HTTP, MCP Apps, Tasks and subscriptions remain outside this
  module.

The protocol and transport layer should move to the official Rust SDK,
`rmcp` v3, once its stable release is available and its remaining server
conformance gap is either closed or shown not to affect anvil. anvil's tool,
resource, validation, redaction, authentication and workspace logic remain
anvil-owned handlers behind that SDK boundary.

This is preferable to extending the current hand-written JSON dispatcher. The
existing implementation began as a deliberately narrow launch shim but now
serves 14 tools, ten resources, activation verification and security-sensitive
graph egress controls. The 2026 revision adds enough versioning, result-envelope,
caching and compatibility rules that maintaining a separate protocol
implementation is no longer a good trade.

If stable `rmcp` v3 is not available when the MCP specification is ratified,
MCP26-001 may authorise a short-lived internal compatibility adapter. It must
use typed protocol DTOs and the same conformance fixtures planned for the SDK
migration. It must not add more ad hoc `serde_json::Value` branching to
`commands/mcp.rs`.

## 2. Upstream facts

The release candidate makes these relevant changes:

1. Removes the `initialize` and `notifications/initialized` handshake for
   modern clients.
2. Removes protocol-level sessions.
3. Requires every modern request to carry
   `io.modelcontextprotocol/protocolVersion` and
   `io.modelcontextprotocol/clientCapabilities` in `params._meta`.
   `io.modelcontextprotocol/clientInfo` is recommended but not required.
4. Requires servers to implement `server/discover`.
5. Requires every modern successful result to carry `resultType`.
6. Recommends server identity in
   `_meta["io.modelcontextprotocol/serverInfo"]` on every modern result.
7. Requires cache hints on discovery, list and resource-read results.
8. Removes `ping` from the modern protocol. The initialise-era lifecycle is
   legacy-only.
9. Changes missing-resource errors to JSON-RPC `-32602`.
10. Makes tool input and output schemas full JSON Schema 2020-12.
11. Adds extension negotiation, MRTR, Tasks and MCP Apps without requiring a
    server to implement those optional features.
12. Deprecates Roots, Sampling and Logging. anvil does not currently advertise
    them.

The final specification is scheduled for 28 July 2026. Before implementation
is merged, the ratified schema and changelog must be compared with the locked
release candidate.

Authoritative references:

- [Release candidate announcement](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
- [Draft changelog](https://modelcontextprotocol.io/specification/draft/changelog)
- [Versioning and compatibility](https://modelcontextprotocol.io/specification/draft/basic/versioning)
- [Server discovery](https://modelcontextprotocol.io/specification/draft/server/discover)
- [Tools](https://modelcontextprotocol.io/specification/draft/server/tools)
- [Official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Rust SDK conformance roadmap](https://github.com/modelcontextprotocol/rust-sdk/blob/main/ROADMAP.md)

## 3. Current repository assessment

### 3.1 Protocol implementation

`crates/anvil-cli/src/commands/mcp.rs` currently:

- pins `DEFAULT_PROTOCOL_VERSION` to `2024-11-05`;
- accepts `initialize`, echoes the client's requested version without checking
  a supported-version set, and returns capabilities and `serverInfo`;
- ignores `notifications/initialized`;
- supports `ping`, `shutdown` and `exit`;
- dispatches `tools/list`, `tools/call`, `resources/list` and
  `resources/read`;
- constructs raw JSON-RPC results with no `resultType`, modern result `_meta`,
  `ttlMs` or `cacheScope`;
- warms the workspace graph from the `initialize` handler;
- reads newline-delimited stdio frames with a four MiB ceiling.

This is a legacy-only protocol shape even though direct calls happen to work
without a completed initialisation handshake.

### 3.2 Activation probe

`crates/anvil-cli/src/activation/mcp_client.rs` currently:

- sends an `initialize` request using protocol `2025-06-18`;
- describes verification as a handshake;
- expects `result.serverInfo.name == "anvil"`;
- has no `server/discover` probe, modern error recognition or fallback
  algorithm.

This must change because modern server identity now lives in result `_meta`, and
a dual-era client must probe modern capability before falling back to a clean
legacy process.

### 3.3 Result and catalogue behaviour

The existing server already has useful compliant properties:

- tool ordering is deterministic because the registry is a static slice;
- resource ordering is deterministic because descriptors are assembled in a
  fixed vector;
- tool input schemas have an object root;
- an unknown or malformed resource URI already maps to `-32602`;
- anvil does not advertise Roots, Sampling, Logging, Tasks, prompts or
  subscriptions;
- stdio stdout is reserved for protocol frames;
- workspace containment, redaction, authentication and the four MiB frame
  ceiling are enforced independently of MCP version.

The migration must preserve these properties.

### 3.4 Stateful application behaviour

The graph egress allowance in
`crates/anvil-cli/src/mcp/resources/mod.rs` is described as a per-session
counter reset by reconnecting. The modern protocol has no session concept.

The control may remain as a process-local security budget because it is
operational rate and egress accounting rather than hidden workflow state.
Code, errors and documentation must call it a **per-process MCP egress budget**,
not a protocol session. Any future workflow state must use explicit,
model-visible handles passed through tool arguments.

## 4. Goals

- Accept conforming MCP `2026-07-28` stdio clients.
- Retain compatibility with supported legacy Cursor, Claude Code, VS Code,
  Zed, OpenCode, Codex and other generated client configurations.
- Use one set of anvil-owned tool and resource handlers for both eras.
- Make protocol negotiation, request metadata and response shaping typed and
  testable.
- Pass the applicable official conformance scenarios for both
  `2026-07-28` and the selected legacy baseline.
- Preserve cross-platform behaviour on Linux, macOS and Windows.
- Preserve current security boundaries and improve trace correlation.

## 5. Non-goals

- Streamable HTTP or the deprecated HTTP+SSE transport.
- OAuth or OpenID Connect MCP authorisation. anvil's current stdio credential
  gate is unchanged.
- MCP Apps.
- Tasks.
- Multi Round-Trip Requests in anvil tools.
- `subscriptions/listen` while anvil advertises no list-change or resource
  subscription capability.
- Reintroducing Roots, Sampling or Logging.
- Adding prompts.
- Changing tool names, tool semantics or resource URIs.
- Changing editor configuration file shapes.

## 6. Compatibility model

### 6.1 Supported versions

The exact legacy set is sealed by MCP26-001 after checking the clients in the
generated support reference. The intended starting point is:

| Era | Versions | Behaviour |
| --- | --- | --- |
| Modern | `2026-07-28` | Stateless, per-request metadata, modern results |
| Legacy | `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05` | Initialise-era stdio process |

The server must never echo an arbitrary requested version. Unsupported modern
versions return `UnsupportedProtocolVersionError`, code `-32022`, with
`supported` and `requested` data.

Legacy negotiation follows the selected legacy specification and is covered by
golden fixtures for every version anvil claims.

### 6.2 Era selection

| Opening/request shape | Server behaviour |
| --- | --- |
| `server/discover` with modern `_meta` | Modern |
| Any supported method with modern `_meta` | Modern |
| `initialize` without modern `_meta` | Legacy for that stdio process |
| Legacy request after successful `initialize` | Legacy |
| Request with neither modern `_meta` nor a prior legacy initialisation | Reject as invalid or uninitialised according to the selected legacy rule |

Modern requests remain independently versioned. A legacy marker exists only to
support requests from the initialise-era client attached to the same stdio
process.

### 6.3 Response shaping

The handler returns an era-neutral domain result. A protocol adapter renders:

- legacy results using the currently supported wire shape;
- modern results with `resultType: "complete"` and
  `_meta["io.modelcontextprotocol/serverInfo"]`;
- future input-required results only if anvil later implements MRTR.

Do not mutate tool handler payloads to inject protocol fields. The adapter owns
the MCP result envelope.

## 7. Modern protocol requirements

### 7.1 Request validation

For every modern request:

- require `params._meta`;
- require a string
  `io.modelcontextprotocol/protocolVersion`;
- require an object
  `io.modelcontextprotocol/clientCapabilities`;
- accept an absent `io.modelcontextprotocol/clientInfo`;
- validate a present client info object without trusting it for authorisation,
  workspace selection or policy;
- retain unknown valid metadata keys;
- reject an unsupported version with `-32022`;
- place bounds on metadata depth and size inside the existing frame limit.

### 7.2 `server/discover`

Implement `server/discover` with:

```json
{
  "resultType": "complete",
  "supportedVersions": ["2026-07-28"],
  "capabilities": {
    "tools": {},
    "resources": {}
  },
  "_meta": {
    "io.modelcontextprotocol/serverInfo": {
      "name": "anvil",
      "version": "<CARGO_PKG_VERSION>"
    }
  },
  "instructions": "<current server instructions>",
  "ttlMs": 3600000,
  "cacheScope": "private"
}
```

Do not advertise prompts, Tasks, Apps, subscriptions or deprecated client
features.

### 7.3 Modern successful results

Every successful modern result includes:

- `resultType: "complete"`;
- `_meta["io.modelcontextprotocol/serverInfo"]`;
- the method-specific result fields.

The server should also accept and propagate W3C Trace Context keys from request
metadata into the current OpenTelemetry span:

- `traceparent`;
- `tracestate`;
- `baggage`.

Malformed trace values are ignored or recorded as validation events. They do
not become authority inputs and are never blindly forwarded to subprocess
environments.

### 7.4 Cache policy

Initial conservative cache policy:

| Method | `ttlMs` | `cacheScope` | Rationale |
| --- | ---: | --- | --- |
| `server/discover` | 3,600,000 | `private` | Stable for the binary process |
| `tools/list` | 3,600,000 | `private` | Fixed catalogue, private local server |
| `resources/list` | 3,600,000 | `private` | Fixed catalogue, private local server |
| `resources/read` | 0 | `private` | Workspace and graph state may change immediately |

`tools/list`, `resources/list` and `resources/read` must include the required
cache fields in modern results. Legacy responses must not be changed merely to
add modern cache fields unless legacy client compatibility tests prove the
addition harmless.

### 7.5 Lifecycle

For modern requests:

- do not require `initialize`;
- do not accept `notifications/initialized` as a lifecycle signal;
- do not advertise or respond successfully to `ping`, `shutdown` or `exit`;
- ignore unknown notifications as required by JSON-RPC;
- stop the stdio process on EOF or external termination.

The current graph warm-up must move from `initialize` to server start or a
one-time lazy action before the first workspace-backed request. Warm-up remains
best-effort and must not delay `server/discover`.

For legacy requests, retain the existing initialise-era lifecycle.

## 8. Official Rust SDK adoption

### 8.1 Boundary

Use `rmcp` for:

- protocol DTOs and version constants;
- stdio transport;
- modern and legacy era negotiation;
- JSON-RPC request and response framing;
- `server/discover`;
- result metadata and cache fields;
- official error types;
- conformance-facing protocol behaviour.

Keep anvil code responsible for:

- the tool registry and descriptors;
- tool calls;
- resources and resource reads;
- credential enforcement;
- workspace containment;
- redaction;
- graph egress accounting;
- daemon calls and embedded fallbacks;
- anvil-specific result payloads.

### 8.2 Adoption gate

Before pinning `rmcp` v3:

1. Confirm a stable crates.io release targets final MCP `2026-07-28`.
2. Confirm the SDK's final wire matches the ratified schema.
3. Review the Rust SDK conformance roadmap. The release-candidate roadmap
   reports 39/40 server scenarios, with JSON Schema 2020-12 outstanding.
4. Prove that the SDK can retain:
   - the four MiB stdio frame ceiling;
   - stdout protocol purity;
   - clean EOF exit;
   - current Windows process behaviour;
   - anvil's synchronous domain handlers without blocking the runtime;
   - current startup and resident-memory budgets.
5. Run licence and dependency review.

If any item blocks adoption, record the exception in an ADR and implement the
typed internal adapter. Do not silently return to a permanent hand-written
dispatcher.

## 9. Work-item sketch (mirrored in APS module MCP26)

These sketches are design intent. Authoritative status, validation, and
dependencies live on the [MCP26 module](../modules/mcp-dual-era-support.aps.md).


### MCP26-001: Ratification and SDK readiness gate

**Intent:** Seal the final upstream contract and implementation path.

**Work:**

- Diff the final `2026-07-28` schema and changelog against the locked release
  candidate.
- Confirm the modern and legacy version matrix against supported clients.
- Evaluate stable `rmcp` v3 against section 8.2.
- Record an ADR choosing SDK adoption or the temporary typed adapter.
- Pin exact conformance-suite and SDK versions.

**Acceptance:**

- No implementation is based solely on a prerelease schema.
- The ADR names the fallback and removal condition if the SDK is not adopted
  immediately.

### MCP26-002: Extract anvil MCP domain handlers

**Intent:** Separate protocol concerns from anvil behaviour before changing the
wire.

**Work:**

- Move dispatch and domain invocation out of `commands/mcp.rs`.
- Introduce typed handler results for tools, resources and errors.
- Keep tool and resource implementations unchanged.
- Add golden domain-result fixtures independent of protocol era.

**Acceptance:**

- The same handler functions can be rendered as legacy or modern results.
- No handler needs to know the negotiated protocol version.

### MCP26-003: Dual-era stdio protocol host

**Intent:** Serve modern and legacy clients from one binary.

**Work:**

- Add modern per-request metadata parsing and validation.
- Add supported-version enforcement and `-32022`.
- Retain legacy initialisation for the sealed legacy set.
- Gate legacy-only lifecycle methods to the legacy path.
- Preserve the four MiB frame ceiling and stdout discipline.

**Acceptance:**

- A modern client calls a tool without initialising.
- A legacy client completes its existing initialisation and tool flow.
- An unknown modern version receives the required supported-version error.
- A modern `exit` notification does not terminate the server.

### MCP26-004: Discovery and capability declaration

**Intent:** Implement mandatory modern discovery.

**Work:**

- Implement `server/discover`.
- Return supported modern versions, tools/resources capabilities,
  instructions, server identity and cache fields.
- Keep the extension map absent or empty.
- Do not claim prompts, list-change notifications or subscriptions.

**Acceptance:**

- Discovery matches the final schema and golden fixture.
- Capability claims exactly match implemented methods.

### MCP26-005: Modern result envelopes and caching

**Intent:** Make every modern result conforming and cache-safe.

**Work:**

- Add `resultType: "complete"` to all successful modern results.
- Stamp server identity in result `_meta`.
- Add cache fields to discovery, lists and resource reads.
- Keep deterministic tool and resource ordering.
- Preserve legacy wire fixtures.

**Acceptance:**

- Modern `tools/list`, `tools/call`, `resources/list` and `resources/read`
  validate against the final schema.
- Workspace resource reads are private and immediately stale.

### MCP26-006: Lifecycle, warm-up and state terminology

**Intent:** Remove modern dependence on initialise-era lifecycle.

**Work:**

- Move graph warm-up out of `initialize`.
- Retain warm-up for both eras without delaying discovery.
- Rename session-based egress comments, errors and docs to process-local MCP
  egress budget.
- Audit all hidden cross-call state. Require explicit tool arguments or handles
  for future workflow state.

**Acceptance:**

- First modern workspace call receives the same graph warm-up behaviour.
- No modern implementation path requires an initialisation side effect.
- User-facing errors no longer tell a modern client to reconnect to reset a
  protocol session. They may say restart the local MCP process.

### MCP26-007: Modern activation verification

**Intent:** Verify the installed anvil entry without assuming a legacy
handshake.

**Work:**

- Probe `server/discover` on a disposable child process.
- Recognise valid modern discovery and modern protocol errors.
- On a non-modern error, timeout or early exit, reap the probe child and spawn a
  fresh child for legacy `initialize`.
- Read modern identity from result `_meta`.
- Add `protocolEra`, `protocolVersion` and `verificationMethod` to diagnostic
  evidence.
- Preserve the existing public tier label until an activation-diagnostic schema
  version explicitly renames it.

**Acceptance:**

- New anvil verifies through discovery.
- An older installed anvil binary verifies through legacy fallback.
- A failed probe leaves no child process behind.
- Linux, macOS and Windows tests cover timeout, early exit and malformed frames.

### MCP26-008: JSON Schema 2020-12 verification

**Intent:** Confirm every published tool descriptor is valid under the new
schema contract.

**Work:**

- Validate all 14 `inputSchema` values against JSON Schema 2020-12.
- Keep the required object root.
- Bound schema depth and validation time.
- Do not dereference external `$ref` values.
- Verify any future `outputSchema` and `structuredContent` pairing.

**Acceptance:**

- Catalogue validation passes for every tool.
- The official conformance scenario passes or a blocking upstream SDK issue is
  recorded.

### MCP26-009: Trace context and observability

**Intent:** Correlate MCP requests without treating client metadata as trusted.

**Work:**

- Extract valid W3C trace context from request `_meta`.
- Parent or link the MCP request span according to existing observability
  policy.
- Record protocol era, protocol version and MCP method on spans.
- Never record tool arguments, proposed source content, credentials or
  unredacted resource bodies.

**Acceptance:**

- A supplied valid `traceparent` joins the expected trace.
- Invalid metadata cannot panic the server or alter authorisation.

### MCP26-010: Conformance and client matrix

**Intent:** Prove protocol correctness and real-client compatibility.

**Work:**

- Run the official `2026-07-28` server conformance suite for advertised
  capabilities.
- Run the selected legacy suite.
- Add repository integration fixtures for:
  - discovery;
  - direct modern `tools/list` and `tools/call`;
  - direct modern resource list/read;
  - missing metadata;
  - unsupported version;
  - modern result metadata;
  - cache fields;
  - legacy initialise and lifecycle;
  - malformed and oversized frames.
- Update benchmark drivers and E2E smoke tests that currently initialise first.
- Exercise supported real clients in both modern and legacy modes where the
  client allows version selection.
- Run on Linux, macOS and Windows.

**Acceptance:**

- Applicable official server scenarios pass.
- No supported legacy client regression is open.
- Startup, first response and resident-memory budgets do not regress beyond an
  explicitly approved threshold.

### MCP26-011: Documentation and release

**Intent:** Make the compatibility posture clear without exposing unnecessary
  protocol detail to ordinary users.

**Work:**

- Update `docs/architecture/mcp-shim-as-built.md`.
- Replace the legacy protocol section in
  `docs/architecture/rust-mcp-server-spec.md`.
- Update `docs/public/anvil/integrations/mcp.md` with version support and
  verification troubleshooting.
- Update activation and support-reference documentation.
- Update release notes and changelog.
- Archive or supersede old APS prose that states the handshake is the required
  protocol.

**Acceptance:**

- No authoritative current document says modern MCP requires `initialize`.
- Release notes state that existing client configurations continue to work.

## 10. Expected file impact

| Area | Expected change |
| --- | --- |
| `Cargo.toml` | Workspace dependency if `rmcp` is adopted |
| `Cargo.lock` | SDK and transitive dependency lock |
| `crates/anvil-cli/Cargo.toml` | MCP SDK dependency and feature selection |
| `crates/workspace-hack/` | Dependency unification if required |
| `crates/anvil-cli/src/commands/mcp.rs` | Thin command host, no raw domain dispatch |
| `crates/anvil-cli/src/mcp/` | Protocol adapter, server, handlers and result shaping |
| `crates/anvil-cli/src/mcp/tools/registry.rs` | Descriptor adaptation and schema validation |
| `crates/anvil-cli/src/mcp/resources/` | Cache metadata adaptation and terminology |
| `crates/anvil-cli/src/activation/mcp_client.rs` | Disposable discovery probe and legacy fallback |
| `crates/anvil-cli/tests/mcp_serve_stdio.rs` | Dual-era integration coverage |
| `crates/anvil-bench/benches/` | Modern request fixtures and retained legacy benchmarks |
| `apps/e2e/src/smoke/smoke.e2e.test.ts` | Modern discovery/direct-call smoke |
| Architecture and public MCP docs | Modern protocol and compatibility posture |
| APS indexes and release notes | Module tracking and shipped behaviour |

`mcp_config.rs`, installer adapters and editor config files should not require
wire-shape changes because stdio launch configuration is unchanged.

## 11. Release sequencing

1. MCP26-001 seals the final specification and SDK decision.
2. MCP26-002 extracts domain handlers and golden fixtures.
3. MCP26-003, MCP26-004 and MCP26-005 implement the dual-era wire.
4. MCP26-006 and MCP26-007 remove hidden handshake assumptions.
5. MCP26-008 and MCP26-009 complete schema and observability work.
6. MCP26-010 runs conformance and the client/platform matrix.
7. MCP26-011 updates authority docs and releases the change.

MCP26-003 through MCP26-005 may be developed together, but they should not merge
without both modern and legacy golden fixtures. Do not ship modern-only support
while the generated client support matrix still contains legacy-only clients.

## 12. Definition of done

- `server/discover` works over stdio.
- A modern client can call every advertised anvil tool and read every
  advertised resource without `initialize`.
- Every modern successful result contains `resultType`.
- Every modern result carries server identity metadata unless the final
  specification changes that recommendation.
- Required cache fields are present with conservative private policy.
- Unsupported protocol versions return `-32022` with supported versions.
- Legacy initialise-era clients continue to work.
- Modern requests cannot trigger legacy lifecycle side effects.
- The activation verifier discovers modern anvil and falls back cleanly to old
  anvil.
- Tool schemas validate as JSON Schema 2020-12.
- Applicable official conformance suites pass.
- Linux, macOS and Windows CI pass.
- Security, redaction, workspace containment, frame limits and graph egress
  controls are preserved.
- Authoritative docs and release notes describe the dual-era posture.

## 13. Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Final schema changes after the RC | Low | High | MCP26-001 final diff gate |
| `rmcp` v3 is released late | Medium | Medium | Typed temporary adapter with removal ADR |
| SDK transport bypasses four MiB limit | Medium | High | Adoption spike and oversize conformance test |
| Legacy client rejects additive fields | Medium | High | Era-specific response rendering and real-client matrix |
| Probe kills legacy servers | Medium | Medium | Disposable modern probe, fresh legacy child |
| Cache leaks workspace data | Low | High | `private`, zero-TTL resource reads |
| Protocol refactor weakens anvil security controls | Low | High | Keep domain handlers, golden fixtures and security regression suite |
| SDK increases startup or binary cost | Medium | Medium | Benchmark gate before adoption |

## 14. Follow-on opportunities

These are intentionally not part of MCP26:

- Streamable HTTP once a supported client or hosted anvil surface requires it.
- MCP Apps for rich gate, finding and architecture visualisations.
- Tasks for long-running full-repository gates.
- MRTR for explicit human approval flows.
- `subscriptions/listen` if tool/resource catalogues become dynamic.
- Typed `outputSchema` plus native `structuredContent` for anvil tool results.

