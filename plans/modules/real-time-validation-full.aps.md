# Real-Time Validation: Full Scope (Unified Validation Server)

## Overview

Build a unified validation server with three interfaces (LSP, HTTP, stdin) that provides real-time validation for editors, IDE AI agents, and CLI AI tools, plus a comprehensive notification framework for alerting users to validation issues across 25+ concurrent agents.

**Current State:** No real-time validation infrastructure. Watch mode exists but is save-triggered only and lacks reasoning quality checks. No notification system for alerting users when validation issues occur.

**Target State:** Single validation server serving:
- **LSP Protocol** → All editors (VS Code, Vim, Emacs, IntelliJ)
- **HTTP REST API** → IDE AI agents (Copilot, Cursor, Cody)
- **stdin/stdout** → CLI AI tools (Aider, Continue, Claude Code)
- **Notification Framework** → Terminal, Desktop, Slack alerts for validation errors

**Scope:** Complete validation infrastructure with multi-interface server, AI tool integrations, notification framework, and comprehensive documentation.

## Problem Statement

**User Pain Points:**

1. **Editor lock-in** — Each editor needs custom integration
2. **AI tool fragmentation** — Cursor, Aider, Copilot each need different integration
3. **No unified standard** — Can't reuse validation across tools
4. **Manual review required** — AI output has no automated quality check
5. **Late feedback loop** — Issues found at PR review, not during generation
6. **Silent failures** — With 25+ agents running, validation errors go unnoticed
7. **Context switching** — Must manually check TUI tab to see if errors occurred

**Success Criteria:**

- [ ] Single validation server supports 15+ tools (editors + AI tools)
- [ ] Validation completes in <150ms regardless of interface
- [ ] LSP works in VS Code, Vim, Neovim, Emacs, IntelliJ
- [ ] HTTP API used by Cursor, Copilot, Cody integrations
- [ ] stdin mode works with Aider, Continue, Claude Code
- [ ] Zero code duplication across interfaces
- [ ] Single binary deployment (`anvil-server`)
- [ ] Notification framework alerts users within 5 seconds of errors
- [ ] Terminal, Desktop, and Slack notifications work reliably
- [ ] Notification rate limiting prevents alert fatigue with 25+ agents

## Solution

### Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                  @eddacraft/anvil-validation-server                         │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │             Validation Core Engine                       │    │
│  │  • validateSchema()                                      │    │
│  │  • scanAntipatterns()                                   │    │
│  │  • validateAIReasoning()                                │    │
│  │  • lintMarkdown()                                       │    │
│  │  • checkLinks()                                         │    │
│  │  Performance: ~115ms total                              │    │
│  └──────────────────────┬──────────────────────────────────┘    │
│                         │                                        │
│                         ▼                                        │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │          Notification Service                            │    │
│  │  • Aggregation (5s window)                              │    │
│  │  • Rate limiting (per channel)                          │    │
│  │  • Severity filtering                                   │    │
│  │  • Priority escalation                                  │    │
│  └──────┬──────────┬──────────┬─────────────────────┘          │
│         │          │          │                                 │
│    ┌────▼────┐ ┌───▼────┐ ┌──▼──────┐                          │
│    │Terminal │ │Desktop │ │ Slack   │                          │
│    │ (Bell)  │ │ (Toast)│ │(Webhook)│                          │
│    └─────────┘ └────────┘ └─────────┘                          │
│                                                                  │
│      ┌───────────────┬───────────────┬─────────────┐           │
│      │  LSP Protocol │   HTTP API    │   stdin     │           │
│      │  (TCP/stdio)  │   (:3737)     │   (JSON)    │           │
│      └───────┬───────┴───────┬───────┴──────┬──────┘           │
└──────────────┼───────────────┼──────────────┼──────────────────┘
               │               │              │
               ▼               ▼              ▼
        ┌──────────┐    ┌──────────┐   ┌──────────┐
        │  Editors │    │AI Agents │   │ CLI AI   │
        │          │    │ (REST)   │   │ Tools    │
        │ VS Code  │    │          │   │          │
        │ Vim      │    │ Copilot  │   │ Aider    │
        │ Emacs    │    │ Cursor   │   │ Continue │
        │ IntelliJ │    │ Cody     │   │ Claude   │
        └──────────┘    └──────────┘   └──────────┘
