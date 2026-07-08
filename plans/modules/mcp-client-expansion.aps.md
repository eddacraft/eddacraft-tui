# MCP Client Expansion

| ID   | Owner | Status   | Progress |
| ---- | ----- | -------- | -------- |
| MCPX | —     | Proposed | 0/6      |

**Last reviewed:** 2026-07-08 — candidate matrix added from operator-provided
target research. Module remains Proposed until MCPX-001 verifies the cited docs,
normalises target names, and captures fixture evidence. Claude Code and Cursor
remain the only currently supported install clients.

## Purpose

Expand Anvil's supported MCP client set beyond Claude Code and Cursor without
turning every MCP-capable tool into an unverified support claim. This module owns
client evidence, config generation, installer wiring, activation detection, docs,
and compatibility smoke coverage for additional clients.

## In Scope

- Supported-client evidence for MCP-capable tools beyond Claude Code and Cursor,
  plus baseline rows for existing Claude Code and Cursor support
- Candidate expansion clients including Grok Build, Devin, Google Antigravity,
  OpenCode, Copilot/VS Code, Codex, Warp, Zed, Copilot CLI, Visual Studio, and
  generic project MCP
- `anvil mcp-config` target additions and generated config validation
- `anvil mcp install --client <target>` additions and idempotent write behaviour
- Activation/start detection and install-picker behaviour for newly supported clients
- Client-specific compatibility smoke checks for `anvil mcp serve --stdio`
- Public docs and troubleshooting for each promoted client

## Out of Scope

- Rust MCP server tool/resource parity — owned by RMCPF
- New graph-context tools/resources — owned by GCTX
- Skill packaging or `anvil skill install` targets — owned by SKPKG
- Non-MCP editor-driver integrations — owned by DRVR/RTAI if revived
- Supporting a client that lacks a stable local MCP configuration contract
- Streamable HTTP transport unless RMCPF-021 keeps it for a specific supported client

## Interfaces

**Depends on:**

- [rust-mcp-full-port](rust-mcp-full-port.aps.md) — canonical Rust MCP server and
  supported-client matrix decisions
- RCLI3-016 / RCLI3-016b — existing `mcp-config` and `mcp install` command
  surfaces
- [activation-mcp-optional](activation-mcp-optional.aps.md) — editor-aware MCP
  install/probe UX during `anvil start`
- `crates/anvil-cli/src/commands/mcp.rs` and
  `crates/anvil-cli/src/commands/mcp_config.rs` — current client enum, path
  resolution, and config writer

**Exposes:**

- A promoted MCP client support matrix with evidence per client
- Additional `anvil mcp-config --target <client>` targets where accepted
- Additional `anvil mcp install --client <client>` targets where accepted
- Docs/runbook entries for supported clients beyond Claude Code and Cursor

## Ready Checklist

Change status to **Ready** when:

- [ ] First-wave target clients are selected from verified evidence, not assumption
- [ ] Each first-wave client has a stable config path/format and restart model
- [ ] RMCPF confirms the required transport for each target is available or kept
- [ ] Work items MCPX-001..006 have enough implementation detail to execute

## Candidate Matrix

This matrix is planning input, not yet an implementation contract. MCPX-001 owns
verifying the cited docs, capturing sample configs, and deciding exact target
names for `McpClient` / `mcp-config`.

### Tier 1 — First-class support candidates

