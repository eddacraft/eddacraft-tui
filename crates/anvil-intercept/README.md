# anvil intercept

| Type   | Authority     | Owner | Status | Freshness                                                                                             |
| ------ | ------------- | ----- | ------ | ----------------------------------------------------------------------------------------------------- |
| README | Authoritative | INTD  | Live   | Last reviewed 2026-08-20 against `f0f834b39`, `src/lib.rs`, `src/save_time.rs`, and `ARCHITECTURE.md` |

| Upstream                                                       | Downstream                           |
| -------------------------------------------------------------- | ------------------------------------ |
| `crates/anvil-intercept/src/**`, ADR-085, ADR-090, and ADR-123 | Interception clients and maintainers |

The interception crate provides INTD-owned save-time validation, authenticated
daemon request handling, workspace admission, and persistent fencing for unsafe
or degraded worktrees. It does not own editor integration or policy meaning;
callers and the policy engine remain separate authorities.

## Entry points

- [`src/lib.rs`](src/lib.rs) exports the interception surfaces.
- [`src/ipc.rs`](src/ipc.rs) handles daemon JSON-RPC requests.
- [`src/save_time.rs`](src/save_time.rs) coordinates save-time checks.
- [`src/validate_paths.rs`](src/validate_paths.rs) validates proposed paths.
- [`src/fence.rs`](src/fence.rs) persists and queries worktree fences.
- [`src/auth.rs`](src/auth.rs) and
  [`src/workspace_admission.rs`](src/workspace_admission.rs) enforce caller and
  workspace boundaries.

## Local validation

```bash
cargo test -p eddacraft-anvil-intercept
```

## Architecture and authorities

Read the source-linked [local architecture](ARCHITECTURE.md) for the validation
and fence boundaries. It is the live implementation-map authority; the former
central [intercept as-built](../../docs/architecture/intercept-as-built.md) is a
dated compatibility and history record. The
[driver framework as-built](../../docs/architecture/driver-framework-as-built.md)
owns the wider integration view.
