# anvil kernel

| Type   | Authority     | Owner | Status | Freshness                                                                                         |
| ------ | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------- |
| README | Authoritative | KERN  | Live   | Last reviewed 2026-08-20 against `f0f834b39`, `src/lib.rs`, `src/watch.rs`, and `ARCHITECTURE.md` |

| Upstream                                                                              | Downstream                                             |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `crates/anvil-kernel/src/**`, `crates/anvil-graph-cache/src/**`, ADR-064, and ADR-123 | Kernel contributors, CLI surfaces, and event consumers |

The anvil kernel coordinates source watching, parsing, semantic graph updates,
policy evaluation, and protocol events. KERN owns this component. Graph storage
and resolution live behind the `anvil-graph-cache` boundary established by
ADR-064; this crate owns orchestration rather than a second graph
implementation.

## Entry points

- [`src/lib.rs`](src/lib.rs) exports the kernel modules and public types.
- [`src/watch.rs`](src/watch.rs) runs initial discovery and incremental watch
  processing.
- [`src/embedded.rs`](src/embedded.rs) exposes in-process operation.
- [`src/protocol/`](src/protocol) emits findings, parse errors, and snapshots to
  consumers.

## Local validation

```bash
cargo test -p eddacraft-anvil-kernel
cargo bench -p eddacraft-anvil-kernel
```

The test suite includes architecture-parity and dual-run coverage. Run the
benchmark only when changing performance-sensitive parsing or graph paths.

## Architecture and authorities

Read the source-linked [local architecture](ARCHITECTURE.md) before changing the
watch pipeline or its invariants. It is the live implementation-map authority;
the former central [kernel as-built](../../docs/architecture/kernel-as-built.md)
is a dated compatibility and history record. Wider placement and diagram rules
are owned by
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
and the
[documentation governance guide](../../docs/guides/documentation-governance.md).
