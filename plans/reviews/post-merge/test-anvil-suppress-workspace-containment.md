# Post-Merge: test/anvil-suppress-workspace-containment

Traces to: #1756 (test gap — workspace-root containment for mutating MCP tools).

This branch is a test-only change (adds stdio integration coverage in
`crates/anvil-cli/tests/mcp_serve_stdio.rs`). All correctness gates run
pre-merge; the only post-merge verification is confirming the new tests
execute and pass on `main` after the rebase-merge.

## After merge

- Confirm the merged-CI Rust test job on `main` includes the new cases and is
  green:
  ```bash
  cargo test -p eddacraft-anvil --test mcp_serve_stdio
  ```
  Expected signal: `test result: ok` with the two new mutating-tool
  containment cases present:
  `mcp_serve_stdio_tools_call_suppress_rejects_workspace_outside_server_root`
  and `mcp_serve_stdio_tools_call_fix_rejects_workspace_outside_server_root`.

Delete this file once the post-merge step above is confirmed.
