# ADR-113: MCP 2026-07-28 dual-era stdio and rmcp adoption path

## Status

Accepted 2026-07-29

## Date

2026-07-29

## Context

anvil's Rust MCP server (`anvil mcp serve --stdio`) is a hand-written JSON-RPC
stdio host in `crates/anvil-cli/src/commands/mcp.rs`. It pins
`DEFAULT_PROTOCOL_VERSION` to `2024-11-05`, accepts `initialize`, echoes the
client-requested version without a supported-version set, and has no
`server/discover`, modern per-request `_meta`, `resultType`, or cache fields.
Activation verification drives a legacy `initialize` probe with protocol
`2025-06-18`.

MCP `2026-07-28` was ratified on 2026-07-28. It removes the modern
`initialize` / `initialized`
handshake and protocol-level sessions, requires `server/discover`, requires
protocol version and client capabilities in `params._meta` on every modern
request, requires `resultType` on successful results, adds cache hints, and
lifts tool schemas to JSON Schema 2020-12. The official Rust SDK (`rmcp`) has
released **v3.0.0**. Its built-in async read transport is not adopted here
because it uses unbounded `read_until`, which cannot preserve anvil's
mandatory four-MiB frame ceiling.

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
     `_meta`, `server/discover`, modern result envelopes). Operator-confirmed.
   - **Legacy:** full initialise-era set:
     `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05`. Operator
     confirmed and now sealed as **keep all four**.
   Unsupported modern versions return `UnsupportedProtocolVersion` with code
   **`-32022`** with `supported` / `requested` data. Never echo an arbitrary
   requested version.

2. **SDK-first protocol layer, temporary typed adapter selected.** Prefer
   the official `rmcp` crate once a stable release passes the adoption gate in
   the module specification §8.2 (frame ceiling, stdout purity, EOF exit,
   Windows behaviour, sync domain handlers, startup/memory budgets, licence
   review). Until that gate passes, MCP26 may implement a **temporary typed
   internal dual-era adapter** that uses protocol DTOs and the same
   conformance fixtures used by the SDK path. It must **not** add further
   ad hoc `serde_json::Value` branching in `commands/mcp.rs`.

3. **Do not pin `rmcp 3.0.0` into the product yet.** Stable release is
   necessary but not sufficient: the evaluated transport fails the frame
   ceiling. MCP26-012 owns a bounded custom transport or upstream remediation.

4. **Domain ownership stays with anvil.** Tool registry, tool calls, resources,
   credential enforcement, workspace containment, redaction, graph egress
   accounting, daemon calls, and anvil-specific payloads remain anvil-owned
   behind the protocol boundary.

5. **Removal condition for the temporary adapter.** If the temporary adapter
   ships, it is removed in a follow-up item once stable `rmcp` v3 (or later)
   passes the §8.2 gate and dual-era golden fixtures remain green. Record the
   pin versions and conformance suite versions in MCP26-001 closeout.

6. **Branch posture.** Ratification and this Accepted ADR lift the publication
   hold. Normal review, CI, merge, and release gates still apply.

7. **Cache policy (operator-approved).** Conservative private cache:
   `server/discover`, `tools/list`, and `resources/list` use `ttlMs=3600000`
   and `cacheScope=private`; `resources/read` uses `ttlMs=0` and
   `cacheScope=private`. Legacy responses do not gain modern cache fields
   unless proven harmless.

8. **Non-goals (operator-approved).** Streamable HTTP, OAuth/OIDC MCP auth,
   MCP Apps, Tasks, MRTR, subscriptions, Roots/Sampling/Logging, prompts, tool
   renames, resource URI changes, and editor config shape changes remain out of
   MCP26.

9. **Release window (operator-approved).** Ship dual-era support in the
   **first anvil release after MCP `2026-07-28` ratification**, or at latest
   **that release + 1** if SDK/adapter readiness or client matrix evidence
   needs one more window. Do not force into a cut already closed before
   ratification.

10. **Behaviour posture (operator-approved).** Process-local MCP egress budget
    terminology (not protocol sessions); graph warm-up off modern `initialize`
    without delaying `server/discover`; activation probes modern discovery on a
    disposable child then a fresh legacy child.

## Operator ratification (2026-07-27)

| Item | Decision |
| ---- | -------- |
| ADR direction A–F (dual-era, SDK-first, temporary typed adapter, no product beta pin, anvil domain ownership, branch until gate) | **Approved** |
| Modern version `2026-07-28` | **Yes** |
| Legacy matrix | **Sealed: keep all four** |
| Unsupported-version error code/shape | **Sealed: `-32022` with requested/supported data** |
| Cache policy | **Approved** |
| Non-goals | **Approved** |
| Session wording / warm-up / activation | **OK** |
| Release | **Next release after ratification, or next+1** |

The final schema diff, full legacy matrix, conformance pins, and typed-adapter
decision were sealed by MCP26-001 on 2026-07-29.

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
| Pin `rmcp 3.0.0` now | Official stable SDK | Built-in transport violates four-MiB frame ceiling |
| Wait for a bounded `rmcp` transport | Cleanest long-term | Unnecessarily blocks ratified dual-era support |

## Consequences

- **Positive:** Clear gate for MCP26-002+; no silent return to permanent ad hoc
  JSON dispatch; dual-era compatibility preserved for MCPX clients.
- **Negative:** Temporary dual maintenance if the adapter ships before stable
  `rmcp` v3; ADR must be amended with exact pins at closeout.
- **Risks:** SDK frame-limit, runtime, Windows, or sync-handler mismatches.
- **Mitigations:** Keep the bounded typed adapter; retain golden fixtures for
  both eras; adopt `rmcp` only after MCP26-012 closes the full gate.

## Sealed pins and boundaries

| Artefact | Sealed value |
| -------- | ------------ |
| Modern protocol | `2026-07-28` |
| Legacy protocols | `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05` |
| Unsupported modern error | `-32022` `UnsupportedProtocolVersion` |
| `rmcp` product pin | None; `3.0.0` evaluated and deferred to MCP26-012 |
| Conformance suite (modern) | `0.2.0-alpha.10`, source `49103de6ed70804e940637bf3e9e29e4a3f54e64` |
| Conformance suite (legacy) | `0.1.16` |

## References

- APS module: [MCP26](../modules/mcp-dual-era-support.aps.md)
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
