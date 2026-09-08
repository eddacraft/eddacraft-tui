---
name: dev-loop-executor
description: >-
  Deprecated compatibility shim for older emitted installations. Do not use as
  the orchestration architecture. Load dev-loop-router and the selected native
  adapter; the active harness lead implements directly by default.
---

# Development loop executor compatibility shim

`dev-loop-executor` is retained temporarily because existing Delivery Shadow and
older catalogue emissions may still load this name. It no longer contains four
harness implementations and is not the canonical executor role.

Load, in order:

1. `agentic-loop`
2. `dev-loop-router`
3. the uniquely named adapter selected by the router

The lead/native primary agent implements directly by default. A delegated worker
is optional and must have an explicit reason.

Do not add harness-specific choreography to this file. Keep projected copies
byte-identical until downstream distributions have migrated to `dev-loop-router`.
