# Watch JSON Fixtures (WOUT-005)

Golden NDJSON fixtures pinning the `anvil.watch.event.v1` wire envelope.

Each `*.jsonl` file contains one canonical event line. The drift-guard test in
[`crates/anvil-cli/tests/watch_json_output.rs`](../../watch_json_output.rs)
asserts:

- every fixture parses as a `WatchEventEnvelope`,
- every required field documented in
  [`docs/specs/watch-output-contract.md`](../../../../../docs/specs/watch-output-contract.md)
  is present with the expected primitive type,
- the spec's documented examples in
  [`docs/public/anvil/integrations/watch-output.md`](../../../../../docs/public/anvil/integrations/watch-output.md)
  match these fixtures byte-for-byte.

Add a new fixture when:

1. Introducing a new `event_type` value (still within `v1` — additive).
2. Documenting a new optional payload field shape.

Never edit an existing fixture in a way that removes or renames a required field
within v1. That is a `v2` breaking change.
