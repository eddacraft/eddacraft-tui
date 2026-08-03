# MCP 2026-07-28 Dual-Era Support

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| MCP26 | —     | In Progress | 11/12    |

**Last reviewed:** 2026-08-03 — dual-era MCP `2026-07-28` support (MCP26-001
through MCP26-011) merged to `main` via PR #3444. Code lands under
`crates/anvil-cli/src/mcp/protocol/` (dispatch, domain, versions, trace, and
related). ADR-113 is Accepted. MCP26-013 merged via PR #3487 on 2026-08-03,
repairing the observed legacy request-metadata interoperability regression;
MCP26-012 remains Ready for product adoption of stable `rmcp` (temporary typed
adapter still in use).
Release: first post-ratification cut or next+1.

**Publication:** Merged via PR
[#3444](https://github.com/eddacraft/anvil-001/pull/3444) on 2026-07-30
(merge commit `41de26aaa` and related dual-era commits on `main`). Covers
MCP26-001..011. Actual-client matrix evidence was deferred as residual operator
work; official stdio conformance remains locally evidenced (HTTP-only official
runner not applicable). Actual-client follow-up exposed an era-selection
regression when a legacy client attaches standard request metadata.
MCP26-013's compatibility repair merged via PR #3487 on 2026-08-03. MCP26-012
(rmcp product pin / adapter removal) stays Ready.

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
- ADR (Accepted):
  [`plans/decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md`](../decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md)
- [Ratified MCP `2026-07-28`](https://modelcontextprotocol.io/specification/2026-07-28)
- [Final changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
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
| Standard legacy request metadata is mistaken for modern protocol intent | Observed | High | Classify modern intent by reserved namespace; retain Codex/rmcp-shaped regression fixtures |
| Modern probe leaves orphan children or kills legacy path | Medium | Medium | Disposable modern probe, fresh legacy child |
| Cache policy leaks workspace data | Low | High | `private` scope; zero-TTL resource reads |
| Protocol refactor weakens security controls | Low | High | Domain-handler extraction + security regression suite |
| SDK raises startup or resident memory cost | Medium | Medium | Benchmark gate before adoption |

## Ready Checklist

Change module status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies and non-goals identified
- [x] Work items MCP26-001..013 defined with intent, outcome, validation
- [x] MCP26-001 seals the final schema, legacy matrix, and SDK/adapter ADR
- [x] No dual-era implementation PR merged before MCP26-001 closed its
      ratification hold

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
| Hotfix | MCP26-013 | Restore legacy request-metadata interoperability |
| 7 | MCP26-012 | Official `rmcp` product pin and temporary-adapter removal |

MCP26-003..005 may develop together but must not merge without both modern and
legacy golden fixtures. MCP26-012 runs after MCP26-001 authorises the SDK path
(or after a stable `rmcp` release exists if 001 closed on the temporary
adapter). MCP26-013 is independent of the SDK adoption and lands first;
MCP26-012 must preserve its real-client-shaped regression fixtures.

## Work Items

### MCP26-001: Ratification and SDK readiness gate

- **Status:** Merged 2026-07-30 via PR #3444
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
- **Evidence (2026-07-29):** Final schema and changelog diff sealed; all four
  legacy versions retained; `rmcp 3.0.0` evaluated and deferred because its
  built-in transport cannot enforce the four-MiB frame ceiling; temporary typed
  adapter selected; modern/legacy conformance runner pins recorded; ADR-113
  Accepted. The ratification hold is lifted.
- **Confidence:** medium

### MCP26-002: Extract anvil MCP domain handlers

- **Status:** Merged 2026-07-30 via PR #3444
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
- **Evidence:** Domain handlers under `mcp/protocol/domain.rs`; thin stdio host;
  merged with dual-era support on `main`.
- **Confidence:** high

### MCP26-003: Dual-era stdio protocol host

- **Status:** Merged 2026-07-30 via PR #3444
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

- **Status:** Merged 2026-07-30 via PR #3444
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

- **Status:** Merged 2026-07-30 via PR #3444
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

- **Status:** Merged 2026-07-30 via PR #3444
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

- **Status:** Merged 2026-07-30 via PR #3444
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

- **Status:** Merged 2026-07-30 via PR #3444
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
- **Evidence:** Catalogue Draft 2020-12 tests green (test-only module;
  `jsonschema` remains a dev-dep); official runner applicability recorded in
  MCP26-010.
- **Confidence:** medium

### MCP26-009: Trace context and observability

- **Status:** Merged 2026-07-30 via PR #3444
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
- **Evidence:** `mcp/protocol/trace.rs` extracts valid W3C `traceparent` from
  request `_meta`, binds to the current span, records protocol era/version/method;
  malformed values ignored (no panic).
- **Confidence:** medium

### MCP26-010: Conformance and client matrix

- **Status:** Merged 2026-07-30 via PR #3444 — actual-client matrix deferred
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
  - `plans/audits/2026-07-29-mcp26-conformance-applicability.md`
- **Dependencies:** MCP26-003, MCP26-004, MCP26-005, MCP26-006, MCP26-007,
  MCP26-008, MCP26-009
- **Validation:** Official applicable server scenarios pass; no open supported
  legacy client regression; platform CI green; budget gate recorded
- **Evidence:** Local modern/legacy stdio and E2E evidence green; official
  runner is HTTP-only and documented as not applicable to anvil's stdio
  transport; local resource budgets pass. Actual-client matrix evidence deferred
  as residual operator work after merge.
- **Confidence:** medium

### MCP26-011: Documentation and release

- **Status:** Merged 2026-07-30 via PR #3444
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

### MCP26-012: Adopt official Rust MCP SDK (`rmcp`) for protocol framing

- **Status:** Ready
- **Intent:** Replace the temporary dual-era protocol adapter with the official
  Rust MCP SDK (`rmcp`) once a stable release targets final MCP `2026-07-28`
  and passes the adoption gate, so protocol framing is not permanently owned
  by hand-written anvil code.
- **Expected Outcome:**
  - Product `Cargo.toml` / `crates/anvil-cli/Cargo.toml` pin a **stable**
    `rmcp` (and any required companion crates) that implements final
    `2026-07-28` dual-era stdio behaviour.
  - Spec §8.2 adoption gate is evidenced: final wire match; four MiB frame
    ceiling; stdout purity; clean EOF exit; Windows process behaviour;
    anvil synchronous domain handlers without blocking the runtime; startup
    and resident-memory budgets; licence review.
  - Temporary adapter under `crates/anvil-cli/src/mcp/protocol/` is removed or
    reduced to a thin `rmcp` integration seam; dual-era golden fixtures and
    `mcp_serve_stdio` / activation probe contracts stay green.
  - Domain ownership remains anvil-only (tools, resources, auth, redaction,
    workspace containment, graph egress, daemon/embedded fallbacks).
  - ADR-113 is **Accepted** or **Amended** with exact `rmcp` and conformance
    suite pins and the adapter removal record.
- **Out of item:**
  - Shipping a **product** pin of `rmcp` `3.0.0-beta.*` (or other pre-release)
    without operator exception — betas may be used only for non-shipped
    evaluation spikes on a work branch (ADR-113).
  - Tier-1 SDK betas (Python / TypeScript / Go / C#) — wrong stack for anvil
    stdio host.
  - Streamable HTTP, OAuth, Apps, Tasks, MRTR (remain MCP26 non-goals).
- **Files:**
  - `Cargo.toml` / `crates/anvil-cli/Cargo.toml` / `Cargo.lock`
  - `crates/anvil-cli/src/mcp/protocol/**` (adapter removal or `rmcp` seam)
  - `crates/anvil-cli/src/commands/mcp.rs` (host wiring)
  - `crates/anvil-cli/src/mcp/**` domain modules (unchanged contracts)
  - `crates/anvil-cli/tests/mcp_serve_stdio.rs`
  - `crates/anvil-cli/tests/mcp_activation_probe.rs`
  - `plans/decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md`
  - `plans/audits/` (adoption-gate evidence note)
- **Dependencies:**
  - MCP26-001 (final schema seal + SDK path authorised; closed on temporary
    adapter — product `rmcp` pin still required here)
  - MCP26-002..011 dual-era contracts and fixtures merged via PR #3444
  - MCP26-013 request-metadata interoperability contract and regression
    fixtures
  - crates.io stable `rmcp` ≥ target major targeting final `2026-07-28`
  - Prefer with conformance evidence from MCP26-010 so official suite version
    binds the pin (stdio local evidence already recorded; HTTP-only official
    runner remains not applicable)
- **Validation:**
  - `cargo test -p eddacraft-anvil --test mcp_serve_stdio`
  - `cargo test -p eddacraft-anvil --test mcp_activation_probe`
  - `cargo test -p eddacraft-anvil --bin anvil protocol::`
  - Oversize-frame / EOF / dual-era golden fixtures remain green under `rmcp`
  - Adoption-gate checklist in audit note all checked or ADR exception recorded
  - `pnpm docs:check` if architecture wording changes for SDK boundary
- **Confidence:** medium
- **Notes:** Temporary adapter path is merged to `main` (PR #3444) and delivers
  dual-era behaviour under `crates/anvil-cli/src/mcp/protocol/`. This item is
  the ADR-113 **removal condition** for that adapter. Evaluation spikes against
  `rmcp` betas are allowed on a work branch only and must not merge as the
  permanent product pin.

### MCP26-013: Restore legacy request-metadata interoperability

- **Status:** Merged 2026-08-03 via PR #3487
- **Intent:** Restore supported initialise-era clients that attach standard
  per-request metadata, without allowing malformed modern requests to fall
  back to the legacy protocol path.
- **Expected Outcome:**
  - Era selection treats `server/discover` and any reserved
    `io.modelcontextprotocol/*` request metadata key as modern intent; the
    presence of `_meta` alone is not a modern discriminator.
  - After successful legacy initialisation, `tools/list` and `tools/call`
    requests carrying standard metadata such as `_meta.progressToken` keep
    legacy dispatch and response envelopes.
  - Partial reserved modern metadata remains on the modern parser and fails
    closed with `-32602`; valid modern requests and unsupported-version
    `-32022` responses remain unchanged.
  - Codex/rmcp-shaped stdio fixtures reproduce the installed-client request
    shape so later protocol-layer changes cannot silently reintroduce the
    regression.
- **Out of item:**
  - Adopting `rmcp` or removing the temporary adapter (MCP26-012).
  - Changing MCP client configuration, daemon lifecycle, tool names, tool
    schemas, resource URIs, or domain-handler semantics.
  - Adding a new transport or widening the sealed legacy-version matrix.
- **Files:**
  - `crates/anvil-cli/src/mcp/protocol/meta.rs`
  - `crates/anvil-cli/tests/mcp_serve_stdio.rs`
  - `plans/modules/mcp-dual-era-support.aps.md`
  - `plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md`
  - `plans/index.aps.md`
- **Dependencies:** MCP26-001..011 merged via PR #3444. This item does not
  depend on MCP26-012; MCP26-012 must preserve the resulting compatibility
  contract and fixtures.
- **Risk:** high — era selection is a public dual-era protocol boundary where
  permissive fallback could hide malformed modern requests and strict
  detection can break supported legacy clients.
- **Design source:**
  [`plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md` §6.2](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md#62-era-selection)
  and [ADR-113](../decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md).
- **Rollback:** Revert the classifier change as one bounded protocol-layer
  patch while retaining the new fixtures as evidence; do not broaden legacy
  fallback or remove strict reserved-namespace validation to force a pass.
- **Validation:**
  - `cargo test -p eddacraft-anvil --bin anvil protocol::`
  - `cargo test -p eddacraft-anvil --test mcp_serve_stdio`
  - `cargo test -p eddacraft-anvil --test mcp_activation_probe`
  - `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`
  - `cargo fmt --all --check`
  - `pnpm docs:check`
  - `pnpm aps:active-lint`
  - `pnpm aps:index:check`
  - `pnpm aps:drift --json`
  - Independent `verify-loop` against the captured Codex/rmcp request shape,
    valid modern traffic, partial modern metadata, and unsupported versions
- **Evidence (2026-08-03):**
  - Root evidence gate: 40 protocol unit tests, 52 hermetic stdio tests, and the
    activation probe passed; clippy with warnings denied, Rust formatting,
    repository formatting, documentation checks, APS lint/index checks, and
    diff hygiene all passed.
  - Independent post-repair `verify-loop`: Pass against base
    `16fa44c1d4b6e57ad905c98b06e7bacea2e5312f`, including malformed unsupported
    metadata precedence and mixed/lookalike metadata probes.
  - Council session `council-187f6579`: Converged after all three minor
    findings were fixed and independently re-reviewed. No findings remain.
  - Residual: no live Codex/rmcp process was launched during verification; the
    black-box stdio fixtures preserve its captured request shape.
- **Publication:**
  [PR #3487](https://github.com/eddacraft/anvil-001/pull/3487) against `main`;
  merged 2026-08-03 as `0db56985214ae885494c21b4798d3358232f1553`.
  Release evidence remains pending.
- **Confidence:** high
- **changeType:** fix
- **releaseIntent:** candidate
- **releaseScope:** patch
- **releaseNote:**
  - **audience:** developer
  - **type:** fixed
  - **text:** Restored Codex and other initialise-era MCP clients that attach
    standard request metadata.

## Decisions

1. **Dual-era, not modern-only** — retain initialise-era stdio for the sealed
   legacy client set until the support matrix no longer needs it.
   Operator-ratified 2026-07-27.
2. **SDK-first protocol layer** — prefer stable `rmcp` v3; temporary typed
   adapter only under MCP26-001 / ADR-113 with an explicit removal condition.
   Operator-ratified 2026-07-27.
3. **Domain handlers stay anvil-owned** — tools, resources, auth, redaction,
   containment, and egress accounting never move into the protocol SDK.
   Operator-ratified 2026-07-27.
4. **Era-neutral results, era-specific envelopes** — handlers must not inject
   protocol fields; the adapter owns `resultType`, server identity, and cache
   metadata.
5. **Conservative private caching** — fixed catalogues may cache for one hour
   privately; workspace resource reads are immediately stale.
   Operator-ratified 2026-07-27.
6. **Process-local egress, not protocol sessions** — modern MCP has no session
   concept; existing byte budgets remain process-local security accounting.
   Operator-ratified 2026-07-27.
7. **Activation probes modern first** — discovery on a disposable child, then a
   fresh legacy child; never leave probe processes behind.
   Operator-ratified 2026-07-27.
8. **Branch until ratification** — satisfied 2026-07-29; dual-era implementation
   (MCP26-001..011) merged 2026-07-30 via PR #3444; MCP26-012 remains open for
   product `rmcp` adoption.
9. **Legacy matrix sealed** — keep `2025-11-25`, `2025-06-18`,
   `2025-03-26`, and `2024-11-05`.
10. **Unsupported-version error sealed** — `-32022` with `requested` and
    `supported` data.
11. **Release window** — ship in the first anvil release after MCP
    `2026-07-28` ratification, or at latest that release + 1.
    Operator-ratified 2026-07-27.
12. **Modern intent uses the reserved namespace, not `_meta` presence** —
    `server/discover` or any `io.modelcontextprotocol/*` request metadata key
    selects strict modern validation. Standard metadata remains legacy after
    initialisation, while partial modern metadata fails closed rather than
    falling back. Operator-ratified 2026-08-03.

## Notes

- Dual-era host (MCP26-001..011) is on `main` via PR #3444. MCP26-013's
  compatibility hotfix is on `main` via PR #3487. MCP26-012 (rmcp product
  adoption / temporary-adapter removal) remains the independent follow-on and
  must retain the MCP26-013 fixtures.

- Spec non-goals (HTTP, Apps, Tasks, MRTR, subscriptions) remain follow-on
  opportunities outside MCP26.
- RMCPF still owns remaining full-port leftovers (RMCPF-021 transport decision,
  RMCPF-030 compatibility harness vs archived TS, RMCPF-031 archive closeout).
  MCP26 owns the protocol-version dual-era cut, not tool/resource parity.
- Stored progress `11/12` is advisory (ADR-053). Item `Status:` lines are
  authoritative.
- MCP26-012 filed 2026-07-28 after clawpatch dual-era pass: product still on
  temporary typed adapter after PR #3444; official `rmcp` adoption is the
  follow-on SDK item.
