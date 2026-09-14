# CIB-101 Mini Council — ANVIL_HOME Uninstall Cleanup

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

CIB-101 originally combined two DISTRIB-006 follow-ups:

1. Windows named-pipe re-root under `ANVIL_HOME`.
2. `anvil uninstall --global` cleanup for the active `ANVIL_HOME` user root.

Mini Council split the item before implementation. The Windows pipe work is now
tracked separately as CIB-106 because it overlaps CIB-100 and needs Windows
matrix validation. This receipt covers the local uninstall cleanup slice only.

Changed surfaces:

- `crates/anvil-cli/src/commands/uninstall.rs`
- `crates/anvil-cli/tests/anvil_home.rs`
- `plans/modules/continuous-improvement-backlog.aps.md`
- `plans/index.aps.md`

## Council constraints applied

- Under `ANVIL_HOME`, global uninstall must remove `<ANVIL_HOME>/user/`, not
  production `~/.anvil/` or default credential paths.
- Default no-`ANVIL_HOME` global uninstall behaviour must remain unchanged.
- Dry-run JSON must report the same scoped path execution would remove.
- `--keep-daemon` and `--keep-mcp` must not suppress user-state cleanup.
- The scoped `<ANVIL_HOME>/user/` delete path must refuse symlinked leaves and a
  symlinked `ANVIL_HOME` prefix on Unix.
- Uninstall daemon stop must not keep bespoke PID-signalling logic; it now uses
  the safer `anvil_intercept::request_daemon_stop()` path on Unix and the
  existing symlink-hardened file removal path for non-Unix stale PID cleanup.

## Implemented behaviour

- `build_plan_with_install_user_dir` lets production code inject the active
  install-root user dir while keeping unit tests pure.
- `run()` resolves `crate::install_root::install_root().user_dir()` and passes it
  into the uninstall planner.
- When an install-root override is active, the global uninstall plan queues
  `RemoveUserAnvil { path: <ANVIL_HOME>/user, install_root_scoped: true }` and
  skips default credential candidates.
- When no install-root override is active, `--global` keeps existing behaviour:
  `~/.anvil/` plus default credential candidates.
- Install-root-scoped user cleanup refuses a symlinked prefix before recursive
  deletion, and `remove_directory` still refuses a symlinked leaf.

## Validation evidence

Focused Rust validation passed:

```text
cargo fmt --check
cargo test -p eddacraft-anvil commands::uninstall
cargo test -p eddacraft-anvil --test anvil_home
cargo clippy -p eddacraft-anvil --all-targets -- -D warnings
```

Observed results:

- `commands::uninstall`: 22 passed.
- `anvil_home` integration suite: 13 passed.
- `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`: passed.

The `anvil_home` integration tests prove:

- `uninstall --global --yes --keep-daemon --keep-mcp` with `ANVIL_HOME=<tmp>`
  removes `<tmp>/user/`.
- production `HOME/.anvil/` is preserved under that override.
- `--dry-run --json` reports the scoped `<ANVIL_HOME>/user/` action and deletes
  nothing.

## Residual / split work

CIB-106 owns Windows named-pipe endpoint re-rooting under `ANVIL_HOME`, including
central resolver adoption across ensure/status/MCP/watch/GCTX clients and Windows
matrix validation.
