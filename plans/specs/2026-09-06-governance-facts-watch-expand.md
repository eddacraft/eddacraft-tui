# Governance facts, MCP pre-write, and Watch expand

Approved in-thread 2026-09-06. Kindling remains the durable record; this slice
ships the producer and live inspector that do not wait on KFIT-010.

## Goal

Every warn/block/fail that matters is a governance fact. Watch Enter shows the
finding instead of a truncated ticker row.

## This slice

- **DPO-007:** MCP `anvil_validate_write` / `anvil_apply_patch` emit
  `gate_evaluated` with pinned `gate_id` `pre-write` on a non-empty diagnostic
  batch. Clean allows stay silent. Paths are config-gated
  (`ANVIL_OBSERVATION_INCLUDE_PATHS`). The existing DPO sidecar sink admits these
  rows until KFIT-007. Verdicts do not change on sink failure.
- **DPO-008:** Watch Queue Enter expands the selected live candidate: class,
  file, rule id, symbol, full message, recording status `live candidate`. Kernel
  `Violation` events keep `policy_id` and `symbol`.

## Later (not this PR)

KFIT-008 → KFIT-007 → KFIT-009 → KFIT-010 → DPO-003 query → DPO-004/005
dashboard. Watch then joins admitted facts; `recording_gap` replaces the
live-candidate badge when admission fails.

## Non-goals

Web dashboard; NDJSON as the terminal store; retaining clean high-frequency
allows; a new observation kind for warnings.
