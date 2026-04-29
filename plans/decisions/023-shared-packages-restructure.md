# ADR-023: Shared Packages Restructure — Retire platform/, Consolidate into shared/

## Status

Proposed

> **Partially superseded (2026-04-29) by [ADR-033](./033-park-ide-mcp-retire-ts-scanner.md).**
> The "Distribution Targets" table and 2026-04-02 README audit
> below list `packages/mcp-server` and `packages/vscode-extension`
> as user-facing shippable packages. Both are now archived under
> `archive/anvil-mcp-server/` and
> `archive/anvil-vscode-extension/` per ADR-033 and no longer ship
> from npm or the VS Code Marketplace. The rest of this ADR's
> shared-packages restructure decision is unaffected.

## Date

2026-04-02

## Context

The monorepo currently has three overlapping shared-code locations:

- **`packages/platform/`** — Infrastructure concerns (config, crypto, storage).
  Created during CRB as extraction targets from `anvil-core`. Config and crypto
  were duplicated into `anvil-core` but the originals were never removed.
  Storage has a solid `IStorageProvider` implementation with path traversal
  protection, but nothing wires it in — the runtime uses raw `fs` calls.

- **`packages/shared/`** — Placeholder (README only) planned for v1.1+. Was
  intended for pure utilities, test helpers, and branded types.

- **`packages/json-render/`** — Dashboard spec schema and web component
  renderer. Currently has zero consumers but is the foundation for DASH
  (web dashboard) and TUIDASH (Ratatui dashboard). A Ratatui counterpart
  (`crates/anvil-tui-render`) is planned under TUIDASH-009/010.

Additionally, the monorepo has unaddressed gaps:

- **Auth** — Fragmented across `apps/anvil-api/src/middleware/admin-auth.ts`
  (API) and `crates/anvil-cli/src/auth/` (Rust CLI) with no shared contracts.
  BAUTH was a beta bridge; real auth (teams, RBAC, enterprise SSO, API keys)
  will span web dashboard, CLI, API, MCP server, and VS Code extension.

- **Telemetry** — Completely absent. No OpenTelemetry, no structured logging
  beyond `console.debug` wrappers. The Rust side has no `tracing` crate either.

The top-level `infra/` directory is already used for Pulumi IaC, so
`packages/infra/` would create a naming collision.

## Decision

Retire `packages/platform/`. Consolidate cross-cutting shared code into
`packages/shared/` (honouring its original intent) and promote
`packages/json-render/` to `packages/libs/render/`.

### Target layout

```
packages/shared/                 # Cross-cutting, no @eddacraft/anvil-* domain deps
                                 # (port/contract packages like anvil-ports are
                                 #  allowed as interface boundaries)
├── auth/                        # Token validation, RBAC types, session contracts
├── storage/                     # IStorageProvider impl (from platform/storage)
├── telemetry/                   # Structured logging, metrics, OTel protocol
├── testing/                     # Test helpers, fixtures, mocks
└── types/                       # Branded types, shared type utilities

packages/libs/                   # Shared libraries (not infrastructure)
└── render/                      # Dashboard spec schema + web renderer
                                 # (renamed from json-render)
                                 # Ratatui counterpart: crates/anvil-tui-render
```

### What moves

| Source | Destination | Action |
|--------|------------|--------|
| `packages/platform/storage/` | `packages/shared/storage/` | Move as-is |
| `packages/platform/config/` | — | Delete (duplicate of `anvil-core/src/config/`) |
| `packages/platform/crypto/` | — | Delete (duplicate of `anvil-core/src/crypto/`) |
| `packages/platform/README.md` | — | Delete |
| `packages/json-render/` | `packages/libs/render/` | Rename, update imports |
| `packages/shared/README.md` | Update | Reflect new structure |

### What gets built (new)

| Package | Scope | When |
|---------|-------|------|
| `shared/auth` | Token validation, RBAC type contracts, session types | Before team/enterprise features (DASH, enterprise auth) |
| `shared/telemetry` | Structured logging, OTel spans, metrics | Before production beta (needed for ops visibility) |
| `shared/testing` | Shared test fixtures, mock factories | When e2e harness is refactored (TFIX/TINT) |
| `shared/types` | Branded types, utility types | As needed |

### Naming conventions

The monorepo is mixed-language. Package naming should be legible to both
Rust and TypeScript developers:

