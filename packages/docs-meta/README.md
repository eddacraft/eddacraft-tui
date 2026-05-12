# @eddacraft/anvil-docs-meta

Parses the DOCGOV-002 documentation governance metadata convention (a
five-column metadata table plus an Upstream/Downstream relationships table
declared immediately after the H1) from Markdown documents and returns a
validated, typed result. It is consumed by the DOCGOV-005 `pnpm docs:check`
metadata validator and reused by the upcoming DOCGOV-006 as-built freshness
check and DOCGOV-007 generated documentation indexes; see
[`docs/guides/documentation-governance.md`](../../docs/guides/documentation-governance.md)
for the canonical convention this package implements.

## Usage

```typescript
import { parseDocGovernance } from '@eddacraft/anvil-docs-meta/parser';

const result = parseDocGovernance(content, 'docs/guides/example.md');
result.metadata.type; // 'Guide'
result.metadata.status; // 'Live'
result.relations.upstream; // ['plans/modules/documentation-governance.aps.md', ...]
```

## Development

```bash
pnpm -F @eddacraft/anvil-docs-meta build
pnpm -F @eddacraft/anvil-docs-meta test
```

See [`AGENTS.md`](./AGENTS.md) for the package layout and contribution rules.
