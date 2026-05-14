# Post-merge: feat-rmcpf-011-012-tools-and-prompts

PR: #TBD
Branch: `feat/rmcpf-011-012-tools-and-prompts`
APS: RMCPF-011, RMCPF-012
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm `cargo test -p eddacraft-anvil --bin anvil mcp::tools` is green on `main` after merge (agent: yes)
- [ ] Confirm `cargo test -p eddacraft-anvil --test mcp_serve_stdio` is green on `main` after merge (agent: yes)
- [ ] Verify the Rust MCP `tools/list` exposes seven tools (`anvil_validate_write`, `anvil_status`, `anvil_check`, `anvil_gate`, `anvil_query_boundary`, `anvil_suppress`, `anvil_fix`) — covered by `mcp_serve_stdio_tools_list_returns_registered_tools` (agent: yes)
- [ ] Verify the Rust MCP `initialize` response omits the `prompts` capability and `prompts/list` returns JSON-RPC `-32601 Method not found` — covered by `mcp_serve_stdio_initialize_does_not_advertise_prompts_capability` and `mcp_serve_stdio_prompts_list_returns_method_not_found` (agent: yes)
- [ ] Advance RMCPF-011 from `In Progress` to `Merged` and update progress count in `plans/index.aps.md` once the PR merges (agent: yes)
- [ ] Advance RMCPF-012 from `In Progress` to `Merged` and update progress count in `plans/index.aps.md` once the PR merges (agent: yes)
- [ ] Confirm `archive/anvil-mcp-server/src/tools/` and `archive/anvil-mcp-server/src/prompts/` remain frozen reference material — no in-flight TS edits introduced (agent: yes)
- [ ] When INTD lands `suppression.apply`, file a follow-up to flip `anvil_suppress` from embedded fallback to daemon-RPC translator and pin the wire-shape parity (human required)

## Notes

The slice keeps `anvil_suppress` on the embedded-fallback path because no
INTD-owned `suppression.apply` exists yet, mirroring how RMCPF-010 shipped
`anvil_check`. The `backend: "embedded"` / `daemonStatus: "not-wired"`
correlation fields are the contract clients use to detect the daemon flip
when it lands — do not remove them.

RMCPF-012 retires all four archived prompts. The decision lives in
`plans/specs/rust-mcp-full-port-inventory.md` §"Prompts — RMCPF-012
disposition" and is pinned by two integration tests. Re-opening the
decision requires a supported-client demand artifact and an ADR update,
not a silent re-add.

The Council adversarial reviewer caught two MAJOR issues in this branch
before commit and they are now pinned by tests:

- `anvil_suppress` rejects a `warningId` containing `\r`, `\n`, control
  characters, spaces, colons, or backticks so a caller cannot inject a
  second source-code line into the suppression comment. Pinned by
  `rejects_warning_id_with_newline_to_block_comment_injection` and
  `rejects_warning_id_with_control_characters`.
- `anvil_fix` AP-001 fails closed on inline `code(); /* eslint-disable */`
  rather than rewriting it to a line-comment directive that would leak
  the suppression to unrelated code. Pinned by
  `ap_001_does_not_rewrite_inline_block_comment_after_code`.
