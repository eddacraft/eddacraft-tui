# MCP Client Expansion

| ID   | Owner | Status   | Progress |
| ---- | ----- | -------- | -------- |
| MCPX | —     | Done | 6/6      |

**Last reviewed:** 2026-07-14 — MCPX-001 verified current vendor contracts and
the first implementation wave is now present across config generation,
managed installation, activation, smoke coverage, and public documentation.
All 6 first-wave items are Done; Tier 2 clients (Visual Studio + Copilot,
generic project MCP) remain unscheduled follow-on work.

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
- MCP `2026-07-28` dual-era protocol support — owned by
  [MCP26](mcp-dual-era-support.aps.md); MCPX clients remain the
  compatibility matrix MCP26 must not regress
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

- [x] First-wave target clients are selected from verified evidence, not assumption
- [x] Each first-wave client has a stable config path/format and restart model,
  or an explicit scope constraint
- [x] RMCPF confirms the required stdio transport is available
- [x] Work items MCPX-001..006 have enough implementation detail to execute

## Candidate Matrix

This matrix is planning input, not yet an implementation contract. MCPX-001 owns
verifying the cited docs, capturing sample configs, and deciding exact target
names for `McpClient` / `mcp-config`.

### Selected first wave (MCPX-001, 2026-07-14)

Retain `claude-code` and `cursor`, then add `codex`, `opencode`,
`gemini-cli`, `antigravity`, `openclaw`, `vscode`, `copilot-cli`, `grok`,
`warp`, and project-scoped `zed`. Every promoted target has a documented local
stdio contract compatible with `anvil mcp serve --stdio`.

Constraints:

- VS Code global installation delegates to `code --add-mcp` so the active user
  profile owns its path; workspace installation writes `.vscode/mcp.json`.
- Zed automation is project-only (`.zed/settings.json`) until its conflicting
  global path documentation is reconciled.
- Gemini CLI and Antigravity are distinct clients despite sharing `.gemini`
  state; detection must use their exact binary/config markers.
- OpenClaw native subcommands are not invoked because released versions expose
  different MCP command sets; direct config is the stable first-wave path.
- Devin is deferred: official documentation exposes Marketplace/UI setup but no
  supported local file or CLI mutation contract.
- Strong detection means the client executable or exact config marker. Shared
  `AGENTS.md`, `.agents/`, `.gemini/`, and `.mcp.json` markers never justify a
  phantom client claim on their own.

### Tier 1 — First-class support candidates

