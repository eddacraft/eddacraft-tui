# anvil CLI architecture

| Type         | Authority | Owner          | Status | Freshness                                                                                                                                          |
| ------------ | --------- | -------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Architecture | Derived   | CLI/LAUNCH/MCP | Live   | Last reviewed 2026-08-20 against `f0f834b39`, `src/activation/**`, `src/mcp/**`, `src/tui.rs`, their tests, ADR-092, ADR-106, ADR-113, and ADR-123 |

| Upstream                                                                                      | Downstream                                                                                              |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `crates/anvil-cli/src/**`, kernel/intercept contracts, ADR-092, ADR-106, ADR-113, and ADR-123 | `anvil start`, MCP clients, interactive CLI commands, and the [anvil TUI](../anvil-tui/ARCHITECTURE.md) |

## Scope and boundaries

This document owns three CLI implementation concerns:

- activation orchestration under [`src/activation/`](src/activation);
- the in-binary MCP server under [`src/mcp/`](src/mcp); and
- terminal lifecycle and event-loop integration in [`src/tui.rs`](src/tui.rs).

Command-specific product behaviour remains with the command module. The kernel,
checks, intercept protocol, and TUI component own their internal contracts. The
cross-system [architecture overview](../../docs/architecture/overview.md),
[auth map](../../docs/architecture/auth-as-built.md), and
[MCP server specification](../../docs/architecture/rust-mcp-server-spec.md)
remain the authorities for their wider concerns.

## Activation orchestration

[`orchestrator/mod.rs`](src/activation/orchestrator/mod.rs) composes detection,
daemon evidence, worktree registration, baseline creation, language profiling,
optional MCP installation, and rendering into one
[`ActivationDiagnostic`](src/activation/diagnostic.rs). Mutating `anvil start`
and its read-only verification path share the same diagnostic vocabulary; a
renderer must not infer a stronger claim from a successful setup step.

The load-bearing honesty contract is the six-value
[`ProtectionState`](src/activation/state.rs):

- `protecting` requires live pre-write validation evidence;
- `ready_restart_required` means configuration is present but the client still
  needs a restart;
- `watching` is a weaker daemon-backed or save-time fallback;
- `needs_action` identifies an actionable incomplete setup;
- `unsupported` names an uncovered platform, profile, or environment; and
- `error` is a hard failure with a diagnostic and repair path.

The enum owns both serialised labels and human headlines. `start`,
`status --verify`, `doctor`, and tutorial copy consume that vocabulary rather
than inventing local success words. ADR-092 makes MCP an optional upgrade on the
activation spine, not the only route to useful watch-time feedback.

### Client registry and installation

[`agent_registry.rs`](src/activation/agent_registry.rs) is the client identity
and configuration-shape registry. Detection and requested installation are
separate: discovering a client does not authorise a write, and an explicit
selection must use that client's supported scope and configuration kind. Managed
writes carry provenance and are designed to be idempotent. ADR-106 owns the
registry decision; client-specific rendering lives below
[`mcp_client/`](src/activation/mcp_client).

Activation failures remain observable in the diagnostic. A partial setup must
not be rendered as `protecting`, and the read-only path must not mutate editor
configuration, hooks, or baselines.

## MCP shim

The MCP shim is served from [`src/mcp/`](src/mcp). Protocol dispatch accepts MCP
lifecycle, resource, prompt, and tool requests over stdio; stdout is
protocol-only. ADR-113 owns the dual-era protocol boundary. Protocol metadata,
version negotiation, and response rendering live under
[`protocol/`](src/mcp/protocol).

The registry in [`tools/registry.rs`](src/mcp/tools/registry.rs) currently pins
fourteen tools in a test:

- write and execution surfaces: `anvil_validate_write`, `anvil_apply_patch`,
  `anvil_gate`, `anvil_suppress`, and `anvil_fix`;
- read and inspection surfaces: `anvil_status`, `anvil_check`,
  `anvil_query_boundary`, `anvil_search_symbols`, `anvil_find_dependents`,
  `anvil_find_callers`, `anvil_impact_of_change`, `anvil_affected_tests`, and
  `anvil_symbol_context`.

