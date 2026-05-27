# Local Tracing

| Type  | Authority | Owner | Status | Freshness                                        |
| ----- | --------- | ----- | ------ | ------------------------------------------------ |
| Guide | Derived   | TRACE | Live   | Metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                                                | Downstream                     |
| ------------------------------------------------------- | ------------------------------ |
| TRACE-004, `crates/anvil-observability/`, intercept CLI | Local daemon and CLI debugging |

TRACE-004 adds local span correlation for daemon and CLI debugging without
selecting a production exporter.

## Defaults

By default, `init_tracing(BinaryKind)` keeps the TRACE-001 JSON formatter and
writes to stderr. Configure verbosity with `ANVIL_LOG` or `RUST_LOG`.

## File Sink

Use `ANVIL_TRACE_SINK=file=<path>` to write JSON-line tracing output to a
user-private local file. On Unix, newly created files are opened with `0600`
permissions; existing sinks are rejected if they are symlinks or are readable /
writable by group or other users. Prefer a directory owned by your user rather
than shared locations such as `/tmp`:

```bash
mkdir -p "$HOME/.local/state/anvil"
ANVIL_LOG=info ANVIL_TRACE_SINK=file="$HOME/.local/state/anvil/trace.jsonl" cargo run -p eddacraft-anvil-intercept
```

In another shell, send a JSON-RPC request with a valid `traceparent`. The daemon
records the incoming `trace_id`, `parent_id`, and `trace_flags` as correlation
fields on the dispatch span and echoes the original header on the response. This
is local correlation, not full OpenTelemetry parent propagation.

## OTLP

`ANVIL_TRACE_SINK=otlp` is intentionally not wired in TRACE-004. Production and
collector-backed exporter choice remains deferred to the EXPORT module so this
first cut does not add OpenTelemetry SDK dependency churn.
