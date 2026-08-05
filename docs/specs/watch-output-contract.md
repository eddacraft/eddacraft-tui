# Watch Output Contract — `anvil.watch.event.v1`

| Type | Authority     | Owner                                                                                                                   | Status | Freshness                                                                                           |
| ---- | ------------- | ----------------------------------------------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------- |
| Spec | Authoritative | WOUT ([`plans/archive/modules/watch-output-contract.aps.md`](../../plans/archive/modules/watch-output-contract.aps.md)) | Live   | Last reviewed 2026-08-05 against CIB-254 machine-output activation and the daemon save-time verdict |

| Upstream                                                                                                                                          | Downstream                                                                                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-kernel-types/src/watch_event.rs` (`WatchEventEnvelope`), `crates/anvil-kernel-types/src/events.rs` (`EngineEvent` in-process shape) | `crates/anvil-cli/src/commands/watch.rs` (serialisation site), [`docs/public/anvil/integrations/watch-output.md`](../public/anvil/integrations/watch-output.md), `crates/anvil-cli/tests/watch_json_output.rs` |

**Version:** 1.1.0 **Status:** Live **Created:** 2026-05-14 **Last Updated:**
2026-08-05

---

## Purpose

`anvil --json watch` already emits one JSON object per line on stdout, but the
shape was implementation-defined: `detail` was a debug-formatted Rust string,
warnings landed on stdout alongside event lines, and there was no documented
versioning. Downstream consumers (shell pipelines, CI scripts, editor sidecars)
had no contract to rely on.

This spec pins the NDJSON stream produced by `anvil --json watch` as
`anvil.watch.event.v1` — a durable, versioned surface that consumers can parse
without depending on Rust debug output.

It complements:

- `crates/anvil-kernel-types/src/events.rs` — the in-process Rust `EngineEvent`
  shape consumed by the TUI surface and other in-tree readers.
- `plans/specs/2026-04-26-diagnostic-envelope-coordination.md` — the
  `anvil.diagnostic.v1` shape used by gate and intercept envelopes. WOUT reuses
  the _style_ (snake_case fields, `schema_version` string, additive evolution
  rule) but defines its own outer envelope because watch is a streaming surface,
  not a single-shot return value.

It is intentionally orthogonal to:

- Daemon notification fan-out (`anvil.notification.v1`).
- The MCP / intercept protocols.

## Principles

1. **NDJSON, one object per line.** Stdout in `--json` mode contains nothing
   else — no banners, no human guidance, no child process output.
2. **stderr is the diagnostic channel.** Warnings, fallback messages, child
   stderr inherited from `--action`, and watcher setup errors land on stderr
   where consumers either ignore or log them.
3. **Versioned, additive evolution.** A consumer pinned to
   `anvil.watch.event.v1` continues to parse a producer that adds new optional
   payload fields or new `event_type` values it does not recognise.
4. **No stdin control protocol.** v1 is one-way: kernel → consumer. A future v2
   may add stdin commands; this spec explicitly leaves stdin reserved.
5. **EOF terminates the stream.** v1 does not emit a `shutdown` event on Ctrl-C;
   consumers detect end-of-stream from EOF on stdout.

## Envelope: `anvil.watch.event.v1`

Every line on stdout in `--json watch` mode deserialises to:

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 42,
  "timestamp": "2026-05-14T10:21:33Z",
  "event_type": "snapshot",
  "payload": { ... }
}
```

| Field            | Type          | Required | Purpose                                                                                                                                                                                                                                                                                                                                        |
| ---------------- | ------------- | :------: | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `schema_version` | string        |   yes    | Outer envelope version. Current: `anvil.watch.event.v1`. Bumps only on breaking changes.                                                                                                                                                                                                                                                       |
| `seq`            | integer (u64) |   yes    | Unique per-process sequence. Starts at 0 on process start. Producers may emit events from multiple worker threads, so a consumer MAY observe two events with consecutive seqs in either order; what is guaranteed is uniqueness within a process and strictly-increasing minted values. Consumers use it to detect dropped or reordered lines. |
| `timestamp`      | string        |   yes    | ISO 8601 UTC timestamp ending in `Z`. Second precision in v1 (e.g. `2026-05-14T10:21:33Z`). Producers MAY emit sub-second precision in additive releases; consumers MUST accept both forms.                                                                                                                                                    |
| `event_type`     | string        |   yes    | Discriminator. v1 known values: `progress`, `snapshot`, `violation`, `error`, `action_result`.                                                                                                                                                                                                                                                 |
| `payload`        | object        |   yes    | Event-specific payload. Shape depends on `event_type`. Unknown `event_type` values MUST have an object payload so v1 consumers can skip them without parse errors.                                                                                                                                                                             |

### Compatibility rule

A consumer pinned to v1 MUST:

- Reject the line when `schema_version` does not start with
  `anvil.watch.event.v1`.
- Tolerate unknown `event_type` strings by surfacing them as informational and
  continuing to read.
- Tolerate unknown optional payload fields by ignoring them.

A producer MUST:

