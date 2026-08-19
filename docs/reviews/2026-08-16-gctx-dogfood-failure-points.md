# GCTX dogfood failure points (2026-08-16)

| Type  | Authority | Owner | Status | Freshness                                                                                                                                                                                                                                                                                          |
| ----- | --------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Advisory  | CIB   | Live   | Last reviewed 2026-08-19: CIB-344 produce-lock reap in `graph_base_trigger.rs` (`activate` sweeps dead-pid `.producing/*.lock`); process-orphan half already on main via #3963. Prior 2026-08-17 CIB-343 handshake. Original measurement 2026-08-16 against anvil 0.9.4-beta intercept pid 1387799 |

| Upstream                                                                                                                                                                                                                                                                                     | Downstream   |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| `docs/guides/ai-context-delivery.md`, `docs/architecture/mcp-shim-as-built.md`, `crates/anvil-intercept/src/full_scan_executor.rs`, `crates/anvil-intercept/src/graph_base_trigger.rs`, `crates/anvil-cli/src/activation/mcp_client.rs`, `crates/anvil-cli/src/activation/agent_registry.rs` | CIB-341..344 |

Session diagnosis of why graph-context looked broken in Grok Build on the
dogfood repo. This note is evidence, not a second backlog. Executable work is
CIB-341..344.

## What still worked

- The anvil MCP shim was attached (`anvil mcp serve --stdio` 0.9.4-beta).
- Graph tools reached the intercept daemon over the Unix socket. First
  `anvil_search_symbols` returned a structured `not_ready`; a later retry
  returned `ready` on a stale graph.
- Snippet egress was the identity-only default. That is not the failure.
- `ANVIL_GCTX_EGRESS` was unset, so the surface was not operator-disabled.

## Failure points

### 1. Full scan never finishes on this repo (CIB-341)

The intercept full scan has a hard 60-second wall-clock budget (`SCAN_TIMEOUT`
in `crates/anvil-intercept/src/full_scan_executor.rs`). anvil-001 is thousands
of Rust and TypeScript files. The daemon log repeats:

`full scan exceeded its wall-clock budget; marked stale (scan-timeout)` for
`/home/aneki/Projects/src/anvil-001`.

Observed GCTX envelope:

- while scanning: `workspace_assurance.state = running`, `generation = 0`,
  outcome `not_ready` ("the workspace graph is warming")
- after timeout: `state = stale`, `reason = scan-timeout`, still
  `generation = 0`, outcome `ready` on a partial graph

A `not_ready` miss correctly fires on-demand re-warm. That restart-and-timeout
loop is why the surface keeps looking down. Stale hits also looked wrong:
rustc-style paths such as `crate::mcp::tools::validate_write` were returned as
`Module` identities.

`anvil start --verify --json` agreed: `save_time.state = stale`,
`reason = scan-timeout`.

### 2. Graph-base production fails spawn and serves cold (CIB-342)

The cheaper path is compose-on-merge-base. The daemon is supposed to re-exec
itself as `anvil graph-base build --repo <git-dir>` (`CurrentExeSpawner` in
`crates/anvil-intercept/src/graph_base_trigger.rs`). The log repeats:

`failed to spawn base-production subprocess; serving cold` with
`No such file or directory` for `/home/aneki/Projects/src/anvil-001/.git`.

A matching `*.base` artefact for HEAD already existed under
`~/.local/state/anvil/graph-cache/base/`. Stale single-flight locks from 31 July
and 1 August were still sitting in `graph-cache/base/.producing/`. Cold full
scans then hit the 60-second cap in failure point 1.

### 3. Live handshake still treats only Claude Code and Cursor as first-wave (CIB-343)

`AgentClientId` already lists twelve installable clients (Claude Code, Cursor,
Codex, OpenCode, Gemini CLI, Antigravity, OpenClaw, VS Code, Copilot CLI, Grok,
Warp, Zed). MCPX first-wave install and `anvil mcp install` write configs for
them.

The activation diagnostic ladder that can promote to
`restart_handshake_verified` / live-validation still iterates only two
`McpClient` impls:

`all_clients() -> &[&cursor::Cursor, &claude_code::ClaudeCode]` in
`crates/anvil-cli/src/activation/mcp_client.rs`.

Measured on this Grok session:

- tools worked over stdio
- `anvil start --verify` stayed `ready_restart_required` with Cursor and Claude
  Code as the handshake-verified clients
- `anvil status` said `mcp: wired (restart pending)` and `daemon: not attesting`
- intercept sessions were only `anvil-start` / `activation-spine`, not a Grok
  MCP handshake session

CIB-227 already shipped the copy fix ("do not imply only Claude Code and
Cursor"). CIB-343 implements the runtime ladder: every first-wave
`AgentClientId` participates in `probe_all` / `anvil start --verify`. This note
keeps the 2026-08-16 measurement as evidence of the pre-fix state.

### 4. Stale MCP shims and produce-locks are never reaped (CIB-344)

The 2026-08-16 host had dozens of leftover `anvil mcp serve --stdio` processes,
including 0.9.2-beta images from 5–14 August, plus month-old `.producing/*.lock`
files. That was the pre-fix measurement.

As of 2026-08-19 the produce-lock half is in `graph_base_trigger.rs`: `activate`
sweeps dead-pid / PID-reuse locks under `graph-cache/base/.producing/`.
`anvil doctor` / `anvil doctor --fix` and `anvil intercept start` / human
`status` report and heal the same class. The process-orphan shim half shipped
earlier on main via #3963. MCPLH re-exec still heals only the current stdio
child.

This was not what made GCTX return `not_ready`; it was standing operator debris.

## Not filed as separate items

- `ANVIL_NO_SAVE_TIME_DRIVER=1` on this host disables save-time warming.
  Operator env, not a product defect.
- MCP `anvil_status` still reports `daemonStatus: not-wired` even when GCTX is
  talking to the daemon. Honesty leftover; do not confuse it with GCTX
  readiness.

## Reproduction (read-only)

```bash
anvil --version
anvil status --json
anvil intercept status --json
anvil start --verify --json
rg 'scan-timeout|serving cold|failed to spawn base-production' ~/.local/state/anvil/intercept.daemon.log
ps -ef | rg 'anvil mcp serve'
```

From an MCP client, call `anvil_search_symbols` twice about a minute apart and
compare `workspace_assurance` plus `outcome.status`.
