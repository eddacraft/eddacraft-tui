# ADR-113: MCP 2026-07-28 dual-era stdio and rmcp adoption path

## Status

Proposed

## Date

2026-07-27

## Context

anvil's Rust MCP server (`anvil mcp serve --stdio`) is a hand-written JSON-RPC
stdio host in `crates/anvil-cli/src/commands/mcp.rs`. It pins
`DEFAULT_PROTOCOL_VERSION` to `2024-11-05`, accepts `initialize`, echoes the
client-requested version without a supported-version set, and has no
`server/discover`, modern per-request `_meta`, `resultType`, or cache fields.
Activation verification drives a legacy `initialize` probe with protocol
`2025-06-18`.

MCP `2026-07-28` (release candidate locked 2026-05-21; final publication
scheduled 2026-07-28) removes the modern `initialize` / `initialized`
handshake and protocol-level sessions, requires `server/discover`, requires
protocol version and client capabilities in `params._meta` on every modern
request, requires `resultType` on successful results, adds cache hints, and
lifts tool schemas to JSON Schema 2020-12. The official Rust SDK
(`rmcp`) tracks this work toward a **v3.0.0** release; as of 2026-07-27 the
crates.io stable max is **2.2.0**, with **3.0.0-beta.2** published. The SDK
roadmap reports 2026-07-28 server conformance **39/40** (outstanding
`json-schema-2020-12`) on suite `0.2.0-alpha.9`.

anvil must keep supporting existing MCPX clients that still speak the
initialise-era protocol over stdio, while accepting modern clients. The
module specification is
[`plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md`](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md)
(MCP26). Gate evidence lives in
[`plans/audits/2026-07-27-mcp26-001-ratification-gate.md`](../audits/2026-07-27-mcp26-001-ratification-gate.md).

This ADR records the implementation path so MCP26 does not expand the
hand-written dispatcher by default and does not pin a pre-release SDK without
an explicit fallback.

## Decision

1. **Dual-era stdio server.** anvil supports:
   - **Modern:** protocol version `2026-07-28` only (stateless, per-request
     `_meta`, `server/discover`, modern result envelopes).
   - **Legacy:** sealed initialise-era set (provisional until MCP26-001
     closeout): `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05`.
   Unsupported modern versions return `UnsupportedProtocolVersion` with code
   **`-32022`** (per draft renumber) and `supported` / `requested` data. Never
   echo an arbitrary requested version.

2. **SDK-first protocol layer, temporary typed adapter authorised.** Prefer
   the official `rmcp` crate once a **stable** crates.io release (expected
   **v3.0.0**) targets final MCP `2026-07-28` and passes the adoption gate in
   the module specification §8.2 (frame ceiling, stdout purity, EOF exit,
   Windows behaviour, sync domain handlers, startup/memory budgets, licence
   review). Until that gate passes, MCP26 may implement a **temporary typed
   internal dual-era adapter** that uses protocol DTOs and the same
   conformance fixtures planned for the SDK path. It must **not** add further
   ad hoc `serde_json::Value` branching in `commands/mcp.rs`.

3. **Do not pin `rmcp` 3.0.0-beta.* into the product dependency graph** as the
   permanent path without operator acceptance. Betas may be used only for
   non-shipped evaluation spikes on this feature branch.

4. **Domain ownership stays with anvil.** Tool registry, tool calls, resources,
   credential enforcement, workspace containment, redaction, graph egress
   accounting, daemon calls, and anvil-specific payloads remain anvil-owned
   behind the protocol boundary.

5. **Removal condition for the temporary adapter.** If the temporary adapter
   ships, it is removed in a follow-up item once stable `rmcp` v3 (or later)
   passes the §8.2 gate and dual-era golden fixtures remain green. Record the
   pin versions and conformance suite versions in MCP26-001 closeout.

6. **Branch posture.** Dual-era implementation and MCP26-001 closeout stay on
   a feature branch until the final specification is ratified and this ADR is
   Accepted (or Amended with final pins).

## Rationale

Extending the current hand-written host for every modern field (version
negotiation, metadata validation, result envelopes, cache hints, error codes)
recreates an SDK. The official SDK already targets dual-version support and
conformance. A short-lived typed adapter preserves schedule if stable v3 lags
ratification, without normalising permanent protocol ownership inside anvil.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| **Chosen:** dual-era + SDK-first + temporary typed adapter | Conformance leverage; schedule insurance; domain isolation | Two-phase migration if adapter ships first |
| Permanent hand-written dual-era host | Full control; no new dependency | High maintenance; diverges from official wire; already at scale limit |
| Modern-only (drop legacy) | Smaller surface | Breaks MCPX clients that still initialise |
| Pin `rmcp` 3.0.0-beta.2 now | Early integration | Pre-release; final schema not sealed; not acceptable as permanent product pin without operator exception |
| Wait for stable `rmcp` v3 with no adapter | Cleanest long-term | Blocks all dual-era progress if v3 slips |

## Consequences

- **Positive:** Clear gate for MCP26-002+; no silent return to permanent ad hoc
  JSON dispatch; dual-era compatibility preserved for MCPX clients.
- **Negative:** Temporary dual maintenance if the adapter ships before stable
  `rmcp` v3; ADR must be amended with exact pins at closeout.
- **Risks:** Final schema renames (e.g. `resultType` string values, error code
  numbers); SDK frame-limit or sync-handler mismatches; beta API churn.
- **Mitigations:** MCP26-001 final-vs-RC diff before wire merge; adoption spike
  before pin; golden fixtures for both eras; branch held until ratification.

## Provisional pins (amend at MCP26-001 closeout)

| Artefact | Provisional value (2026-07-27) | Seal condition |
| -------- | ------------------------------ | -------------- |
| Modern protocol | `2026-07-28` | Final schema published |
| Legacy protocols | `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05` | Confirm against MCPX clients post-ratification |
| Unsupported modern error | `-32022` `UnsupportedProtocolVersion` | Confirm in final schema |
| `rmcp` product pin | *none yet* (stable max `2.2.0`; newest `3.0.0-beta.2`) | Stable ≥3.0.0 targeting final + §8.2 gate |
| Conformance suite (modern) | roadmap baseline `0.2.0-alpha.9` (39/40 server) | Final suite version for `2026-07-28` |
| Conformance suite (legacy) | roadmap `0.1.16` for `2025-11-25` | Selected legacy suite for sealed matrix |

## References

- APS module: [MCP26](../modules/mcp-2026-07-28-dual-era-support.aps.md)
- Spec: [2026-07-27 dual-era support](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md)
- Gate audit: [2026-07-27 MCP26-001](../audits/2026-07-27-mcp26-001-ratification-gate.md)
- Related ADRs: [033](033-park-ide-mcp-retire-ts-scanner.md),
  [044](044-mcp-entry-activation-owned.md),
  [083](083-gctx-mcp-delivery-target.md),
  [092](092-mcp-optional-activation-spine.md),
  [106](106-agent-integration-registry-and-managed-installers.md)
- External: [RC announcement](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/),
  [draft changelog](https://modelcontextprotocol.io/specification/draft/changelog),
  [rust-sdk ROADMAP](https://github.com/modelcontextprotocol/rust-sdk/blob/main/ROADMAP.md),
  [crates.io rmcp](https://crates.io/crates/rmcp)