```

### Core Components

**1. Validation Server** (`packages/validation-server/src/`)

Single Node.js process with three interfaces:

- `lsp.ts` — LSP protocol handler
- `http.ts` — Express REST API
- `stdin.ts` — Newline-delimited JSON processor
- `core/validator.ts` — Shared validation engine

**2. Interface Implementations**

**LSP Protocol:**
```typescript
// Auto-detects when launched via editor
connection.onDidChangeTextDocument(async (change) => {
  const diagnostics = await validateInMemory(change.document.getText());
  connection.sendDiagnostics({ uri: change.document.uri, diagnostics });
});
```

**HTTP REST API:**
```typescript
// POST /validate - Real-time validation
app.post('/validate', async (req, res) => {
  const { content, filePath, options } = req.body;
  const result = await validateInMemory(content, options);
  res.json({ issues: result.issues, duration: result.duration });
});
```

**stdin/stdout:**
```typescript
// Newline-delimited JSON
rl.on('line', async (line) => {
  const request = JSON.parse(line);
  const result = await validateInMemory(request.content);
  process.stdout.write(JSON.stringify(result) + '\n');
});
```

**3. AI Tool Integrations**

**Cursor IDE Extension:**
```typescript
// cursor-anvil/src/extension.ts
cursor.onDidGenerateCode(async (event) => {
  const response = await fetch('http://localhost:3737/validate', {
    method: 'POST',
    body: JSON.stringify({ content: event.text })
  });
  const result = await response.json();
  if (result.issues.length > 0) {
    cursor.showPanel('anvil-validation', { issues: result.issues });
  }
});
```

**Aider Configuration:**
```yaml
# .aider.conf.yml
lint-cmd: "anvil-server --stdin"
# Aider pipes generated code through anvil-server
# Shows issues to AI for automatic fixing
```

**Claude Code MCP Tool:**
```typescript
// anvil-mcp-server/src/tools/validate.ts
export const validatePlanTool: Tool = {
  name: 'validate_plan',
  description: 'Validate planning document reasoning quality',
  async execute({ content }) {
    const response = await fetch('http://localhost:3737/validate', {
      method: 'POST',
      body: JSON.stringify({ content })
    });
    return await response.json();
  }
};
```

## Implementation

### Phase 1: Validation Server Core (5 days)

**Goal:** Single-binary server with three interfaces sharing validation core

**Tasks:**

#### SERVER-001: Extract validation core

**Intent:** Create shared validation engine used by all interfaces

**Implementation:**
1. Create `packages/validation-server/` package
2. Move fast validator from simplified scope to `src/core/validator.ts`
3. Export unified API: `validateInMemory(content, options)`
4. Add TypeScript types for all interfaces
5. Unit tests for core validation

**Acceptance:**
- Single validation function used by all interfaces
- <150ms execution time
- Content-hash caching works
- All checks (schema, antipatterns, AI reasoning) available

**Confidence:** high (extraction from simplified scope, well-defined interface)

**Dependencies:** Requires simplified scope completion (fast validator exists)

#### SERVER-002: Implement HTTP REST API

**Intent:** Express server providing POST /validate endpoint

**Implementation:**
1. Create `src/http.ts` with Express app
2. POST /validate endpoint:
   - Body: `{ content, filePath, options }`
   - Response: `{ issues, duration, timestamp }`
3. GET /health endpoint for monitoring
4. CORS support for web-based tools
5. Error handling and logging
6. Integration tests

**Acceptance:**
- Responds in <160ms (150ms validation + 10ms HTTP overhead)
- CORS headers present
- Graceful error handling
- Health check works

**Confidence:** high (Express REST API is straightforward)

#### SERVER-003: Implement stdin/stdout interface

**Intent:** Newline-delimited JSON protocol for CLI tools

**Implementation:**
1. Create `src/stdin.ts` with readline interface
2. Parse JSON from stdin (one request per line)
3. Validate and write response to stdout (one JSON per line)
4. Error responses to stderr
5. Streaming mode support
6. Integration tests with echo piping

**Acceptance:**
- Processes requests in <120ms (150ms validation + minimal I/O)
- Handles malformed JSON gracefully
- Works with standard Unix pipes
- Backpressure handling for fast input

**Confidence:** high (stdin/stdout is simple, well-understood pattern)

#### SERVER-004: Implement LSP protocol

**Intent:** Language Server Protocol for editor integration

**Implementation:**
1. Create `src/lsp.ts` using `vscode-languageserver`
2. Implement textDocument/didChange handler
3. Send diagnostics on document changes
4. Support both stdio and TCP transports
5. Debouncing (300ms) for human typing
6. Integration tests

**Acceptance:**
- Works with VS Code, Vim (via coc.nvim)
- Diagnostics appear in <450ms (300ms debounce + 150ms validation)
- Graceful shutdown handling
- Multiple clients supported (TCP mode)

**Confidence:** medium (LSP protocol is complex, requires thorough testing)

#### SERVER-005: Build unified server launcher

**Intent:** Single binary that auto-detects which interface to use

**Implementation:**
1. Create `src/index.ts` with mode detection:
   - If stdin is pipe → stdin mode
   - If `--lsp` flag → LSP mode
   - If `--http` flag → HTTP mode
   - Default → HTTP + LSP simultaneously
2. Add CLI arguments (--port, --debounce, --checks)
3. Graceful shutdown handlers
4. Logging configuration
5. Integration tests for all modes

**Acceptance:**
- `anvil-server` works in all three modes
- Auto-detection is reliable
- Flags override auto-detection
- Logs to appropriate streams

**Confidence:** high (mode detection logic is simple)

### Phase 2: Editor Integration (3 days)

**Goal:** LSP clients configured for major editors

**Tasks:**

#### EDITOR-001: VS Code LSP client

**Intent:** Enable Anvil validation in VS Code via LSP

**Implementation:**
1. Update `packages/vscode-extension/package.json`:
   - Add language server configuration
   - Point to `anvil-server --lsp`
2. Configure server spawn in extension activation
3. Register document selectors (markdown, *.aps.md)
4. Handle server lifecycle (start/stop/restart)
5. User settings for enabling/disabling

**Acceptance:**
- Server starts automatically with VS Code
- Diagnostics appear on document change
- Settings persist across sessions
- Clean shutdown on VS Code exit

**Confidence:** high (VS Code LSP client is well-documented)

#### EDITOR-002: Vim/Neovim LSP client

**Intent:** Enable Anvil validation in Vim via coc.nvim or nvim-lspconfig

**Implementation:**
1. Create `coc-settings.json` example for coc.nvim
2. Create `init.lua` example for nvim-lspconfig
3. Document installation steps
4. Test with both Vim 8+ and Neovim
5. Document troubleshooting

**Acceptance:**
- Works with coc.nvim (Vim 8+)
- Works with nvim-lspconfig (Neovim 0.7+)
- Diagnostics show in location list
- Documentation is clear

**Confidence:** medium (Vim configuration can be finicky, requires testing on multiple versions)

#### EDITOR-003: Emacs LSP client

**Intent:** Enable Anvil validation in Emacs via lsp-mode

**Implementation:**
1. Create lsp-mode client definition
2. Add to user's `.emacs` or `init.el`
3. Configure markdown-mode integration
4. Document installation steps
5. Test with Emacs 27+

**Acceptance:**
- Works with lsp-mode
- Integrates with flycheck
- Keybindings for common actions
- Documentation is clear

**Confidence:** medium (Emacs configuration requires elisp knowledge, testing needed)

#### EDITOR-004: IntelliJ IDEA LSP client

**Intent:** Enable Anvil validation in IntelliJ via LSP4IJ plugin

**Implementation:**
1. Create LSP4IJ configuration
2. Document plugin installation
3. Configure file type associations
4. Test with IntelliJ IDEA, PyCharm, WebStorm
5. Document platform-specific quirks

**Acceptance:**
- Works with LSP4IJ plugin
- Diagnostics show in IntelliJ's problems panel
- Hot-reload supported
- Multi-project support works

**Confidence:** low (IntelliJ LSP support is less mature, may have limitations)

### Phase 3: AI Tool Integrations (5 days)

**Goal:** Working integrations with 5 major AI tools

**Tasks:**

#### AI-001: Aider integration

**Intent:** Aider validates generated code via stdin interface

**Implementation:**
1. Create `.aider.conf.yml` template
2. Configure `lint-cmd: "anvil-server --stdin"`
3. Test with Aider's post-generation hook
4. Document workflow:
   - Aider generates → pipes to anvil-server
   - Validation fails → Aider shows issues to AI
   - AI regenerates with fixes
5. Create example session

**Acceptance:**
- Aider pipes code through anvil-server correctly
- Issues shown to AI for fixing
- Works with Aider's watch mode
- Documentation is clear

**Confidence:** high (stdin interface is simple, Aider supports custom lint commands)

#### AI-002: Cursor IDE extension

**Intent:** Cursor validates AI-generated code via HTTP API

**Implementation:**
1. Create `cursor-anvil-extension/` package
2. Hook into Cursor's `onDidGenerateCode` event
3. POST generated content to anvil-server HTTP API
4. Show validation panel in Cursor UI
5. Add "Ask AI to fix" button
6. Publish to Cursor extension marketplace

**Acceptance:**
- Validates all Cursor-generated code
- Shows issues in Cursor's UI
- Auto-fix sends issues back to AI
- <200ms latency (feels instant)

**Confidence:** medium (Cursor extension API may have limitations, needs testing)

#### AI-003: GitHub Copilot integration

**Intent:** Copilot suggestions validated before acceptance

**Implementation:**
1. Update `packages/vscode-extension/`
2. Hook into VS Code's inline completion events
3. Detect large insertions (likely Copilot suggestions)
4. Validate via HTTP API
5. Show inline warnings on Copilot suggestions
6. Add quick action "Validate with Anvil"

**Acceptance:**
- Detects Copilot suggestions reliably
- Validation happens before user accepts
- Non-intrusive (doesn't block suggestions)
- Clear indication of validated vs non-validated

**Confidence:** medium (Copilot detection is heuristic-based, may have false positives)

#### AI-004: Continue.dev integration

**Intent:** Continue validates generated code via post-generation hook

**Implementation:**
1. Create `.continuerc.json` template
2. Configure `postGenerationHooks`:
   ```json
   { "command": "anvil-server --stdin", "failOnError": true }
   ```
3. Test with Continue's chat interface
4. Document workflow
5. Create example session

**Acceptance:**
- Continue pipes generated code through anvil-server
- Validation errors shown in chat
- User can ask Continue to fix issues
- Works with Continue's streaming mode

**Confidence:** high (Continue supports post-generation hooks explicitly)

#### AI-005: Claude Code MCP server

**Intent:** Claude Code can validate plans via MCP tool

**Implementation:**
1. Create `anvil-mcp-server/` package
2. Implement `validate_plan` tool:
   - Calls HTTP API (localhost:3737)
   - Returns issues as structured data
3. Add to Claude Code's MCP server registry
4. Document usage in Claude prompts
5. Create example conversation

**Acceptance:**
- Claude can call validate_plan tool
- Results formatted for Claude's understanding
- Claude uses validation in planning workflow
- Tool is discoverable via MCP protocol

**Confidence:** high (MCP tool implementation is straightforward)

### Phase 4: Documentation & Testing (2 days)

**Goal:** Comprehensive documentation and integration testing

**Tasks:**

#### DOC-003: Architecture documentation

**Intent:** Document complete validation server architecture

**Implementation:**
1. Create `packages/validation-server/README.md`
2. Sections:
   - Architecture overview
   - Interface specifications (LSP, HTTP, stdin)
   - API reference
   - Performance characteristics
   - Configuration options
3. Mermaid diagrams for data flow

**Acceptance:**
- New contributor can understand architecture in 15 minutes
- All three interfaces documented
- API reference is complete

**Confidence:** high (documentation is straightforward)

#### DOC-004: Integration guides

**Intent:** Step-by-step guides for each editor and AI tool

**Implementation:**
1. Create `docs/integrations/` directory
2. Guides for each tool:
   - editors/vscode.md
   - editors/vim.md
   - editors/emacs.md
   - editors/intellij.md
   - ai-tools/aider.md
   - ai-tools/cursor.md
   - ai-tools/copilot.md
   - ai-tools/continue.md
   - ai-tools/claude-code.md
3. Screenshots and examples

**Acceptance:**
- Each guide has setup steps, examples, troubleshooting
- Screenshots show expected results
- Copy-paste configs provided

**Confidence:** high (guides are based on implementation work)

#### TEST-003: Multi-interface integration tests

**Intent:** End-to-end tests covering all three interfaces

**Implementation:**
1. LSP client test (simulate editor connection)
2. HTTP API test (curl-based validation)
3. stdin test (echo piping)
4. Concurrent access test (all three simultaneously)
5. Performance benchmarks
6. Load testing (100 concurrent requests)

**Acceptance:**
- All interfaces work correctly
- Concurrent access doesn't cause issues
- Performance <150ms under load
- No memory leaks in long-running tests

**Confidence:** high (integration testing is well-understood)

#### TEST-004: User acceptance testing

**Intent:** Real-world testing with all supported tools

**Implementation:**
1. Install server in 5 different editors
2. Test with 5 different AI tools
3. Generate 50 plans across all tools
4. Measure latency, false positives, usability
5. Collect feedback from testers
6. Iterate on issues found

**Acceptance:**
- Server works reliably across all tools
- Latency <200ms in real-world usage
- <10% false positive rate
- User feedback >8/10

**Confidence:** medium (requires coordination with testers, issues likely to emerge)

### Phase 5: Notification Framework (6.5 days)

**Goal:** Comprehensive notification system for alerting users to validation errors across 25+ concurrent agents

**Context:** With 25+ agents running across multiple terminal tabs, users need immediate notification when validation errors occur. Without notifications, errors may go unnoticed for hours, especially when users are focused on a single tab or editing in their primary editor.

**Tasks:**

#### NOTIFY-001: Core notification service

**Intent:** Build notification service with filtering, aggregation, rate limiting, and priority escalation

**Implementation:**
1. Create `core/src/notifications/` package
2. Implement `NotificationService` class:
   - Channel registry (supports multiple notification channels)
   - Severity filtering (error, warning, info)
   - Aggregation window (collect notifications for 5s, send summary)
   - Rate limiting (per-channel limits to prevent spam)
   - Priority escalation (more errors → more aggressive notifications)
   - Quiet hours support (no notifications during sleep hours)
3. Define core types:
   ```typescript
   interface Notification {
     severity: 'error' | 'warning' | 'info';
     title: string;
     message: string;
     file?: string;
     timestamp: Date;
   }

   interface NotificationChannel {
     name: string;
     send(notification: Notification): Promise<void>;
     test(): Promise<boolean>;
   }
   ```
4. Implement aggregator:
   - Collects notifications for 5-second window
   - Groups by severity
   - Sends single summary notification
   - Example: "3 new errors in 3 files" instead of 3 separate notifications
5. Implement rate limiter:
   - Per-channel rate limits
   - Example: Max 1 desktop notification per 30 seconds
   - Prevents notification spam with 25+ agents
6. Implement filters:
   - Severity-based routing (errors → all channels, warnings → terminal only)
   - Quiet hours (no notifications 11pm-7am except critical)
7. Unit tests for all components

**Acceptance:**
- [ ] Can register multiple notification channels
- [ ] Aggregates notifications within 5-second window
- [ ] Rate limits work per channel (tested with rapid-fire notifications)
- [ ] Severity filters route correctly (errors vs warnings vs info)
- [ ] Quiet hours respected (time-based test)
- [ ] Unit test coverage >90%

**Confidence:** high (well-defined problem, clear architecture)

**Dependencies:** None (independent of validation server)

#### NOTIFY-002: P0 notification channels + Slack webhook

**Intent:** Implement essential notification channels (Terminal, Desktop, Sound, Colour, Slack)

**Implementation:**

1. **Terminal Channel** (`channels/terminal.channel.ts`)
   - System bell on error (`\a`)
   - Terminal title update (`\033]0;Anvil: 🔴 3 errors\007`)
   - tmux status bar integration (`tmux set-option status-right`)
   - Works on Linux, macOS, Windows

2. **Desktop Channel** (`channels/desktop.channel.ts`)
   - Linux: `notify-send` (freedesktop notifications)
   - macOS: `osascript` with `display notification`
   - Windows: PowerShell `New-BurntToastNotification`
   - Auto-detect OS and use appropriate command
   - Configurable urgency levels (low, normal, critical)

3. **Sound Channel** (`channels/sound.channel.ts`)
   - System beep (default, works everywhere)
   - Custom .wav file support (configurable paths)
   - Different sounds for error/warning/success
   - Silence mode for quiet hours

4. **Colour Channel** (`channels/colour.channel.ts`)
   - TUI visual feedback
   - Flash red background on new error
   - Yellow highlight for warnings
   - Green for success
   - Integrates with TUI component

5. **Slack Webhook Channel** (`channels/webhook.channel.ts`)
   - Generic webhook implementation (works with Slack, Discord, etc.)
   - POST JSON to configured URL
   - Configurable payload format
   - Retry logic on failure (3 retries with exponential backoff)
   - Example Slack payload:
     ```json
     {
       "text": "🔴 Anvil: 3 errors in auth-strategy.md",
       "channel": "#anvil-alerts",
       "username": "Anvil Bot",
       "icon_emoji": ":hammer:"
     }
     ```

6. Channel auto-detection and configuration
7. Integration tests for each channel

**Acceptance:**
- [ ] Terminal: bell works, title updates, tmux integration confirmed
- [ ] Desktop: notifications work on Linux/macOS/Windows
- [ ] Sound: system beep works, custom .wav playback works
- [ ] Colour: TUI visual feedback works (red flash, yellow highlight)
- [ ] Slack: webhook POST succeeds, retry logic works
- [ ] All channels tested on target platforms
- [ ] Error handling for missing dependencies (e.g., notify-send not installed)

**Confidence:** high (all channels use well-established patterns)

**Dependencies:** NOTIFY-001 (requires notification service)

#### NOTIFY-003: Configuration & CLI

**Intent:** YAML configuration and CLI commands for notification management

**Implementation:**
1. Create `.anvil/notifications.yml` schema:
   ```yaml
   channels:
     terminal: true
     desktop: true
     sound: true
     colour: true
     slack: true

   routing:
     error: [terminal, desktop, sound, colour, slack]
     warning: [terminal, colour]
     info: [colour]

   aggregation:
     enabled: true
     window: 5000  # milliseconds

   rate_limits:
     desktop:
       max: 1
       window: 30000
     slack:
       max: 1
       window: 300000  # 5 minutes

   channel_config:
     slack:
       webhook_url: "https://hooks.slack.com/services/YOUR/WEBHOOK"
       channel: "#anvil-alerts"
   ```

2. CLI commands:
   - `anvil notifications list` — Show enabled channels
   - `anvil notifications test [channel]` — Send test notification
   - `anvil notifications enable <channel>` — Enable a channel
   - `anvil notifications disable <channel>` — Disable a channel

3. Config validation with Zod schema
4. Helpful error messages for misconfiguration
5. Documentation in code comments

**Acceptance:**
- [ ] YAML config loads correctly
- [ ] Config validation catches common errors
- [ ] CLI commands work as expected
- [ ] Test command verifies channel works before enabling
- [ ] Clear error messages for misconfigurations

**Confidence:** high (straightforward CLI + config work)

**Dependencies:** NOTIFY-002 (requires channels to exist)

#### NOTIFY-004: TUI integration

**Intent:** Show notification status in TUI dashboard

**Implementation:**
1. Add notification panel to TUI:
   ```
   Notifications:
   ├─ 🔔 Terminal: Enabled
   ├─ 🖥️  Desktop: Enabled (1 sent, rate limited)
   ├─ 🔊 Sound: Enabled
   ├─ 🎨 Colour: Enabled
   └─ 💬 Slack: Enabled (last: 2m ago)

   Press [n] to configure notifications
   ```

2. Interactive config:
   - Press 'n' → opens notification config screen
   - Toggle channels on/off
   - Test channels inline
   - View recent notification history

3. Real-time status updates:
   - Show last sent time per channel
   - Show rate limit status
   - Show notification queue depth

4. Colour channel integration (flash red on error)

**Acceptance:**
- [ ] TUI shows notification status panel
- [ ] Interactive config works (toggle channels)
- [ ] Real-time updates display correctly
- [ ] Colour flash works on new errors
- [ ] Keyboard shortcuts documented

**Confidence:** high (TUI already exists, adding panel is straightforward)

**Dependencies:** NOTIFY-003 (requires config), TUI-013-015 (requires TUI dashboard)

#### NOTIFY-005: Documentation

**Intent:** Complete user and developer documentation for notification system

**Implementation:**
1. User guide: `docs/notifications.md`
   - Overview of notification system
   - Setup guide for each channel
   - Configuration examples
   - Troubleshooting common issues
   - Slack webhook setup (with screenshots)
   - Desktop notification permissions (macOS, Linux)

2. Example configs:
   - Solo developer (terminal + desktop)
   - Small team (+ Slack)
   - Large team with quiet hours
   - CI/CD integration

3. Developer guide:
   - Adding custom notification channels
   - Channel interface documentation
   - Testing notifications
   - Debugging

4. Troubleshooting section:
   - "Desktop notifications not appearing" → check permissions
   - "Slack webhook 404" → verify URL
   - "Too many notifications" → adjust rate limits

**Acceptance:**
- [ ] User can set up notifications in <10 minutes
- [ ] All channels documented with examples
- [ ] Troubleshooting covers 90% of common issues
- [ ] Developer guide enables custom channel creation

**Confidence:** high (documentation based on implementation)

**Dependencies:** NOTIFY-004 (requires implementation complete)

## Dependencies

**External:**
- `vscode-languageserver` — LSP protocol implementation
- `express` — HTTP server
- `readline` — stdin/stdout processing
- Simplified scope completion — Fast validator core

**Internal:**
- Simplified scope (VALID-001, VALID-002, VALID-003) must be complete first

## Testing Strategy

### Unit Tests

- Core validator: All checks in isolation
- HTTP API: Request/response handling
- stdin processor: JSON parsing and streaming
- LSP handlers: Protocol message handling

### Integration Tests

- End-to-end: Each interface with real validation
- Multi-interface: All three running simultaneously
- Performance: <150ms validation under load
- Concurrent access: Thread safety

### User Acceptance Testing

- 5 editors × 10 test documents = 50 editor tests
- 5 AI tools × 10 generated plans = 50 AI tool tests
- Latency measurements
- False positive/negative tracking
- User feedback surveys

## Documentation

**User Documentation:**
- [ ] Architecture overview (`packages/validation-server/README.md`)
- [ ] Installation guide for each editor
- [ ] Integration guide for each AI tool
- [ ] Configuration reference
- [ ] Troubleshooting guide

**Developer Documentation:**
- [ ] Server API documentation
- [ ] Adding new interfaces
- [ ] Protocol specifications
- [ ] Contributing guide

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| LSP protocol complexity | high | medium | Use battle-tested library (vscode-languageserver), extensive testing |
| AI tool APIs change | high | medium | Version lock integrations, monitor for breaking changes |
| Performance degradation under load | high | low | Load testing, caching, rate limiting |
| Editor-specific LSP bugs | medium | high | Test on multiple editors, document known issues |
| HTTP API security concerns | high | low | Localhost-only by default, add auth for remote access |
| Concurrent access race conditions | high | low | Use Node.js single-threaded model, async/await properly |

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Editors supported | 4+ (VS Code, Vim, Emacs, IntelliJ) | Installation testing |
| AI tools supported | 5+ (Aider, Cursor, Copilot, Continue, Claude) | Integration testing |
| Validation latency | <150ms | Timing instrumentation |
| HTTP latency | <160ms | Benchmarking under load |
| LSP latency | <450ms | Debounce + validation timing |
| Server uptime | >99.9% | 24h stress test |
| Code duplication | 0% | Architecture review (single validation core) |
| User satisfaction | >8/10 | Post-release survey |
| Notification latency | <5s from error to alert | End-to-end timing |
| Notification channels | 5 (Terminal, Desktop, Sound, Colour, Slack) | Integration testing |
| Rate limit effectiveness | <10 notifications/hour with 25+ agents | Real-world usage testing |
| Aggregation effectiveness | 90% fewer notifications vs no aggregation | A/B comparison testing |

## Comparison: Simplified vs Full Scope

| Feature | Simplified Scope | Full Scope | Benefit of Full |
|---------|------------------|------------|-----------------|
| **Terminal watch mode** | ✅ | ✅ | - |
| **VS Code integration** | ⚠️ (via watch) | ✅ (LSP) | In-editor squiggly lines |
| **Vim/Emacs integration** | ⚠️ (via watch) | ✅ (LSP) | Native editor experience |
| **Cursor integration** | ⚠️ (watch detects files) | ✅ (HTTP API) | Real-time during generation |
| **Aider integration** | ⚠️ (watch detects files) | ✅ (stdin) | Automatic AI fixing loop |
| **Copilot validation** | ❌ | ✅ (HTTP) | Pre-acceptance validation |
| **Notification framework** | ✅ (P0 channels) | ✅ (P0 + Slack) | Team awareness via Slack |
| **Terminal notifications** | ✅ | ✅ | - |
| **Desktop notifications** | ✅ | ✅ | - |
| **Slack integration** | ❌ | ✅ | Team-wide error visibility |
| **Implementation time** | 10 days | 22 days | +12 days |
| **Maintenance burden** | Low | Medium | +3 interfaces + 5 channels |
| **Editor coverage** | Any (via terminal) | All (native) | Better UX |
| **AI tool coverage** | Any (via files) | All (native) | Tighter integration |

## Open Questions

- [x] Should HTTP API require authentication? **Decision:** Localhost-only by default, optional API key for remote
- [x] Should LSP server be separate binary or same as HTTP? **Decision:** Same binary, auto-detects mode
- [ ] Should we support WebSocket for real-time updates? **Decision:** TBD, may add in future if demand exists
- [ ] How to handle server crashes? **Decision:** TBD, likely auto-restart via systemd/pm2
- [ ] Should editors auto-start server or require manual start? **Decision:** TBD after UX testing

## Future Work (Out of Scope)

**Not included in full scope:**

- ❌ Auto-fix generation (AI-powered suggestions)
- ❌ Web UI dashboard (browser-based monitoring)
- ❌ CI/CD pipeline integration (GitHub Actions, GitLab CI)
- ❌ Cloud-hosted validation service (SaaS offering)
- ❌ Collaborative validation (team-wide config sync)
- ❌ Custom pattern authoring UI (visual pattern builder)

**Notification-related future work:**

**FUTURE-001: Investigation - Mobile/Email notification options (1 day)**

**Intent:** Research feasibility and implementation approaches for mobile push notifications and email notifications

**Scope:**
- Evaluate mobile push services (Pushover, Telegram, custom app)
- Evaluate email options (SMTP providers, transactional email services)
- Assess cost/benefit vs P0 channels (terminal, desktop, sound, Slack)
- Document findings and recommendation
- Note: All options work client-side (no Anvil hosting required)
  - Mobile: User provides Pushover/Telegram API keys
  - Email: User provides SMTP credentials (Gmail, SendGrid, etc.)

**Output:** Decision document with implementation estimate and priority recommendation

**Priority:** P2 (after P0 channels proven in production)

**These may be considered for future phases.**

---

**Status:** Draft
**Priority:** Medium (after simplified scope validates value)
**Dependencies:** Simplified scope must be complete first
**Target Milestone:** v0.7.0 — Unified Validation Server with Notifications
**Estimated Effort:** 22 days (5 + 3 + 5 + 2 + 6.5, rounded to 22)
