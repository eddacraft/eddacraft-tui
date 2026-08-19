# anvil dashboard server

| Type   | Authority     | Owner | Status | Freshness                                                                                                                                         |
| ------ | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| README | Authoritative | DASH  | Live   | Last reviewed 2026-08-20 against `d6c8b565c`, `crates/anvil-dashboard-server/src/server.rs`, and `crates/anvil-dashboard-server/src/workspace.rs` |

| Upstream                                                                                                          | Downstream                                                          |
| ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `crates/anvil-dashboard-server/src/**`, ADR-104, ADR-123, `apps/dashboard/`, and `docs/guides/local-dashboard.md` | anvil CLI, dashboard web app, and this component's architecture doc |

This DASH-owned Rust crate serves the embedded browser application and a
read-only API for one local workspace. It owns loopback transport guards,
capability loading, bounded workspace reads, and the OpenAPI contract. It is not
a remotely deployable or multi-user service.

## Entry points

- [`src/lib.rs`](src/lib.rs) exposes the server library.
- [`src/main.rs`](src/main.rs) provides the standalone binary.
- [`src/server.rs`](src/server.rs) defines routes, the UI fallback, and loopback
  request guards.
- [`src/capabilities/`](src/capabilities) loads protection, history, pattern,
  and plan artefacts.
- [`src/workspace.rs`](src/workspace.rs) enforces canonical, capped reads.
- [`src/openapi.rs`](src/openapi.rs) publishes the client contract.

## Local validation

```bash
cargo test -p eddacraft-anvil-dashboard-server
```

When the API changes, also run the dashboard's contract check:

```bash
pnpm --filter @eddacraft/anvil-dashboard check:api
```

## Architecture and authorities

Read the [local architecture](ARCHITECTURE.md) for capability and trust
boundaries. The [local dashboard guide](../../docs/guides/local-dashboard.md)
owns operator behaviour. The
[TUI as-built](../../docs/architecture/tui-as-built.md) remains the retained
central dashboard-surface map pending DOCRB-005.