| Target | Type | MCP config location | Detection markers | Anvil support note |
| ------ | ---- | ------------------- | ----------------- | ------------------ |
| **Grok Build** | Agent CLI / coding harness | User `~/.grok/config.toml`; project `.grok/config.toml`; credentials `~/.grok/mcp_credentials.json` | `.grok/config.toml`, `.grok/`, `AGENTS.md`, `.mcp.json` | Treat as Tier 1 if docs/fixtures verify. Grok imports MCP config from Claude, Cursor, and `.mcp.json`, so it is valuable for detection and migration. |
| **Devin CLI / Desktop / Cloud** | Agent CLI / desktop / cloud agent | Marketplace/UI only; no verified local mutation contract | Exact Devin installation only | Deferred from automation. Document manual Marketplace setup until Cognition publishes a supported local file or CLI API. |
| **Gemini CLI** | Agent CLI / coding harness | User `~/.gemini/settings.json`; project `.gemini/settings.json` | `gemini` binary or exact settings file | First wave. Keep separate from Antigravity; both support stdio but use different config paths and reload flows. |
| **Google Antigravity** | Agent-first IDE / CLI | User `~/.gemini/config/mcp_config.json`; workspace `.agents/mcp_config.json` | Antigravity binary/app or exact config file | First wave. A generic `.gemini/` or `.agents/` directory is not sufficient detection evidence. |
| **OpenClaw** | Agent CLI / gateway | Resolve with `openclaw config file`; default `~/.openclaw/openclaw.json` | `openclaw` binary or exact active config | First wave through the stable direct-config shape; version-varying native mutation commands are not invoked. |
| **OpenCode** | Open coding harness | Global `~/.config/opencode/opencode.json`; project `opencode.json`; MCP key `mcp` | `opencode.json`, `.opencode/`, `.opencode/agents/`, `.opencode/commands/`, `.opencode/skills/`, `.opencode/plugins/`, `.opencode/tools/` | Tier 1. Strong project-level markers make it a clean Anvil support target. |
| **GitHub Copilot in VS Code** | IDE agent / enterprise default | Workspace `.vscode/mcp.json`; user profile MCP config via VS Code command palette; dev container `customizations.vscode.mcp` | `.vscode/mcp.json`, `.vscode/`, `.devcontainer/devcontainer.json` | Enterprise-priority candidate if docs/fixtures verify. Treat separately from Copilot CLI. |
| **Claude Code** | Agent CLI / coding harness | User/local `~/.claude.json`; project `.mcp.json`; settings `.claude/settings.json`, `.claude/settings.local.json` | `.mcp.json`, `.claude/`, `CLAUDE.md`, `.claude/settings.json`, `.claude/settings.local.json` | Already supported. Keep as Tier 1 and use as the shared-project MCP baseline. |
| **Cursor** | AI IDE | Project `.cursor/mcp.json` | `.cursor/`, `.cursor/mcp.json`, `.cursor/rules/*.md`, `.cursor/rules/*.mdc` | Already supported. Keep as Tier 1 because support exists and the install base is large, even if strategic expansion now focuses on other harnesses. |
| **OpenAI Codex** | Agent CLI / coding harness | User `~/.codex/config.toml`; project `.codex/config.toml`; MCP table `[mcp_servers.<id>]` | `.codex/`, `.codex/config.toml` | First-wave candidate if docs/fixtures verify; aligns with Anvil's `AGENTS.md` strategy. |
| **Warp** | Agentic terminal / workflow host | UI Settings -> Agents -> MCP servers; project `.warp/.mcp.json`; cloud agents via `--mcp`, agent config, or shared MCP UUIDs | `.warp/`, `.warp/.mcp.json` | Support as a terminal harness, not just a terminal. Useful for command-driven Anvil operations. |
| **Zed** | Editor / agent host | Project `.zed/settings.json`; global path deferred pending vendor-doc reconciliation | Exact project settings or verified Zed executable | First-wave project scope only. Do not guess a user-global path. |
| **GitHub Copilot CLI** | Agent CLI / coding harness | User `${COPILOT_HOME:-~/.copilot}/mcp-config.json`; workspace `.mcp.json`, `.github/mcp.json`; session `--additional-mcp-config` | `.mcp.json`, `.github/mcp.json`; `.copilot/` is usually user-level; plugin markers `agents/`, `skills/`, `hooks.json`, `.github/plugin/marketplace.json` | Treat separately from VS Code Copilot. It has built-in MCPs, workspace loading, plugin packaging, and its own precedence model. |

Tier-1 source links to verify in MCPX-001: [Grok Build][grok-mcp],
[Devin][devin-mcp], [Google Antigravity][antigravity-mcp],
[Gemini CLI][gemini-cli-mcp], [OpenClaw][openclaw-mcp],
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

- **Status:** Done 2026-07-14 — current primary vendor documentation verified;
  promoted targets and scope constraints are recorded above.
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

- **Status:** Done 2026-07-14 — every promoted target has a scope-aware output
  adapter and verification coverage.
- **Intent:** Generate correct MCP server configuration for each accepted first-wave client.
- **Expected Outcome:** `anvil mcp-config --target <client>` supports the promoted
  client targets, emits valid config, and `--verify` reports missing/malformed
  entries without writing.
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_config`
- **Dependencies:** MCPX-001
- **Confidence:** medium

### MCPX-003: Extend `anvil mcp install --client`

- **Status:** Done 2026-07-14 — managed JSON/TOML adapters preserve unrelated
  settings, write atomically, refuse foreign Anvil entries and unsafe
  symlinks, and rewrite only Anvil-owned drift.
- **Intent:** Make installation for accepted clients a one-command, idempotent operation.
- **Expected Outcome:** `anvil mcp install --client <client>` resolves the client
  config location, writes the Anvil server entry safely, preserves unrelated user
  config, rewrites Anvil-owned drift, and prints accurate restart/reload guidance.
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_config`
- **Dependencies:** MCPX-002
- **Confidence:** medium

