# Post-merge test plan — jsonrpc oversized notification liveness

Branch: `test/jsonrpc-oversized-notification-liveness`
Issue: #1749 (Clawpatch CLAWP-017)
Council verdict: pass-with-minors

## What / why

The `oversized_scan_buffer_notification_is_dropped_silently` conformance test in
`crates/anvil-intercept/tests/jsonrpc_conformance.rs` claimed in a comment to
verify the connection stays alive after an oversized JSON-RPC notification, but
it only dropped the recovered client without issuing a follow-up request. The
liveness contract was asserted by comment, not by code.

The test now recovers the client after the no-response timeout, writes a normal
id-bearing `session.list` request on the same stream, and asserts a well-formed
JSON-RPC result with the matching id (`liveness-1`) before shutdown. A
regression that wedges the read loop after an oversized notification surfaces as
a timeout on the follow-up request.

Tests-only change:

- `crates/anvil-intercept/tests/jsonrpc_conformance.rs` —
  `oversized_scan_buffer_notification_is_dropped_silently` now exercises
  post-notification liveness.

## Gate commands run (pre-PR, all green)

```
cargo fmt --all --check
cargo test -p anvil-intercept --test jsonrpc_conformance oversized_scan_buffer_notification_is_dropped_silently
cargo clippy -p anvil-intercept --all-targets -- -D warnings
```

Results: fmt exit 0 (clean); 1 test passed; clippy exit 0 (no warnings).

## Post-merge verification

After merge to `main`, confirm the test runs in the standard suite and stays
green:

```
cargo test -p anvil-intercept --test jsonrpc_conformance
```

Expect `oversized_scan_buffer_notification_is_dropped_silently` to pass. A
failure (timeout on the follow-up `session.list`) indicates the read-loop
liveness contract of #1749 has regressed.