- Keep `schema_version` exactly `anvil.watch.event.v1` for every additive
  release within v1.
- Add new optional fields only; never remove or rename a required field within
  v1.
- Reserve the `schema_version` bump (`anvil.watch.event.v2`) for breaking
  changes — renames, removals, semantic shifts, required-field additions.

## Event Variants

### `progress`

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 0,
  "timestamp": "2026-05-14T10:21:30Z",
  "event_type": "progress",
  "payload": {
    "phase": "initial-scan",
    "current": 12,
    "total": 100
  }
}
```

| Field     | Type    | Required | Notes                                                           |
| --------- | ------- | :------: | --------------------------------------------------------------- |
| `phase`   | string  |   yes    | Producer-defined phase tag (e.g. `initial-scan`, `re-resolve`). |
| `current` | integer |   yes    | Progress counter, 0-based.                                      |
| `total`   | integer |   yes    | Total expected. `0` is permitted for indeterminate work.        |

### `snapshot`

```json
{
  "schema_version": "anvil.watch.event.v1",
  "seq": 3,
  "timestamp": "2026-05-14T10:21:30Z",
  "event_type": "snapshot",
  "payload": {
    "node_count": 312,
    "edge_count": 845,
    "files_watched": 64
  }
}
```

| Field           | Type    | Required | Notes                                   |
| --------------- | ------- | :------: | --------------------------------------- |
| `node_count`    | integer |   yes    | Symbols currently in the graph.         |
| `edge_count`    | integer |   yes    | Edges currently in the graph.           |
| `files_watched` | integer |   yes    | Files currently tracked by the watcher. |

### `violation`

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

| Field       | Type   | Required | Notes                                                        |
| ----------- | ------ | :------: | ------------------------------------------------------------ |
| `policy_id` | string |   yes    | Stable rule ID.                                              |
| `file`      | string |   yes    | Workspace-relative path. Forward slashes on every platform.  |
| `symbol`    | string |   yes    | Symbol name in violation. May be empty for file-scope rules. |
| `message`   | string |   yes    | Human-readable detail.                                       |

### `error`

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

| Field         | Type         | Required | Notes                                                                                                                                                                                                                                                                 |
| ------------- | ------------ | :------: | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `code`        | string       |   yes    | One of `ParseError`, `ConfigError`, `Internal` — **PascalCase, pinned**. New codes require a spec amendment. The casing is deliberately _not_ snake_case (unlike `event_type`); the values are the Rust variant names and renaming them is a v2-only breaking change. |
| `file`        | string\|null | optional | Workspace-relative path when the error has a file anchor. Omitted or `null` for engine-wide errors.                                                                                                                                                                   |
| `message`     | string       |   yes    | Human-readable detail.                                                                                                                                                                                                                                                |
| `recoverable` | boolean      |   yes    | `true` if the watcher continues after this error; `false` if the watcher will exit.                                                                                                                                                                                   |

### `action_result`

`action_result` is a general additive shape for the observable outcome of a
dispatched watch action. Its initial CIB-254 producer scope is narrower: watch
emits it only for daemon-supplied `check` verdicts. Subprocess `check` and
`gate` outcomes are not re-enveloped in this slice. The required `action` field
is structurally unique among WOUT-v1 payload variants.

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

| Field            | Type          | Required | Notes                                                                                                                                                           |
| ---------------- | ------------- | :------: | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `action`         | string        |   yes    | The dispatched action name. The current producer emits `check`; future additive producers may use values such as `gate`. This is the structurally unique field. |
| `exit_code`      | integer\|null | optional | The child or action exit code. Omitted when the action did not exit normally, including cancellation or wait failure.                                           |
| `duration_ms`    | integer (u64) |   yes    | Wall-clock action duration in milliseconds.                                                                                                                     |
| `error_detail`   | string\|null  | optional | Cause-specific detail when the action could not report a normal exit.                                                                                           |
| `daemon_verdict` | object\|null  | optional | Structured save-time daemon evidence when the daemon path supplied the action result.                                                                           |

A present `daemon_verdict` contains:

| Field              | Type             | Required | Notes                                                                                                                                         |
| ------------------ | ---------------- | :------: | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `assurance_state`  | string           |   yes    | Daemon assurance state, currently `clean`, `stale`, `pending`, `running`, `bounded`, `unavailable`, or `unknown`.                             |
| `assurance_reason` | string\|null     | optional | Kebab-case reason when the assurance state has a reason; omitted for states without one.                                                      |
| `coverage`         | string           |   yes    | What the verdict attests: currently `certified` or `partial`.                                                                                 |
| `check_families`   | array of strings |   yes    | Exact families evaluated. The live daemon path currently reports `["antipattern"]`; consumers MUST NOT widen that claim.                      |
| `finding_count`    | integer (u64)    |   yes    | Number of canonical diagnostics in this verdict. Producers MUST keep it equal to `diagnostics.length`.                                        |
| `diagnostics`      | array of objects |   yes    | Canonical `anvil.diagnostic.v1` entries, including schema version, severity, location, category, source, optional remediation hint, and mode. |

`exit_code: 0` and `finding_count: 0` do not turn degraded daemon evidence into
a pass. Consumers MUST treat `coverage: partial`, or a non-clean assurance state
such as `stale`, as degraded and surface its state/reason. A certified daemon
verdict is scoped only to its exact `check_families`.

Live-daemon family routing is deterministic once that route is selected.
Selection and delivery are not fully deterministic: debounce coalescing,
reconnect/full-scan warm-up, and restored-window timing races remain unbounded.
`--no-daemon` forces the scoped fallback path.

## Stdout / Stderr Ownership

| Channel | Purpose                                                                                                     | v1 Guarantee                                                                                                                               |
| ------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| stdout  | NDJSON event records, one per line, ending with a single `\n`                                               | Every byte on stdout in `--json` mode parses as a v1 envelope line. No banners, no progress text, no child process output.                 |
| stderr  | Human-readable diagnostics, warnings, fallback notices, child process stderr inherited from `--action` runs | May contain prefixed labels such as `[warn] …`, `[watching] …`. Not part of the contract; consumers may ignore or capture as a log stream. |

Rules that producers MUST follow:

1. The startup `[watching] …` and `[excluding] …` banners are suppressed in
   `--json` mode (they were never on the stdout path before WOUT either, but
   this is now a guarantee, not a side effect).
2. Bare-exclude pattern warnings (`warn_on_bare_exclude_patterns`) MUST route to
   stderr when `--json` is set.
3. Watcher setup errors (`bail!(...)` paths in `commands/watch.rs`) MUST surface
   to stderr — they are not encoded as v1 events because they prevent the stream
   from starting.
4. Action child stdout in `--json` mode MUST be discarded (`Stdio::null()`) so
   the child cannot interleave bytes with parent NDJSON. Action child stderr MAY
   be inherited; consumers that want clean stderr capture should run the child
   through `--no-tui` and route stderr themselves.
5. Ctrl-C / shutdown closes stdout via EOF. v1 does NOT emit a terminal
   `shutdown` event; consumers detect end-of-stream from EOF.

## Versioning and Non-Goals

### What v1 promises

- Field names and types stable for every event documented above.
- Additive evolution: new optional payload fields, new `event_type` values
  surfaced with object payloads.
- Stable stdout/stderr ownership rules.
- `event_type` is the authoritative discriminator. Consumers MUST dispatch on
  `event_type` rather than guessing the payload shape from required field names.
  Producers MUST ship a payload whose required fields match the variant named in
  `event_type`.

### Adding a new event variant (forward-compat rule)

Any new payload variant added within v1 — Rust-side or otherwise — MUST
introduce at least one required field name that does not appear in any other
variant. The current variants are distinguishable by `phase`, `node_count`,
`policy_id`, `code`, and `action` respectively; a new variant that reused only
existing required field names would be silently routable to an older payload
type by structural deserialisers (notably serde's `untagged` enum mode used in
the Rust binding). The rule keeps the contract safe even when consumers do not
strictly follow the `event_type`-first dispatch guidance above.

### What v1 explicitly does NOT promise

- A terminal `shutdown` / `stopped` event.
- A stdin control protocol.
- Action child stderr being captured as a structured event (covered as a
  potential v2 `action_output` variant — see Open Questions).
- Backpressure semantics beyond "stdout is line-buffered; reader controls
  consumption". A slow reader can block the writer; this is documented in the
  consumer guide, not the contract.

### Migration from pre-contract behaviour

Pre-WOUT, `anvil --json watch` emitted lines shaped as:

```json
{
  "timestamp": "2026-05-14T10:21:30Z",
  "event_type": "Snapshot",
  "detail": "Snapshot { node_count: 312, edge_count: 845, files_watched: 64 }"
}
```

Consumers parsing the old `detail` debug string MUST update to read the v1 typed
`payload` object. The old shape is not preserved — pre-contract output was not
guaranteed and no `schema_version` was present, so any existing consumer that
read it was relying on implementation detail.

## Validation

- `pnpm docs:check` for spec / docs hygiene.
- Round-trip serde tests on the v1 envelope variants live in
  `crates/anvil-kernel-types/src/watch_event.rs`.
- Golden NDJSON fixtures in `crates/anvil-cli/tests/fixtures/watch-json/` pin
  the on-wire shape (WOUT-005).
- An integration test spawns `anvil --json watch` and asserts parseability
  end-to-end (WOUT-004).

## Open Questions

1. Should `anvil.watch.event.v1` also publish a JSON Schema under
   `packages/anvil/contracts` for non-Rust consumers? **Still deferred.**
   WOUT-005 landed golden fixtures first; schema export remains a separate
   future work item.
2. Should JSON mode emit a terminal `shutdown` / `stopped` event on Ctrl-C? **No
   for v1.** EOF terminates the stream; revisit if v2 work shows a need.
3. Should action child `stderr` become an explicit `action_output` event? **Not
   in v1.** Reserved as a candidate v2 variant; v1 inherits stderr.
