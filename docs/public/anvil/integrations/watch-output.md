---
id: watch-output
title: Watch JSON Output
description:
  Parse anvil --json watch as a versioned NDJSON stream from your own tools.
sidebar_position: 4
---

# Watch JSON Output

| Type        | Authority     | Owner                                                                                                                                              | Status | Freshness                                                                    |
| ----------- | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------- |
| Public docs | Authoritative | WOUT ([`plans/modules/watch-output-contract.aps.md`](https://github.com/eddacraft/anvil-001/blob/main/plans/modules/watch-output-contract.aps.md)) | Live   | Last reviewed 2026-06-08 against `main` for the v0.8.0-beta consumer surface |

| Upstream                                                                                                                                            | Downstream                                                                              |
| --------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| [`docs/specs/watch-output-contract.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/specs/watch-output-contract.md), `anvil --json watch` | Editor sidecars, CI shell pipelines, `jq` scripts, language-specific consumer libraries |

`anvil --json watch` emits a versioned newline-delimited JSON (NDJSON) stream on
stdout. You can pipe it into `jq`, a shell loop, or a small reader process
without coupling to terminal output.

`--json` is a global flag, so place it before the subcommand: use
`anvil --json watch`, not `anvil watch --json`.

The initial scan builds baseline/readiness state. It emits progress and snapshot
events, but existing repository contents are not reported as new violations
until a later file change introduces or re-surfaces them.

:::info Contract version

This page describes `anvil.watch.event.v1`. The contract is additive within v1 —
new optional payload fields and new `event_type` values may appear, but the
documented variants below stay stable. The full normative spec lives at
[`docs/specs/watch-output-contract.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/specs/watch-output-contract.md).

The stream is identical whether `anvil watch` runs its own scoped `check` per
save or routes save-time validation through the resident daemon (the default
when the daemon is live; `ANVIL_WATCH_DAEMON=0` opts out). The backing changes;
the event contract does not — so consumers never need to branch on it.

:::

## The Envelope

Every line on stdout deserialises to:

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 42,
  "timestamp": "2026-05-14T10:21:33Z",
  "event_type": "snapshot",
  "payload": { "node_count": 312, "edge_count": 845, "files_watched": 64 }
}
```

| Field            | Notes                                                                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `schema_version` | Pin to the `anvil.watch.event.v1` prefix. Refuse other values.                                                                              |
| `seq`            | Unique per process. Use to detect dropped or reordered lines.                                                                               |
| `timestamp`      | ISO 8601 UTC, second precision today (e.g. `2026-05-14T10:21:33Z`), may gain sub-second precision in additive releases. Always ends in `Z`. |
| `event_type`     | One of `progress`, `snapshot`, `violation`, `error`. New values may appear; treat unknowns as informational.                                |
| `payload`        | Always an object. Shape depends on `event_type`.                                                                                            |

## stdout vs stderr

| Channel    | Contains                                                                                                                                | What to do                                       |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| **stdout** | Only NDJSON event records, one per line.                                                                                                | Parse as your data stream.                       |
| **stderr** | Human-readable diagnostics: startup warnings, bare-exclude advice, watcher setup errors, child action stderr inherited from `--action`. | Capture as a log or ignore. Never parse as JSON. |

This means you can confidently do:

```bash
anvil --json watch > events.ndjson 2> watch.log
```

…and `events.ndjson` will be 100% parseable.

## Event Variants

### `progress`

Emitted during the initial scan and any rescan.

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 0,
  "timestamp": "2026-05-14T10:21:30Z",
  "event_type": "progress",
  "payload": { "phase": "initial-scan", "current": 12, "total": 100 }
}
```

### `snapshot`

Emitted after each debounced batch is processed. The first snapshot marks
"initial scan complete"; subsequent snapshots mark "graph re-converged after a
file change".

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 3,
  "timestamp": "2026-05-14T10:21:30Z",
  "event_type": "snapshot",
  "payload": { "node_count": 312, "edge_count": 845, "files_watched": 64 }
}
```

### `violation`

Emitted when a rule fires on a snapshot.

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

### `error`

Emitted on engine errors. `recoverable: true` means the watcher continues
running; `recoverable: false` means the stream will terminate shortly after.

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

## Consumer Examples

### Count snapshots with `jq`

```bash
anvil --json watch | jq -c 'select(.event_type=="snapshot") | .payload'
```

### Shell loop that reacts to violations

```bash
anvil --json watch | while IFS= read -r line; do
  type=$(printf '%s' "$line" | jq -r '.event_type')
  if [ "$type" = "violation" ]; then
    printf '%s\n' "$line" | jq -r '"\(.payload.policy_id): \(.payload.file) — \(.payload.message)"'
  fi
done
```

### A small long-running reader (Node.js)

```javascript
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

const child = spawn('anvil', ['--json', 'watch'], {
  stdio: ['ignore', 'pipe', 'inherit'],
});
const rl = createInterface({ input: child.stdout });

rl.on('line', (line) => {
  const event = JSON.parse(line);
  if (!event.schema_version?.startsWith('anvil.watch.event.v1')) return;
  if (event.event_type === 'violation') {
    console.log(
      `[${event.seq}] ${event.payload.policy_id} on ${event.payload.file}`
    );
  }
});

child.on('exit', (code) => {
  process.exit(code ?? 0);
});
```

### A Python reader

```python
import json
import subprocess

proc = subprocess.Popen(
    ["anvil", "--json", "watch"],
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,  # or capture and drain this in a separate thread
    text=True,
)

assert proc.stdout is not None
for line in proc.stdout:
    event = json.loads(line)
    if not event["schema_version"].startswith("anvil.watch.event.v1"):
        continue
    if event["event_type"] == "violation":
        p = event["payload"]
        print(f"{p['policy_id']}: {p['file']} — {p['message']}")
```

## Pinning and Forward Compatibility

A consumer pinned to v1 should:

1. Check `schema_version` starts with `anvil.watch.event.v1` and skip the line
   otherwise.
2. Treat unknown `event_type` values as informational — log them, but do not
   panic.
3. Ignore unknown optional fields in known payload variants.

A producer (Anvil itself) commits to:

1. Never removing or renaming a required field within v1.
2. Only adding optional fields and additional `event_type` values within v1.
3. Bumping the `schema_version` to `anvil.watch.event.v2` for any breaking
   change.

## Limits

- **No stdin control.** v1 is a one-way stream. There is no protocol on stdin;
  closing stdin is undefined.
- **No terminal event.** The stream ends on EOF, not on a synthetic `shutdown`
  record. Detect end-of-stream from EOF.
- **Back-pressure is reader-paced.** stdout is line-buffered; a slow reader
  blocks the writer. Drain promptly or buffer to disk.
- **Action child output.** With `--action`, the dispatched child's stdout is
  discarded in `--json` mode so it cannot interleave with the event stream.
  Child stderr is inherited and appears on the parent's stderr.
