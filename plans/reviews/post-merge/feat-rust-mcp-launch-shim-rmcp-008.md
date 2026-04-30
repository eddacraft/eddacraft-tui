# Post-merge: feat-rust-mcp-launch-shim-rmcp-008

PR: #1154
Branch: `feat/rust-mcp-launch-shim-rmcp-008`
APS: RMCP-008
Merged: 2026-04-28 via PR #1154
Verified: complete — agent-runnable checks passed 2026-04-30; Claude Code GUI dry-run passed and was recorded 2026-04-30

## Steps

- [x] Run `cargo build -p eddacraft-anvil` (agent: yes) — passed 2026-04-30
- [x] Run `pnpm --filter @eddacraft/anvil-e2e test:smoke` (agent: yes) — passed 2026-04-30
- [x] Run the Cursor or Claude Code MCP dry-run from `plans/specs/2026-04-26-rtai-demo-runbook.md` and record the result in the RMCP Launch Validation Log (human required) — Claude Code dry-run passed 2026-04-30; see runbook §8

## Notes

RMCP-008 includes headless Rust MCP smoke coverage in this branch. The human GUI
dry-run is complete: Claude Code exercised `anvil_validate_write` against the
AI-001 reasoning rule and the result is recorded in the RMCP Launch Validation
Log. Release backend status is embedded-fallback-backed, not daemon-backed:
`DaemonValidationClient` returned `Unavailable`, so validation ran through the
embedded `anvil-checks` pipeline. Daemon-backed MCP client wiring is a post-A1
RMCP/RMCPF follow-up, not part of the RMCP-008 release gate.