- **TS packages**: `@eddacraft/shared-<name>` (e.g., `@eddacraft/shared-auth`)
- **Rust crates**: `eddacraft-<name>` for shared libraries (e.g., `eddacraft-tui`)
- **Product crates**: `anvil-<name>` (e.g., `anvil-kernel`, `anvil-cli`)
- **Libs**: `@eddacraft/render` (no `shared-` prefix — it's a library, not infra)

### Layer map

| Layer | Rust | TypeScript | Purpose |
|-------|------|-----------|---------|
| **Product** | `crates/anvil-*` | `packages/anvil/*` (winding down) | Domain logic |
| **Shared** | `crates/eddacraft-*` | `packages/shared/*` | Cross-cutting infra |
| **Libs** | `crates/anvil-tui-render` | `packages/libs/render` | Shared libraries |
| **Apps** | — | `apps/*` | Deployable services |
| **IaC** | — | `infra/` | Pulumi infrastructure |

### Storage: not TS-only

`shared/storage` provides the `IStorageProvider` abstraction and `FileStorage`
implementation (with path traversal and symlink escape protection). Consumers
include:

- MCP server (reading workspace state)
- E2E tests (fixture management)
- Future: any TS service needing sandboxed file access

The Rust side handles its own file I/O via `std::fs` in the kernel and CLI
crates. If a shared Rust storage abstraction is ever needed, it would be a
separate crate (`eddacraft-storage`), not a binding to the TS package.

### Auth: the real scope

BAUTH (beta-auth-streamline) was a tactical bridge — device flow, OTP, admin
approval. It ships in `apps/anvil-api` and `crates/anvil-cli/src/auth/` with
no shared contracts.

`shared/auth` will define the **contracts** that both sides implement:

- Token types and validation interfaces
- RBAC role/permission enums
- Session lifecycle types
- Team/org scoping types

The Rust CLI and TS API each implement these contracts in their own runtime.
The shared package owns the types, not the implementations.

### Render: dual-target library

`libs/render` owns the dashboard specification schema (JSON) and the web
component renderer (React/Next.js). The Ratatui counterpart
(`crates/anvil-tui-render`) reads the same JSON specs and renders to terminal
widgets. Both share the spec schema; rendering is target-specific.

This is a library, not infrastructure — it has domain semantics (dashboards,
metrics, gates) and distinct rendering targets. It belongs in `libs/`, not
`shared/`.

## Consequences

- `packages/platform/` is retired (3 sub-packages: 1 moved, 2 deleted)
- `packages/shared/` becomes the canonical home for cross-cutting TS packages
- `packages/libs/` is established for shared libraries with domain semantics
- New auth/telemetry work has a clear home before it's needed
- `json-render` rename requires updating imports in DASH work items
- The `packages/anvil/*` deprecation path is unaffected — those packages wind
  down independently as the MCP server and e2e harness migrate to calling the
  Rust binary

## Package README Requirement

Every package under `packages/` and every crate under `crates/` must have a
`README.md` covering:

- **Purpose** — what the package does, in one paragraph
- **Status** — active, winding down, or placeholder
- **API surface** — key exports, entry points
- **Usage examples** — how to import and use
- **Consumers** — what depends on this package (internal and external)

### Shippable packages

Packages intended for public distribution require additional sections:

- **Installation** — npm/cargo install instructions
- **Configuration** — options, defaults, environment variables
- **Compatibility** — supported runtimes, framework versions
- **Changelog** — or link to CHANGELOG.md

Current shippable packages:

| Package | Distribution target |
|---------|-------------------|
| `eslint-plugin-anvil` | npm — ESLint plugin for Anvil conventions (used internally and shipped to users) |
| `crates/anvil-cli` | Binary — the Anvil CLI (`install.sh`, GitHub releases) |
| `packages/mcp-server` | npm — MCP server for Claude Code / AI tool integration |
| `packages/vscode-extension` | VS Code Marketplace |

### Audit (2026-04-02)

10 packages are missing READMEs (excluding 4 being retired). Priority order:

1. `packages/mcp-server` — user-facing, shippable
2. `packages/eslint-plugin-anvil` — user-facing, shippable
3. `apps/website` — public site
4. `packages/anvil/core` — central TS dependency
5. `packages/anvil/runtime` — MCP server dependency
6. `packages/transactional` — email templates
7. `packages/json-render` — write for new `libs/render` location
8. `packages/tooling/*` (2) — shared config
9. `packages/anvil/policy` + `packages/anvil/ports` — active but winding down
10. `crates/anvil-bench` — developer reference

## References

- `packages/platform/README.md` — original platform vision (6-package plan)
- `packages/shared/README.md` — original shared vision (util, testing, brand)
- ADR-014 — language allocation tree (TS vs Rust boundaries)
- APS modules: DASH, DASHCORE, DASHARCH, TUIDASH (dashboard consumers)
- APS module: BAUTH (beta auth, tactical bridge)
