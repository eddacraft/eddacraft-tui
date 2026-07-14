# ADR-106: Agent integration registry and managed installers

- **Status:** Accepted
- **Date:** 2026-07-14
- **Owners:** Product and CLI
- **Related:** SKPKG, MCPX, ADR-018, ADR-063

## Context

Anvil exposes two independent agent integrations: a customer-readable Agent
Skill and an MCP server. The supported clients, their discovery signals,
configuration locations, and reload instructions have grown beyond the two
hard-coded MCP clients in the CLI. Treating an agent name as proof that every
integration is supported would over-claim coverage and would let the skill and
MCP installers drift apart.

Existing third-party installers also demonstrate unsafe behaviours that Anvil
must not copy: overwriting unmanaged files, refusing harmless repeat installs,
following symlinks outside the selected scope, and maintaining different client
lists in separate commands.

## Decision

Anvil will maintain one typed agent-client registry in the CLI. A registry entry
has a stable client identifier, explicit detection evidence, and independent
capability flags for skill discovery and MCP configuration. Client identity does
not imply either capability.

The registry is shared by `anvil start`, `anvil skill install`, `anvil mcp
install`, and the compatibility `anvil mcp-config` command. Command surfaces
remain thin and delegate path selection, config shape, verification, and reload
guidance to the selected registry adapter.

Skill installation will use a pinned, vendored snapshot embedded in the Anvil
binary with `include_str!`. The installed bundle contains provenance recording
the catalogue commit, Anvil version, and managed file hashes. A repeat install
is idempotent; an update may replace files only when their current hashes match
the previous managed manifest. Unmanaged or user-modified content is refused
with a diagnostic rather than overwritten.

Configuration mutation must preserve unrelated settings, use atomic file
replacement, and refuse symlink traversal outside the selected root. Where a
client owns global configuration through profiles rather than a stable file,
Anvil delegates to the vendor CLI instead of guessing the path.

Interactive installation detects installed clients and preselects them. The
user chooses global or project scope, with global as the default. Scripted use
provides explicit `--client` and `--scope` arguments plus read-only preview or
verification modes.

The MCP first wave retains Claude Code and Cursor and adds Codex, OpenCode,
Gemini CLI, Antigravity, OpenClaw, VS Code/Copilot, Copilot CLI, Grok Build,
Warp, and project-scoped Zed. Support is constrained to documented stdio
configuration and stable locations or vendor CLI operations. Devin is deferred
because its supported integration is marketplace/UI-managed rather than a
documented local file or command surface.

Status reporting distinguishes client detection, config presence, MCP
handshake, and live tool validation. No earlier state may be presented as proof
of a later state.

## Consequences

- Client additions require one registry entry and capability-specific adapter,
  reducing drift between command surfaces.
- The beta binary grows by the size of the readable skill bundle and can ship
  independently of the private catalogue.
- Safe refusal creates explicit remediation work for unmanaged drift instead of
  silently destroying user configuration.
- Some clients have project-only or vendor-CLI-only support; the UI and JSON
  output must disclose those constraints.
- This expands Anvil's enforcement reach without turning Anvil into a general
  agent orchestrator.

## Alternatives considered

- **Fetch the private catalogue during installation.** Rejected because beta
  customers do not have an authenticated, stable distribution channel and
  installation must work offline.
- **Keep a separate client list in every command.** Rejected because the lists
  and paths had already diverged.
- **Treat common directory names as client detection.** Rejected because
  generic `.agents`, `.gemini`, and `.mcp.json` artefacts are weak evidence and
  create false-positive installs.
- **Overwrite existing entries on every run.** Rejected because it destroys
  unmanaged user changes and makes rollback provenance impossible.
