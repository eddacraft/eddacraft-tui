# @eddacraft/render

Dashboard specification schema, validation, and React component renderer for
the Anvil web dashboard. This is the web-side counterpart to the Ratatui TUI
renderer (`crates/anvil-tui-render`); both consume the same JSON spec format.

Introduced by ADR-015 as a domain library under `packages/libs/`.

## Status

Active

## Usage

```tsx
import { DashboardRenderer } from '@eddacraft/render';

export function DashboardPage({ spec }: { spec: unknown }) {
  return <DashboardRenderer spec={spec} className="my-dashboard" />;
}
```

## API Surface

- **`DashboardRenderer`** — React component that validates a JSON spec and
  renders it using the Anvil component catalog. Wraps output in an error
  boundary.
- **`validateSpec(spec)`** — Validates a raw JSON spec against the catalog's
  Zod schema. Returns `{ valid, errors }`.
- **`getComponentNames()`** — Lists all registered component names.
- **`registry`** / **`catalog`** — `@json-render/shadcn` component registry
  and catalog instance.

## Dependencies

- `@json-render/core`, `@json-render/react`, `@json-render/shadcn` for the
  spec schema and rendering engine.
- `zod` for schema validation.
- Peer dependencies: `react`, `react-dom`, `tailwindcss`.

## Consumers

- `apps/website` (dashboard pages)

## Development

```bash
pnpm --filter @eddacraft/render test
pnpm --filter @eddacraft/render build
```
