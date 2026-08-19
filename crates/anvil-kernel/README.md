# anvil kernel

| Type   | Authority     | Owner | Status | Freshness                                                                                                              |
| ------ | ------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------- |
| README | Authoritative | KERN  | Live   | Last reviewed 2026-08-20 against `d6c8b565c`, `crates/anvil-kernel/src/lib.rs`, and `crates/anvil-kernel/src/watch.rs` |

| Upstream                                                                                   | Downstream                                                      |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------------- |
| `crates/anvil-kernel/src/**`, ADR-064, ADR-123, and `docs/architecture/kernel-as-built.md` | Kernel contributors, `crates/anvil-kernel/ARCHITECTURE.md`, CLI |

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
watch pipeline or its invariants. The retained central
[kernel as-built](../../docs/architecture/kernel-as-built.md) remains the
pre-migration implementation map until DOCRB-005; this pilot does not supersede
it. Wider placement and diagram rules are owned by
[ADR-123](../../plans/decisions/123-documentation-authority-and-diagram-model.md)
and the
[documentation governance guide](../../docs/guides/documentation-governance.md).
