# Real-time Operations Feed Contract (Draft)

This document defines the minimum event feed contract required by dashboard operations views.

## Event feed schema

Required fields per event:
- `eventId` (string)
- `timestamp` (ISO-8601 UTC)
- `source` (service/component name)
- `severity` (`info` | `warn` | `error`)
- `type` (machine-readable event type)
- `summary` (human-readable short text)
- `metadata` (object; optional key-value details)

## Transport support

- Preferred: **SSE** for one-way server-to-dashboard updates.
- Optional: **WebSocket** when bi-directional control is needed.

## Reconnect and fallback expectations

- Clients must retry reconnect with bounded exponential backoff.
- Feed consumers must deduplicate by `eventId` on reconnect.
- If real-time transport is unavailable, UI must fallback to periodic polling with degraded-mode indicator.
