# anvil dashboard server architecture

| Type         | Authority | Owner | Status | Freshness                                                                                                                                                                                              |
| ------------ | --------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Architecture | Derived   | DASH  | Live   | Last reviewed 2026-08-20 against `d6c8b565c`, `crates/anvil-dashboard-server/src/server.rs`, `crates/anvil-dashboard-server/src/workspace.rs`, and `crates/anvil-dashboard-server/src/capabilities/**` |

| Upstream                                                                                                                                        | Downstream                                          |
| ----------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| `crates/anvil-dashboard-server/src/**`, `apps/dashboard/src/api/generated/openapi.d.ts`, ADR-104, ADR-123, and `docs/guides/local-dashboard.md` | Dashboard client, anvil CLI, and server maintainers |

> **DOCRB-004 pilot:** this source-linked local map does not replace the
> retained central [TUI as-built](../../docs/architecture/tui-as-built.md) or
> the operator [local dashboard guide](../../docs/guides/local-dashboard.md).
> DOCRB-005 owns central migration.

## Scope and boundaries

The server owns one read-only, single-workspace loopback surface.
[`server.rs`](src/server.rs) refuses non-loopback listeners. This is a
machine-local unauthenticated boundary: any process on the machine can address
the service, so loopback does not authenticate a caller or provide per-user
isolation. Host, Origin, and fetch-site checks mitigate browser CSRF; local
non-browser clients can omit or forge those headers. The service must not be
exposed through a proxy or port-forward.

## Capability and access flow

This diagram owns the server's request-to-workspace concern.

```mermaid
flowchart LR
    Request[Browser or local API request] --> Guard[loopback and browser request-policy checks]
    Guard --> Route[read-only route]
    Route --> Capability[protection, history, patterns, or plans loader]
    Capability --> Workspace[canonical WorkspaceAnchor read]
    Workspace --> Artefact[bounded local artefact]
    Artefact --> Response[JSON response or embedded UI]
```

The guard and route nodes trace to [`server.rs`](src/server.rs). Capability
loaders trace to [`capabilities/`](src/capabilities), and the read boundary
traces to [`workspace.rs`](src/workspace.rs). In prose: a request that passes
the server's loopback and browser-request policy selects a read-only route; the
matching capability asks the anchored workspace boundary for a capped artefact;
the route returns structured JSON or an embedded UI asset.

## Invariants, failure, and fallback

- `serve` rejects a non-loopback listener before accepting requests.
- Host, Origin, and fetch-site checks mitigate cross-origin browser requests;
  they are not caller authentication and cannot be trusted for local-client
  identity.
- `WorkspaceAnchor` reads are relative to a canonical root, reject path escape
  and symlinks, and cap each artefact at 4 MiB.
- Missing, unsafe, oversized, or unavailable artefacts become structured API
  errors or honest empty states rather than fabricated protection results.
- A development binary without bundled UI returns an explanatory 503 for the
  browser shell while the read-only API remains available.
- API payloads default to `Cache-Control: no-store`; only fingerprinted assets
  opt into immutable caching.
- Current limitation: loopback provides neither caller authentication nor
  per-user isolation. A capability token or owner-only IPC transport would be
  required to establish per-user isolation.

The generated contract boundary is defined in [`openapi.rs`](src/openapi.rs) and
consumed by the dashboard's
[`generate-api.mjs`](../../apps/dashboard/scripts/generate-api.mjs).
