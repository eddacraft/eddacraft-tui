# anvil-intercept-proto

| Type   | Authority     | Owner     | Status | Freshness                                                                                        |
| ------ | ------------- | --------- | ------ | ------------------------------------------------------------------------------------------------ |
| README | Authoritative | INTD/DRVR | Live   | Last reviewed 2026-08-20 against `crates/anvil-intercept-proto/src` and its tests at `f0f834b39` |

| Upstream                                                                 | Downstream                                                           |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------- |
| `crates/anvil-intercept-proto/src`, ADR-015, ADR-030, and protocol specs | intercept daemon, CLI/MCP consumers, and TypeScript protocol mirrors |

Rust wire-contract crate shared by the intercept daemon and its clients. It owns
serialised identifiers, JSON-RPC method names and payloads, session/status
records, and the permissive `.anvil.yaml` enforcement/telemetry input shapes. It
does not own daemon admission or enforcement policy.

## Modules

- `src/protocol.rs` owns the `anvil/` JSON-RPC namespace, capability vocabulary,
  manifest slice, request/result payloads, and method catalogue.
- `src/session.rs` owns session identifiers, agent tags, session lifecycle, and
  the NDJSON command/envelope shapes.
- `src/status.rs` owns daemon, worktree, fence, health, latency, cache,
  subscriber, and save-time-driver status snapshots.
- `src/enforcement_config.rs` owns forwards-compatible configuration input
  shapes. Consumers apply defaults, merging, clamps, and policy themselves.

## Contract invariants

- Rust is authoritative over the TypeScript mirrors in
  `packages/anvil-driver-client/src/protocol`, `session`, `diagnostics`, and
  `protection_claim`.
- Wire additions use additive/defaulted fields or explicit catch-all variants
  where the consumer must survive a newer producer.
- The protocol layer preserves the full `Off`, `Warn`, `Fence`, and `Interrupt`
  vocabulary but does not decide the effective posture.
- Session identifiers are opaque at this layer; the daemon registry owns
  validation and uniqueness.
- Unknown top-level configuration keys are ignored so unrelated consumer
  settings do not wedge deserialisation.
- `DaemonStatusV1.generated_at_unix == 0` means no trustworthy snapshot anchor;
  consumers fall back to the documented per-session evidence path.
- Unknown save-time driver states fail safe as absent coverage.

## Local validation

```bash
cargo test -p eddacraft-anvil-intercept-proto
```

Protocol changes also require the TypeScript mirror tests:

```bash
pnpm --filter @eddacraft/anvil-driver-client test
pnpm --filter @eddacraft/anvil-driver-client typecheck
```

## Related authorities

- [Cross-system driver framework](../../docs/architecture/driver-framework-as-built.md)
- [TypeScript driver-client architecture](../../packages/anvil-driver-client/ARCHITECTURE.md)
- [Intercept component architecture](../anvil-intercept/ARCHITECTURE.md)
- [ADR-015](../../plans/decisions/015-intercept-loop-enforcement.md)
- [ADR-030](../../plans/decisions/030-surface-drivers-supersede-napi-cutover.md)
