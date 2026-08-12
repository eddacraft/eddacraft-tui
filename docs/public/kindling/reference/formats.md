---
id: formats
title: Formats
description:
  JSON shapes for observations, capsules, and the export/import bundle.
sidebar_position: 3
owner: DOCSYNC
---

# Formats

The wire and on-disk JSON shapes Kindling uses. All timestamps are **epoch
milliseconds** unless noted. JSON keys are `camelCase`.

## Observation

```json
{
  "id": "9c20f3a1-2b4d-4f8e-9a1c-7e0d5b6f2a33",
  "kind": "error",
  "content": "JWT validation failed: token expired",
  "provenance": {
    "stack": "Error: Token expired\n  at validateToken.ts:42"
  },
  "ts": 1750416064123,
  "scopeIds": { "sessionId": "session-1", "repoId": "my-project" },
  "redacted": false
}
```

| Field        | Type    | Notes                                                        |
| ------------ | ------- | ------------------------------------------------------------ |
| `id`         | string  | Unique identifier (UUID).                                    |
| `kind`       | enum    | See [kinds](#observation-kinds).                             |
| `content`    | string  | The captured text.                                           |
| `provenance` | object  | Free-form, source-specific metadata.                         |
| `ts`         | number  | Epoch milliseconds.                                          |
| `scopeIds`   | object  | Any of `sessionId`, `repoId`, `agentId`, `userId`, `taskId`. |
| `redacted`   | boolean | Whether the content has been forgotten.                      |

### Observation kinds

```
tool_call | command | file_diff | error | message
node_start | node_end | node_output | node_error
```

See [Observations](/kindling/concepts/observations#kinds) for what each means.

## Capsule

```json
{
  "id": "cap_8a3f...",
  "type": "session",
  "intent": "investigating the memory leak",
  "status": "closed",
  "openedAt": 1750416000000,
  "closedAt": 1750416600000,
  "scopeIds": { "repoId": "my-project" },
  "observationIds": ["9c20f3a1-...", "5f1c..."],
  "summaryId": "sum_1b2c..."
}
```

| Field            | Type     | Notes                                    |
| ---------------- | -------- | ---------------------------------------- |
| `type`           | enum     | `session` or `pocketflow_node`.          |
| `status`         | enum     | `open` or `closed`.                      |
| `closedAt`       | number?  | Present only once closed.                |
| `observationIds` | string[] | Member observation IDs, in order.        |
| `summaryId`      | string?  | Set when a summary is produced on close. |

## Summary

```json
{
  "id": "sum_1b2c...",
  "content": "Fixed JWT expiration check in middleware",
  "confidence": 0.85,
  "evidenceRefs": ["9c20f3a1-..."],
  "capsuleId": "cap_8a3f...",
  "ts": 1750416600000,
  "scopeIds": { "repoId": "my-project" }
}
```

## Pin

```json
{
  "id": "pin_4d5e...",
  "targetType": "observation",
  "targetId": "9c20f3a1-...",
  "reason": "root cause — keep handy",
  "createdAt": 1750416100000,
  "expiresAt": null,
  "scopeIds": { "sessionId": "session-1" }
}
```

The CLI `--note` flag on `kindling pin` maps to `reason` in JSON.

## Redacted content

Forgotten observations keep their record but replace `content` with `[redacted]`
and set `redacted: true`. They are excluded from exports and from retrieval
unless `includeRedacted` is set on programmatic retrieve calls.

## Export bundle

`kindling export` writes a bundle that round-trips through `kindling import`:

```json
{
  "bundleVersion": "1.0",
  "exportedAt": 1750416700000,
  "dataset": {
    "version": "1.0",
    "exportedAt": 1750416700000,
    "scope": { "repoId": "my-project" },
    "observations": [
      /* Observation[] */
    ],
    "capsules": [
      /* Capsule[] */
    ],
    "summaries": [
      /* Summary[] */
    ],
    "pins": [
      /* Pin[] */
    ]
  },
  "metadata": {
    "description": "Kindling memory export",
    "exportedAt": "2025-06-20T11:05:00.000Z"
  }
}
```

| Field           | Notes                                                                                                            |
| --------------- | ---------------------------------------------------------------------------------------------------------------- |
| `bundleVersion` | Bundle format version (`"1.0"`).                                                                                 |
| `exportedAt`    | Epoch milliseconds.                                                                                              |
| `dataset`       | The entity payload. Entities are ordered deterministically (observations by `ts`, capsules by `openedAt`, etc.). |
| `dataset.scope` | The scope filter applied, when `--session`/`--repo` was given. Omitted otherwise.                                |
| `metadata`      | Optional. Carries a human description and an ISO-8601 `exportedAt`.                                              |

Redacted observations are excluded from exports.

## Next

- [Core concepts: observations →](/kindling/concepts/observations)
- [CLI: export / import →](/kindling/reference/cli#export)
