# @eddacraft/shared-storage

File system storage provider implementing the `IStorageProvider` interface from
`@eddacraft/anvil-ports`. Provides path traversal protection and symlink escape
detection out of the box.

Extracted from `@eddacraft/anvil-runtime` as part of ADR-015 to allow shared
infrastructure packages to use storage without pulling in the full runtime.

## Status

Active

## Usage

```ts
import { createFileStorage } from '@eddacraft/shared-storage';

const storage = createFileStorage('/path/to/base');

await storage.write('config.json', JSON.stringify({ key: 'value' }));
const content = await storage.read('config.json');
const exists = await storage.exists('config.json');
const files = await storage.list('.');
```

## API Surface

- **`FileStorage`** — Class implementing `IStorageProvider` with `read`,
  `readBuffer`, `write`, `exists`, `delete`, `list`, and `mkdir` methods.
- **`createFileStorage(baseDir?)`** — Factory function returning a
  `FileStorage` instance.

All path operations are sandboxed to the configured `baseDir`. Attempts to
traverse outside it (via `../` or symlinks) throw an error.

## Consumers

- `@eddacraft/anvil-runtime` (gate runner, export, watch)
- `@eddacraft/anvil-mcp-server` (resource providers)

## Development

```bash
pnpm --filter @eddacraft/shared-storage test
pnpm --filter @eddacraft/shared-storage build
```
