# CLI Output Stream Policy

All Anvil CLI commands follow a strict stdout/stderr split so that piped output
is always machine-parseable. This policy applies to both the Rust binary
(`crates/anvil-cli/`) and the legacy TypeScript CLI (`apps/anvil-cli/`).

## Rules

| Content type                                       | Stream     | Rationale                                   |
| -------------------------------------------------- | ---------- | ------------------------------------------- |
| Structured data (JSON, machine-parseable)          | **stdout** | Clean piping to `jq`, scripts, CI artefacts |
| Human results (summaries, tables, formatted text)  | **stderr** | Visible in terminal, invisible to pipes     |
| Progress indicators (spinners, status lines)       | **stderr** | Already correct via `ora`                   |
| Status messages (`success()`, `info()`, `error()`) | **stderr** | Diagnostics, not data                       |

## TypeScript Helpers (`output.ts`)

> **Note:** This section covers the legacy TypeScript CLI helpers at
> `apps/anvil-cli/src/utils/output.ts`. The Rust CLI implements the same
> stdout/stderr split natively via clap and direct `eprintln!`/`println!` calls.

Import helpers from `../utils/output.js`:

```ts
import { success, error, warning, info, data } from '../utils/output.js';
```

| Function        | Stream | Use for                    |
| --------------- | ------ | -------------------------- |
| `success(msg)`  | stderr | Completion confirmations   |
| `error(msg)`    | stderr | Error diagnostics          |
| `warning(msg)`  | stderr | Non-fatal issues           |
| `info(msg)`     | stderr | Informational status       |
| `data(content)` | stdout | Explicit structured output |

For JSON output, either use `data(JSON.stringify(obj, null, 2))` or
`console.log(JSON.stringify(...))` directly. Both write to stdout.

## `--json` mode conventions

Commands with a `--json` flag must:

1. Avoid emitting any human-readable output (spinners, summaries, status
   messages) to **stdout** when `--json` is active. Human-readable output on
   **stderr** is fine and typically needs no changes, since the helpers above
   already write to stderr.
2. Write exactly one JSON document to stdout.
3. Never mix human text into the stdout stream.

This ensures `anvil check --json | jq .` always produces valid JSON.

## Adding a new command

### TypeScript (legacy CLI)

1. Use `success()`, `info()`, `error()`, `warning()` for all human output.
2. Use `data()` or `console.log(JSON.stringify(...))` for structured output.
3. Never use bare `console.log()` for diagnostics or progress text.
4. Use `process.stderr.write()` for custom progress indicators.

### Rust CLI

1. Use `eprintln!()` for all human output (diagnostics, status, progress).
2. Use `println!()` for structured data (JSON output).
3. With `--json`, suppress all `eprintln!` output and emit exactly one JSON
   document via `println!`.
4. Use `clap`'s built-in output for help and version text.
