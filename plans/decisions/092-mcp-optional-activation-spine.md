# ADR-092: MCP-Optional Activation Spine

## Status

Proposed

## Date

2026-06-26

## Context

`anvil start` is the daily golden path. Beta smoke on **v0.8.2-beta** (Windows,
Matt + Dave) shows the path still parks at `ready_restart_required` with
`worktree_unenforced` even when:

- the per-user save-time daemon is up and answering IPC (CIB-072 ensure succeeded),
- MCP clients report `restart_handshake_verified`, and
- L0 `anvil_validate_write` prompts fire in Claude Code.

The failure mode is **not** `daemon_unreachable` (what CIB-072 / #2609 targeted).
Promotion is skipped because the worktree never registers with the intercept
daemon — session registration today expects `anvil-run` wrapped launches or
PreToolUse hooks (`register-session`) per ADR-015, and `anvil start` does not
install hooks.

Separately, MCP is:

- optional (many users block MCP in corporate environments),
- per-editor and unreliable (approval prompts, restart handshakes),
- worktree-hostile (stdio shims, no daemon session registration), and
- insufficient alone for L1/L2 save-time protection.

Yet activation state machines and copy still treat MCP handshake completion as
the primary gate to `Protecting`, leaving honest daemon-backed posture stuck at
`ready_restart_required`.

Evidence: GH [#2937](https://github.com/eddacraft/anvil-001/issues/2937),
recurrence of [#1831](https://github.com/eddacraft/anvil-001/issues/1831) /
[#2583](https://github.com/eddacraft/anvil-001/issues/2583). APS module:
[`activation-mcp-optional`](../modules/activation-mcp-optional.aps.md) (ACTMO).

## Decision

Adopt an **MCP-optional activation spine** for `anvil start`:

1. **Spine (required):** per-user daemon ensure → worktree registration with
   the intercept daemon → git hooks (where policy allows) → save-time validation
   armed (L1/L2). This path must succeed without any MCP surface.
2. **MCP (optional upgrade):** when the editor permits MCP, install/configure
   `mcpServers.anvil` (ADR-044) and add tool-approval allow rules where the
   client supports them. MCP provides L0 pre-write validation; it does **not**
   substitute for worktree registration.
3. **Honest states:** `Protecting` requires daemon attestation
   (`WorktreeClaimState::Enforced` or equivalent promotion predicate per
   MLP2-051f), not MCP handshake alone. When MCP cannot close the loop but the
   spine is live, fall through to an honest daemon-backed state (`watching` /
   save-time armed) rather than indefinite `ready_restart_required`.
4. **Corporate opt-out:** `--no-mcp` / `ANVIL_NO_MCP` skips MCP install and
   MCP-gated promotion; the spine still runs.
5. **Graph:** graph warm follows daemon content events; it is not a blocker for
   activation success reporting.

## Consequences

### Positive

- `anvil start` works for MCP-blocked and MCP-unreliable environments.
- Windows beta smoke root cause (`worktree_unenforced`) is addressable without
  requiring another editor restart loop.
- Clear layering: L0 MCP optional; L1/L2 daemon-backed save-time is the default
  product value.

### Negative

- More orchestration in `anvil start` (hooks, registration, state machine).
- Docs and tutorials must stop implying MCP alone equals "protected".

### Neutral

- ADR-044 MCP entry ownership unchanged; ACTMO-007 adds allow-list writes on
  Claude install where supported.
- DLIFE daemon lifecycle posture unchanged; ACTMO extends what happens after
  ensure succeeds.

## Alternatives Considered

| Alternative | Why rejected |
| ----------- | ------------ |
| Require MCP for `Protecting` | Fails corporate users; smoke shows handshake verified is insufficient |
| Require `anvil-run` wrapper for all agents | Not realistic for editor-native agents on Windows |
| Stall at `ready_restart_required` until MCP validates | Current broken UX; non-terminating for #2937 |
| Foreground daemon only | DLIFE-001/ADR-082 already rejected for daily use |

## References

- [ADR-015](015-intercept-loop-enforcement.md) — session registration model
- [ADR-044](044-mcp-entry-activation-owned.md) — MCP config ownership
- [ADR-061](061-save-time-daemon-delta-validation.md) — save-time validation
- [ADR-082](082-daemon-lifecycle-user-startup.md) — daemon ensure on start
- [daemon-lifecycle](../modules/daemon-lifecycle.aps.md) — DLIFE (merged)
- [activation-mcp-optional](../modules/activation-mcp-optional.aps.md) — ACTMO