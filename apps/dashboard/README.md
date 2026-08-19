# anvil dashboard web application

| Type   | Authority     | Owner | Status | Freshness                                                                                                           |
| ------ | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| README | Authoritative | DASH  | Live   | Last reviewed 2026-08-20 against `d6c8b565c`, `apps/dashboard/src/main.tsx`, and `apps/dashboard/src/api/client.ts` |

| Upstream                                                                                                                        | Downstream                                                |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `apps/dashboard/src/**`, `crates/anvil-dashboard-server/src/openapi.rs`, ADR-104, ADR-123, and `docs/guides/local-dashboard.md` | Browser contributors and `apps/dashboard/ARCHITECTURE.md` |

This DASH-owned React application is the read-only browser interface for a local
anvil workspace. It renders protection, gate, warning, and plan data from the
same-origin loopback dashboard server. It does not scan the workspace or provide
mutation controls.

## Entry points

- [`src/main.tsx`](src/main.tsx) installs query and routing providers.
- [`src/router.tsx`](src/router.tsx) defines browser navigation.
- [`src/api/client.ts`](src/api/client.ts) wraps the generated OpenAPI contract.
- [`src/api/generated/openapi.d.ts`](src/api/generated/openapi.d.ts) is the
  generated client type surface.
- [`scripts/generate-api.mjs`](scripts/generate-api.mjs) regenerates and
  verifies that surface from the Rust server contract.

## Local validation

```bash
pnpm --filter @eddacraft/anvil-dashboard test
pnpm --filter @eddacraft/anvil-dashboard typecheck
pnpm --filter @eddacraft/anvil-dashboard check:api
pnpm --filter @eddacraft/anvil-dashboard build
```

## Architecture and authorities

Read the [local architecture](ARCHITECTURE.md) for the UI-to-server flow. The
authoritative operator behaviour is in the
[local dashboard guide](../../docs/guides/local-dashboard.md); this README does
not restate its rollout or usage contract. The retained central
[TUI as-built](../../docs/architecture/tui-as-built.md) remains the wider
dashboard-surface map until DOCRB-005 decides its disposition.
