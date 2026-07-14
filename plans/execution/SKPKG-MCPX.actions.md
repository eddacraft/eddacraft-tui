# SKPKG and MCPX first-wave execution actions

## Objective

Ship the beta Anvil skill bundle and the first supported MCP client wave through
one typed client registry, with global installation as the interactive default
and an explicit project-scope choice.

## Actions

1. Add registry contract tests for stable client IDs, detection strength,
   independent skill/MCP capabilities, supported scopes, and reload guidance.
2. Add failing MCP config tests for every first-wave file shape and path; then
   implement semantic merge, atomic writes, idempotence, and verification.
3. Add failing skill installer tests for preview, global/project placement,
   manifest provenance, repeat installation, and refusal of unmanaged or
   modified files; then embed and install the pinned bundle.
4. Route `start`, `skill install`, `mcp install`, and `mcp-config` through the
   shared registry without weakening existing Claude Code or Cursor behaviour.
5. Update customer and architecture documentation, then reconcile SKPKG/MCPX
   feature state from the verified implementation evidence.
6. Run targeted tests, formatting, Clippy, documentation and APS validation;
   obtain Council review and fresh independent verification before publishing.

## Validation

```sh
CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/skpkg-mcpx cargo test -p eddacraft-anvil --test mcp_config
CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/skpkg-mcpx cargo test -p eddacraft-anvil --test skill_install
CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/skpkg-mcpx cargo test -p eddacraft-anvil agent_registry
CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/skpkg-mcpx cargo fmt --all -- --check
CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/skpkg-mcpx cargo clippy -p eddacraft-anvil --all-targets -- -D warnings
pnpm docs:check
pnpm aps:active-lint
pnpm aps:index:check
```

Use the worktree's normal cache name in place of `skpkg-mcpx` when rerunning
these commands from another branch.

## Closeout evidence

- Managed skill-install fixtures cover preview, global/project placement,
  provenance, repeat installation, and refusal of unmanaged, modified, or
  symlinked destinations.
- MCP fixtures cover the promoted JSON/TOML shapes, scope resolution,
  preservation of unrelated settings, foreign-entry refusal, atomic writes,
  verification, and Copilot CLI tool enablement.
- Activation fixtures cover explicit and detected clients, scope constraints,
  opt-out, all-client opt-in, and unchanged TUI consent authority.
- Council findings were remediated before independent verification and PR
  publication.