| Target | Type | MCP config location | Detection markers | Anvil support note |
| ------ | ---- | ------------------- | ----------------- | ------------------ |
| **Grok Build** | Agent CLI / coding harness | User `~/.grok/config.toml`; project `.grok/config.toml`; credentials `~/.grok/mcp_credentials.json` | `.grok/config.toml`, `.grok/`, `AGENTS.md`, `.mcp.json` | Treat as Tier 1 if docs/fixtures verify. Grok imports MCP config from Claude, Cursor, and `.mcp.json`, so it is valuable for detection and migration. |
| **Devin CLI / Desktop / Cloud** | Agent CLI / desktop / cloud agent | Local `.devin/config.local.json`; project `.devin/config.json`; user `~/.config/devin/config.json` or `%APPDATA%\devin\config.json`; cloud Settings -> Connections -> MCP servers | `.devin/`, `.devin/config.json`, `.devin/config.local.json`, `AGENTS.md`, `AGENTS.local.md`, `AGENT.md`, `.windsurfrules`, `.windsurf/rules/`, `.cursor/rules/`, `.claude/`, `CLAUDE.md` | Replaces the earlier Windsurf row if verified. Treat Windsurf markers as Devin compatibility signals, not a standalone first-wave client by default. |
| **Google Antigravity** | Agent-first IDE / CLI | Shared central config `~/.gemini/config/mcp_config.json` | User marker `~/.gemini/config/mcp_config.json`; repo marker weak, so only use project conventions such as `AGENTS.md` as weak signals | Treat as the Google default, not Gemini CLI. The `.gemini` path is a config detail and should not overclaim Gemini CLI support. |
| **OpenCode** | Open coding harness | Global `~/.config/opencode/opencode.json`; project `opencode.json`; MCP key `mcp` | `opencode.json`, `.opencode/`, `.opencode/agents/`, `.opencode/commands/`, `.opencode/skills/`, `.opencode/plugins/`, `.opencode/tools/` | Tier 1. Strong project-level markers make it a clean Anvil support target. |
| **GitHub Copilot in VS Code** | IDE agent / enterprise default | Workspace `.vscode/mcp.json`; user profile MCP config via VS Code command palette; dev container `customizations.vscode.mcp` | `.vscode/mcp.json`, `.vscode/`, `.devcontainer/devcontainer.json` | Enterprise-priority candidate if docs/fixtures verify. Treat separately from Copilot CLI. |
| **Claude Code** | Agent CLI / coding harness | User/local `~/.claude.json`; project `.mcp.json`; settings `.claude/settings.json`, `.claude/settings.local.json` | `.mcp.json`, `.claude/`, `CLAUDE.md`, `.claude/settings.json`, `.claude/settings.local.json` | Already supported. Keep as Tier 1 and use as the shared-project MCP baseline. |
| **Cursor** | AI IDE | Project `.cursor/mcp.json` | `.cursor/`, `.cursor/mcp.json`, `.cursor/rules/*.md`, `.cursor/rules/*.mdc` | Already supported. Keep as Tier 1 because support exists and the install base is large, even if strategic expansion now focuses on other harnesses. |
| **OpenAI Codex** | Agent CLI / coding harness | User `~/.codex/config.toml`; project `.codex/config.toml`; MCP table `[mcp_servers.<id>]` | `.codex/`, `.codex/config.toml` | First-wave candidate if docs/fixtures verify; aligns with Anvil's `AGENTS.md` strategy. |
| **Warp** | Agentic terminal / workflow host | UI Settings -> Agents -> MCP servers; project `.warp/.mcp.json`; cloud agents via `--mcp`, agent config, or shared MCP UUIDs | `.warp/`, `.warp/.mcp.json` | Support as a terminal harness, not just a terminal. Useful for command-driven Anvil operations. |
| **Zed** | Editor / agent host | Zed settings / Agent Panel configuration; repo-scoped MCP file appears weaker than VS Code, Warp, or OpenCode | `.zed/` if present, but weak; prefer installed Zed plus agent-integration signals | Strategic editor-host candidate if MCPX-001 verifies a stable local config contract; may also be supported through adjacent harnesses such as Devin/OpenCode/Codex/Claude running inside or beside Zed. |
| **GitHub Copilot CLI** | Agent CLI / coding harness | User `${COPILOT_HOME:-~/.copilot}/mcp-config.json`; workspace `.mcp.json`, `.github/mcp.json`; session `--additional-mcp-config` | `.mcp.json`, `.github/mcp.json`; `.copilot/` is usually user-level; plugin markers `agents/`, `skills/`, `hooks.json`, `.github/plugin/marketplace.json` | Treat separately from VS Code Copilot. It has built-in MCPs, workspace loading, plugin packaging, and its own precedence model. |

Tier-1 source links to verify in MCPX-001: [Grok Build][grok-mcp],
[Devin][devin-mcp], [Google Antigravity][antigravity-mcp],
[OpenCode][opencode-config], [VS Code][vscode-mcp], [Claude Code][claude-mcp],
[Codex][codex-config], [Warp][warp-mcp], [Zed][zed-mcp], and
[GitHub Copilot CLI][copilot-cli-mcp].

### Tier 2 — Default detection, narrower lifecycle support

| Target | Type | MCP config location | Detection markers | Anvil support note |
| ------ | ---- | ------------------- | ----------------- | ------------------ |
| **Visual Studio + Copilot** | Enterprise IDE | User `%USERPROFILE%\.mcp.json`; solution `.vs/mcp.json`, `.mcp.json`, `.vscode/mcp.json`, `.cursor/mcp.json` | `.sln`, `.vs/`, `.mcp.json`, `.vscode/mcp.json`, `.cursor/mcp.json` | Important for Microsoft-heavy enterprise customers. Support detection and validation after VS Code/Copilot unless evidence changes priority. |
| **Generic project MCP** | Shared MCP convention | `.mcp.json`, `.vscode/mcp.json`, `.cursor/mcp.json`, `.warp/.mcp.json` | `.mcp.json`, `mcp.json`, `mcpServers`, `servers` | Escape hatch: manually add any MCP server, then optionally project it into specific harness configs. |

Tier-2 source links to verify in MCPX-001: [Visual Studio][visual-studio-mcp]
and [Claude project MCP convention][anthropic-mcp].

## Work Items

### MCPX-001: Client evidence matrix

