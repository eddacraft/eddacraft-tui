---
id: watch-output
title: Watch output reference
description:
  Interpret ready, file, finding, and recovery messages from save-time
  validation.
---

# Watch output reference

The watcher has four observable phases:

1. **Starting:** configuration and daemon routing are checked.
2. **Ready:** the watched file set is established.
3. **Event:** a supported saved file is analysed.
4. **Result:** findings or an explicit clean result are printed.

Use:

```text
anvil watch --no-tui
```

for plain logs, or:

```bash
anvil --json watch
```

for a machine-readable stream. `--json` is a global option, so it comes before
`watch`.

## Machine-readable event stream

JSON mode writes newline-delimited JSON (NDJSON) to standard output: each line
is one complete event object. Human-readable diagnostics go to standard error,
so a consumer can parse standard output without stripping log messages.

Every event contains:

| Field            | Meaning                                                                                |
| ---------------- | -------------------------------------------------------------------------------------- |
| `schema_version` | The event contract. Accept `anvil.watch.event.v1`; handle other values explicitly.     |
| `seq`            | A process-local sequence number that helps detect missing or reordered events.         |
| `timestamp`      | The UTC time at which anvil emitted the event.                                         |
| `event_type`     | The payload variant: `progress`, `snapshot`, `violation`, `error`, or `action_result`. |
| `payload`        | Data for that event type.                                                              |

Treat unknown event types as additive information rather than terminating a
long-running consumer.

### Initial scan progress

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 0,
  "timestamp": "2026-05-14T10:21:30Z",
  "event_type": "progress",
  "payload": { "phase": "initial-scan", "current": 12, "total": 100 }
}
```

### Graph snapshot

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 3,
  "timestamp": "2026-05-14T10:21:30Z",
  "event_type": "snapshot",
  "payload": { "node_count": 312, "edge_count": 845, "files_watched": 64 }
}
```

### Finding

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 7,
  "timestamp": "2026-05-14T10:21:31Z",
  "event_type": "violation",
  "payload": {
    "policy_id": "no-circular-deps",
    "file": "src/main.ts",
    "symbol": "App",
    "message": "Circular dependency detected"
  }
}
```

### Recoverable error

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 9,
  "timestamp": "2026-05-14T10:21:31Z",
  "event_type": "error",
  "payload": {
    "code": "ParseError",
    "file": "src/broken.ts",
    "message": "Unexpected token",
    "recoverable": true
  }
}
```

When `recoverable` is `true`, keep reading the stream. When it is `false`,
record the error and expect the watcher to stop.

### Action result and daemon scope

`action_result` is general enough for dispatched action outcomes, but its
current producer scope is daemon-supplied `check` verdicts only. Subprocess
`check` and `gate` outcomes are not re-enveloped. For a daemon result,
`daemon_verdict` exposes the evidence scope instead of hiding it:

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 10,
  "timestamp": "2026-08-05T10:21:31Z",
  "event_type": "action_result",
  "payload": {
    "action": "check",
    "exit_code": 0,
    "duration_ms": 17,
    "daemon_verdict": {
      "assurance_state": "stale",
      "assurance_reason": "cross-file-resolution-needed",
      "coverage": "partial",
      "check_families": ["antipattern"],
      "finding_count": 0,
      "diagnostics": []
    }
  }
}
```

The daemon verdict's `diagnostics` are canonical `anvil.diagnostic.v1` objects.
`check_families` is the exact evaluated scope; the live daemon route currently
reports `["antipattern"]`. It does not attest secret detection or another family
that is absent from the array.

Do not treat `exit_code: 0` or `finding_count: 0` as a pass when
`assurance_state` is not `clean` or `coverage` is `partial`. Surface the state
and reason as degraded evidence.

Live-daemon family routing is deterministic once selected. Debounce,
reconnect/full-scan warm-up, and restored-window races remain unbounded, so
route selection and delivery can still vary. Use `--no-daemon` to force the
scoped fallback.

## Do not infer more than the output proves

- A ready watcher proves save-time coverage, not pre-write protection.
- A clean file result covers that analysed input and its named check families,
  not the entire repository.
- A daemon status proves the service is reachable, not that an AI client is
  connected.
- A skipped file should name the coverage reason.

Use `anvil start --verify` for the end-to-end protection state.

## Next step

See [activation states](../guides/start-output-contracts.md).
