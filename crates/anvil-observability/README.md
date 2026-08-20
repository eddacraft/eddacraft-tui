# anvil-observability

| Type   | Authority     | Owner | Status | Freshness                                                                                      |
| ------ | ------------- | ----- | ------ | ---------------------------------------------------------------------------------------------- |
| README | Authoritative | OBS   | Live   | Last reviewed 2026-08-20 against `crates/anvil-observability/src` and its tests at `f0f834b39` |

| Upstream                                                                            | Downstream                                       |
| ----------------------------------------------------------------------------------- | ------------------------------------------------ |
| `crates/anvil-observability/src`, ADR-035, and the observability namespace registry | anvil CLI and intercept-daemon tracing consumers |

Rust tracing primitives for anvil binaries. The crate owns subscriber
initialisation, JSON field/event redaction, local trace-file admission, and W3C
`traceparent` parsing/generation helpers. It does not make spans a durable
system of record and does not own exporter policy or the namespace catalogue.

## Entry points and flow

- `src/lib.rs::init_tracing` installs the process-global subscriber for either
  the CLI or intercept daemon.
- `src/redaction.rs` replaces exact case-insensitive sensitive field names with
  `<redacted>` in both span fields and event fields. Argument observations keep
  only a value-free shape and coarse length bucket.
- `src/traceparent.rs` parses, generates, and formats version-00 W3C
  `traceparent` values. Binding helpers record correlation fields on a tracing
  span; they do not install OpenTelemetry parent propagation.

Filter precedence is `ANVIL_LOG`, then `RUST_LOG`, then the binary default. The
CLI writes tracing JSON to stderr so command/stdout JSON stays valid; the daemon
uses its captured stdout path. `ANVIL_TRACE_SINK=file=<path>` selects a local
append-only file. On Unix, existing symlinks, non-regular files, wrong-owner
files, or group/world permission bits are rejected; a new file is created
owner-only. `otlp` is rejected because exporter wiring belongs to its own
authority.

## Invariants and failure behaviour

- Only binary entrypoints call `init_tracing`; libraries emit spans but never
  install the global subscriber.
- A second subscriber installation returns `AlreadyInstalled`.
- Sensitive-name matching is exact and case-insensitive; safe names such as
  `token_type` are not substring-redacted.
- Raw argument values are never stored by `ArgShape`; sensitive arguments also
  omit type and length shape.
- Trace output is diagnostic evidence, not durable governance truth. Kindling
  and the notification envelope own their respective durable/live facts.
- Unsupported sinks fail explicitly instead of silently changing destination.

## Local validation

```bash
cargo test -p eddacraft-anvil-observability
```

## Source references

- `crates/anvil-observability/src/lib.rs`
- `crates/anvil-observability/src/redaction.rs`
- `crates/anvil-observability/src/traceparent.rs`

## Related authorities

- [Observability namespace registry](../../docs/observability/namespace-registry.md)
- [Tracing operations](../../docs/runbooks/observability-triage.md)
- [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md)
- [Pre-migration historical snapshot](../../docs/architecture/observability-as-built.md)