- **Status:** Proposed
- **Intent:** Decide which MCP-capable clients are supportable in the first expansion wave.
- **Expected Outcome:** The candidate matrix above is verified against current
  vendor docs and local fixtures, target names are normalised, and each candidate
  has a support verdict with config path, config schema, supported transport,
  restart/reload behaviour, and detection signal.
- **Validation:** Evidence matrix reviewed against current client documentation and
  at least one local config fixture per promoted client.
- **Dependencies:** RMCPF-003, RMCPF-021 if any candidate requires non-stdio transport
- **Confidence:** medium

### MCPX-002: Extend `anvil mcp-config` targets

- **Status:** Proposed
- **Intent:** Generate correct MCP server configuration for each accepted first-wave client.
- **Expected Outcome:** `anvil mcp-config --target <client>` supports the promoted
  client targets, emits valid config, and `--verify` reports missing/malformed
  entries without writing.
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_config`
- **Dependencies:** MCPX-001
- **Confidence:** medium

### MCPX-003: Extend `anvil mcp install --client`

- **Status:** Proposed
- **Intent:** Make installation for accepted clients a one-command, idempotent operation.
- **Expected Outcome:** `anvil mcp install --client <client>` resolves the client
  config location, writes the Anvil server entry safely, preserves unrelated user
  config, rewrites Anvil-owned drift, and prints accurate restart/reload guidance.
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_config mcp_install`
- **Dependencies:** MCPX-002
- **Confidence:** medium

### MCPX-004: Activation detection and install picker

- **Status:** Proposed
- **Intent:** Teach `anvil start` to offer newly supported clients only when there is strong evidence they are installed or explicitly requested.
- **Expected Outcome:** The activation install picker includes promoted clients
  with real detection signals, skips undetected clients by default, honours an
  explicit all-clients opt-in, and avoids phantom "AI tools detected" claims.
- **Validation:** `cargo test -p eddacraft-anvil activation::mcp_client`
- **Dependencies:** MCPX-003, ACTMO-012
- **Confidence:** medium

### MCPX-005: Client compatibility smoke coverage

- **Status:** Proposed
- **Intent:** Prove promoted clients can launch and communicate with `anvil mcp serve --stdio`.
- **Expected Outcome:** Fixture-backed or client-harness smoke tests cover config
  generation, process launch shape, `initialize`, `tools/list`, and at least one
  safe `tools/call` per promoted client.
- **Validation:** Targeted MCP client smoke tests documented in the item closeout;
  exact command depends on MCPX-001 client selection.
- **Dependencies:** MCPX-003, RMCPF-030
- **Confidence:** low

### MCPX-006: Docs and troubleshooting

- **Status:** Proposed
- **Intent:** Document newly supported MCP clients without over-claiming clients that remain deferred.
- **Expected Outcome:** Public docs list supported clients, install commands,
  verification commands, restart/reload guidance, known limitations, and deferred
  candidates with reasons.
- **Validation:** `pnpm docs:check`
- **Dependencies:** MCPX-003, MCPX-005
- **Confidence:** high

## Decisions

1. **Evidence before support claims** — a client is supported only after MCPX-001
   records its config contract and MCPX-005 proves the launch path, not because it
   is generally MCP-capable.
2. **Stdio-first** — new clients use `anvil mcp serve --stdio` unless RMCPF-021
   explicitly keeps another transport for that client.
3. **No phantom installs** — activation follows ACTMO-012: fresh config writes
   require strong editor/client detection or an explicit operator opt-in.

## Notes

- Current supported install clients remain `cursor` and `claude-code`.
- RMCPF already deferred Continue, VS Code, and Windsurf pending fresh evidence;
  this module supersedes that loose parking lot with a broader target matrix.
- Codex, Grok Build, Devin, Antigravity, OpenCode, Warp, Zed, Copilot CLI, and
  Visual Studio support all depend on verified local config contracts and smoke
  evidence.

[anthropic-mcp]: https://docs.anthropic.com/en/docs/claude-code/mcp
[antigravity-mcp]: https://codelabs.developers.google.com/developer-knowledge-mcp-antigravity
[claude-mcp]: https://code.claude.com/docs/en/mcp-quickstart
[codex-config]: https://developers.openai.com/codex/config-basic
[copilot-cli-mcp]: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers
[cursor-mcp]: https://cursor.com/docs/mcp
[devin-mcp]: https://cli.devin.ai/docs/extensibility/mcp/overview
[grok-mcp]: https://docs.x.ai/build/features/mcp-servers
[opencode-config]: https://opencode.ai/docs/config/
[visual-studio-mcp]: https://learn.microsoft.com/en-us/visualstudio/ide/mcp-servers?view=visualstudio
[vscode-mcp]: https://code.visualstudio.com/docs/agent-customization/mcp-servers
[warp-mcp]: https://docs.warp.dev/agent-platform/capabilities/mcp/
[zed-mcp]: https://zed.dev/docs/assistant/model-context-protocol
