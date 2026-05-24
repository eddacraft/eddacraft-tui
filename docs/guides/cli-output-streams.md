# CLI Output Stream Policy

| Type  | Authority     | Owner | Status | Freshness                                                                                                         |
| ----- | ------------- | ----- | ------ | ----------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | CLAR  | Live   | Last reviewed 2026-05-25 against `crates/anvil-cli/src/output/` and `crates/anvil-kernel-types/src/diagnostic.rs` |

| Upstream                                                                                            | Downstream                                                   |
| --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `crates/anvil-cli/src/output/`, `crates/anvil-kernel-types/src/diagnostic.rs`, `docs/public/anvil/` | CLI commands, MCP tools, public release docs, `CHANGELOG.md` |

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

### Current payload notes

- `doctor --json` emits an object root:
  `{ "checks": [...], "notifications": [...], "schema_version": "2.0.0" }`.
  Every check carries a structured `remediation` object
  (`{ summary, command?, doc_url? }`).
- `check --json`, `gate --json`, and `audit --json` all include a
  `notifications[]` field alongside their existing payloads, sharing the same
  envelope shape as `doctor`.
- The notification envelope is the shared `Notification` shape owned by
  `crates/anvil-kernel-types`; subscribers can filter by class, priority, and
  source without parsing per-command payloads.
- `gate --profile ai` and the MCP `validate_write` tool emit diagnostics in the
  canonical `anvil.diagnostic.v1` envelope (also owned by `anvil-kernel-types`).
  Save-time, watch, mid-edit, and gate surfaces all converge on this envelope so
  agent and editor consumers parse one shape, not four.
- When evolving any of the JSON payloads above, bump `schema_version` where
  present, document the change in `CHANGELOG.md`, and update the public release
  docs before shipping.

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

## Error display convention (path-leakage guardrail)

`anyhow::Error` chains can embed absolute paths — most commonly via
`notify::Error` and `std::io::Error` — whose Display includes the watched
directory, the user's `$HOME`, or project layout. When such output is captured
(CI logs, terminal recordings, screenshots, pastebins) it reveals usernames and
internal directory structure.

**Rule for non-top-level sites** (TUI flows, best-effort fallbacks, logging
during `run()` execution):

| Verbosity          | Format                        | Notes                                        |
| ------------------ | ----------------------------- | -------------------------------------------- |
| Default            | `{err}` — outer context only  | Programmer-written string; no wrapped paths. |
| `--verbose` / `-v` | `{err:#}` — full anyhow chain | Includes root cause and any embedded paths.  |

Prefer the helper `crate::util::format_user_error(&err, verbose)` rather than
raw `format!` — it keeps the convention in one place and has unit coverage
against path leakage.

**Blind spot — path-embedding context strings.** At `verbose = false` the helper
prints only the outermost context (`{err}`) and so avoids printing paths that
live in the _wrapped_ error chain (e.g. `notify::Error`, `std::io::Error`).
Context strings added by the programmer that _themselves_ embed a path become
part of that outermost message and will leak even at `verbose = false`:

```rust
// BAD — path is in the outer context, {err} prints it
std::fs::read_to_string(&path)
    .with_context(|| format!("reading {}", path.display()))
```

If an error chain is going to be routed through `format_user_error`, keep the
outer context path-free and let the inner `io::Error` carry the path only for
verbose mode:

```rust
// GOOD — outer context is path-free; the wrapped io::Error carries the path
std::fs::read_to_string(&path).context("reading workspace file")
```

```rust
// GOOD — TUI/interactive context, non-verbose default
eprintln!(
    "Watch demo unavailable: {}",
    crate::util::format_user_error(&err, false)
);

// BAD — unconditionally prints the full chain, leaks cwd when watcher errors
eprintln!("Watch demo unavailable: {err:#}");
```

**Top-level CLI error handlers** (`main.rs` command dispatch) are exempt: the
user ran the command, a full chain is useful, and the shell session is already
private to them. Any new top-level handler that tees to shared log storage (e.g.
tracing subscribers shipping to a SaaS sink) must opt back into this convention.

### Audit scope

When adding a new `eprintln!`/`tracing::warn!`/`tracing::error!` site that
prints an `anyhow::Error`, ask:

1. Can the error chain contain a filesystem path? (`notify::`, `std::io::`,
   `std::fs::`, `reqwest::Error` with a local URL, `tempfile::` errors.)
2. Is this site reachable during CI or automated capture? (TUI demos, progress
   watchers, best-effort fallbacks that don't abort.)

If both answers are "yes," route through `format_user_error(&err, verbose)` with
a `verbose` source that reflects the user's `--verbose` choice.

See issue #1017 and council review `council-8a7372c7` (finding C-009) for the
original motivating case in the tutorial watcher.