### MCPX-004: Activation detection and install picker

- **Status:** Done 2026-07-14 — `anvil start` uses the shared registry, strong
  detection, explicit client/scope controls, and consent-preserving TUI/plain
  flows.
- **Intent:** Teach `anvil start` to offer newly supported clients only when there is strong evidence they are installed or explicitly requested.
- **Expected Outcome:** The activation install picker includes promoted clients
  with real detection signals, skips undetected clients by default, honours an
  explicit all-clients opt-in, and avoids phantom "AI tools detected" claims.
- **Validation:** `cargo test -p eddacraft-anvil activation::mcp_client`
- **Dependencies:** MCPX-003, ACTMO-012
- **Confidence:** medium

### MCPX-005: Client compatibility smoke coverage

- **Status:** Done 2026-07-14 — fixture-backed config tests cover every
  promoted shape and the canonical stdio protocol smoke covers initialise,
  tool listing, and a safe tool call through the shared server command.
- **Intent:** Prove promoted clients can launch and communicate with `anvil mcp serve --stdio`.
- **Expected Outcome:** Fixture-backed or client-harness smoke tests cover config
  generation, process launch shape, `initialize`, `tools/list`, and at least one
  safe `tools/call` per promoted client.
- **Validation:** Targeted MCP client smoke tests documented in the item closeout;
  exact command depends on MCPX-001 client selection.
- **Dependencies:** MCPX-003, RMCPF-030
- **Confidence:** low

### MCPX-006: Docs and troubleshooting

- **Status:** Done 2026-07-14 — the public integration guide documents the
  promoted matrix, scope constraints, install/verify commands, reload
  guidance, and deferred Devin automation.
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
4. **One agent registry** — SKPKG and MCPX share detection identity and
   capability metadata while keeping skill discovery and MCP support as
   independent, evidence-backed flags.
5. **Constrained support is explicit** — profile-owned or conflicting global
   paths delegate to the vendor CLI or remain project-only; support output names
   that constraint instead of guessing.

## Notes

- The module remains `In Progress` and its stored `0/6` aggregate is unchanged
  in this feature PR even though item statuses are authoritative. Per
  `plans/project-context.md` and ADR-053, aggregate counters are reconciled by a
  separate single-writer bookkeeping change after merge.
- Claude Code and Cursor remain supported baselines. The first wave adds
  Codex, OpenCode, Gemini CLI, Antigravity, OpenClaw, VS Code, Copilot CLI,
  Grok Build, Warp, and project-scoped Zed.
- Devin and Visual Studio remain deferred because their verified automation
  contracts do not meet this wave's local stdio configuration boundary.
- Continue and Windsurf remain outside this wave pending fresh evidence.

[anthropic-mcp]: https://docs.anthropic.com/en/docs/claude-code/mcp
[antigravity-mcp]: https://codelabs.developers.google.com/developer-knowledge-mcp-antigravity
[claude-mcp]: https://code.claude.com/docs/en/mcp-quickstart
[codex-config]: https://developers.openai.com/codex/config-basic
[copilot-cli-mcp]: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers
[cursor-mcp]: https://cursor.com/docs/mcp
[devin-mcp]: https://cli.devin.ai/docs/extensibility/mcp/overview
[grok-mcp]: https://docs.x.ai/build/features/mcp-servers
[gemini-cli-mcp]: https://geminicli.com/docs/tools/mcp-server/
[openclaw-mcp]: https://docs.openclaw.ai/cli/mcp
[opencode-config]: https://opencode.ai/docs/config/
[visual-studio-mcp]: https://learn.microsoft.com/en-us/visualstudio/ide/mcp-servers?view=visualstudio
[vscode-mcp]: https://code.visualstudio.com/docs/agent-customization/mcp-servers
[warp-mcp]: https://docs.warp.dev/agent-platform/capabilities/mcp/
[zed-mcp]: https://zed.dev/docs/assistant/model-context-protocol
