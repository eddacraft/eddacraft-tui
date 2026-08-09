# Agent-facing validate_write ergonomics

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for RMCPF Phase 4 validate_write ergonomics | MCPX / DRVR / agent-ready reliability | Accepted | 2026-08-09 — Design approved; implementation via RMCPF-040..044 |

| Upstream | Downstream |
| -------- | ---------- |
| Field evidence (agent harnesses dumping full `proposedContent` + full `anvil.mcp.validate-write.v1` envelopes on clean allow), [mcp-shim-as-built](../../docs/architecture/mcp-shim-as-built.md) §4, MLP2-051b `protection_claim`, CIB-005 patch mode, [ADR-083](../decisions/083-gctx-mcp-delivery-target.md), [MCP live-heal](./2026-08-09-mcp-live-heal-without-harness-restart.md) (availability; orthogonal), `anvil-developer-functions` skill | [RMCPF-040..044](../modules/rust-mcp-full-port.aps.md); optional ADR if default response shape becomes a durable public contract |

**Design approved 2026-08-09** (amended same day: Layer B is
**harness-agnostic**, not a vendor-specific UI project). Implementation is
authorised only by Ready/In Progress APS work items RMCPF-040..044.

## 1. Problem

`anvil_validate_write` is load-bearing for agent edit loops. The loop is
correct; **cost** is not:

1. **Wire cost (model context).** Full `proposedContent` plus a full response
   envelope on clean allow (empty diagnostics, zeroed summary, correlation,
   protection_claim, tier).
2. **Display cost (human scrollback).** Harnesses dump full tool args/results
   by default — a cross-harness pattern.

| Layer | Owner | Saves |
| ----- | ----- | ----- |
| **A. Wire lean** | anvil MCP — **primary product work** | Model tokens for every client |
| **B. Display lean** | Each harness; anvil publishes **agnostic guidance** only | Human scrollback |

## 2. Product principle

> Minimal wire for the common allow path; full wire when the agent must act;
> collapsed UI by default in clients that support it; verbose only when the
> human opts in. Validation quality is unchanged.

## 3. Layer A — wire lean (anvil ships)

### Request ranking

1. `anvil_apply_patch` + unifiedDiff (added lines)
2. `anvil_validate_write` + patch only (full post-image, CIB-005)
3. Full `proposedContent` (creates / no patch)
4. `preview` + `contentSha256` — **partial** only

### Response (decision-gated)

- Clean **allow** + `detail: minimal`: `{ "schema", "decision": "allow" }`
- **warn** / **veto** / errors: actionable full payload
- Request `detail: "minimal" | "full"`; env `ANVIL_MCP_VALIDATE_DETAIL`; request
  wins. **A1 default remains full**; **A4 flips default to minimal**.

### Delivery

| Item | Slice | Status |
| ---- | ----- | ------ |
| RMCPF-040 | A1 detail + minimal builder | In Progress |
| RMCPF-041 | A2 skill / tool copy | Ready |
| RMCPF-042 | A3 apply_patch parity | Ready |
| RMCPF-043 | A4 flip default to minimal | Ready |
| RMCPF-044 | B1 harness-agnostic display docs | Ready |

## 4. Layer B — display lean (guidance only)

Portable one-line summary: tool · path · decision · optional finding hint.
Never fold non-allow into allow-only groups. Display ≠ context. **No**
harness-specific UI code under this design.

## 5. Non-goals

- Weakening scan quality for full/post-image modes
- Vendor UI patches (Grok, Claude, Cursor, …)
- Blocking wire lean on any client UI
- Live-heal / graph warm (separate designs)
- Install-path overhead (none expected)

## 6. Decision log

| ID | Decision |
| -- | -------- |
| D1 | Two layers: wire + display |
| D2 | Decision-gated response; keep `validate-write.v1` |
| D3 | Prefer apply_patch / patch over full content |
| D4 | Preview+hash is partial, not default |
| D5 | Opt-in full envelope for claim/telemetry clients |
| D6 | Never group-fold non-allow into allow-only summaries |
| D7 | APS items authorise code; design alone does not |
| D8 | Layer B is harness-agnostic guidance only |