The registry, not this count in prose, is canonical. Mutating or
execution-triggering tools require authentication; read-only graph-context tools
also pass the daemon's workspace-root admission boundary and charge successful
identity-only output against the graph egress budget.

### Validation and fallback

A pre-write request is normalised and contained within the admitted workspace
before scanning. [`tools/validate_write.rs`](src/mcp/tools/validate_write.rs)
handles complete content, preview plus digest, and patch inputs;
[`tools/apply_patch.rs`](src/mcp/tools/apply_patch.rs) is the lean patch
surface. Both return the shared decision vocabulary and correlation envelope.

The daemon-backed route and the embedded scanner are correctness alternatives,
not different policy products. Transport errors, malformed replies, and
shape-violating replies must not be interpreted as a clean result. Enforcement
mode is resolved in [`enforcement.rs`](src/mcp/enforcement.rs); the MCP default
is `Interrupt`, while `block` remains a compatibility alias for that vetoing
posture. Workspace paths and operator identity are redacted from egress where
the resource/tool contract requires it.

Graph-context reads are identity-only by default. Source snippets require both
workspace egress enablement and a request that opts into source. The graph may
return `not_ready`, `unavailable`, or `disabled`; callers must preserve those
outcomes instead of presenting an empty graph as authoritative.

## CLI TUI runner

[`src/tui.rs`](src/tui.rs) owns the terminal session, not surface state.
[`TerminalGuard`](src/tui.rs) enables raw mode and the alternate screen, rolls
back partial setup, restores on explicit leave or `Drop`, and installs a panic
hook that restores the terminal before the previous hook reports a panic. The
autoplay tutorial has an explicit contained-panic boundary so a recovered worker
panic does not corrupt the live frame; it records the diagnostic through the
structured logging path.

[`run_surface_with_exit`](src/tui.rs) returns `SurfaceExit::Quit` or
`SurfaceExit::Back`. [`TuiSession`](src/tui.rs) keeps one terminal session open
across multi-phase flows. The standard loop draws shell chrome, renders the
surface, maps keyboard input through `KeyHandler`, preserves text-entry
semantics, advances animations, and exits only when the surface reports quit or
back.

Tutorial and watch use specialised runners because they consume live channels.
Tutorial forwards file-change paths to the tutorial state machine. Watch adapts
kernel events, coalesces snapshot-driven work, and uses a dirty-frame gate;
animation can still request a redraw. Channel disconnects and runner failures
return errors rather than silently claiming a healthy session.

Surface rendering and navigation contracts belong to
[the anvil TUI architecture](../anvil-tui/ARCHITECTURE.md). Shared theme,
keyboard, widget, lifecycle, and snapshot contracts belong to
[`eddacraft-tui`](../eddacraft-tui/README.md).

## Failure and trust invariants

- Activation claims are evidence-based; written configuration is not proof of
  live pre-write protection.
- MCP requests are workspace-contained before file or graph access.
- A vetoing decision, malformed daemon reply, or failed scan cannot become an
  implicit allow.
- MCP stdout contains protocol frames only; diagnostics use the designated
  error/log channels.
- Terminal raw mode and alternate-screen state are restored on normal exit,
  setup failure, and unwinding panic.
- TUI state remains render-only; filesystem and process I/O stay in the CLI
  runner or command layer.

## Decisions, retained authorities, and history

- [ADR-092](../../plans/decisions/092-mcp-optional-activation-spine.md) governs
  the activation spine and honest fallback.
- [ADR-106](../../plans/decisions/106-agent-integration-registry-and-managed-installers.md)
  governs the client registry and managed installers.
- [ADR-113](../../plans/decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md)
  governs the MCP protocol transition.
- [ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
  governs this component-local authority.
- [`rust-mcp-server-spec.md`](../../docs/architecture/rust-mcp-server-spec.md)
  retains MCP design intent; this document records the current CLI
  implementation boundary.
- The former activation, MCP-shim, and CLI-runner as-builts remain at their
  original central paths as compatibility and history records. Use
  `git log --follow -- <path>` to inspect their pre-migration detail.
