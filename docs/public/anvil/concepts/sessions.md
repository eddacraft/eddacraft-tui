---
id: sessions
title: Local runs and retained state
description:
  Understand what a command run records and how long-lived protection differs.
---

# Local runs and retained state

A terminal command is a **run**: it starts, reports evidence, and exits.
Save-time protection is longer-lived and continues until you stop its foreground
process or stop the local daemon.

## Short-lived runs

Examples include:

- `anvil check`;
- `anvil gate`;
- `anvil audit`; and
- `anvil doctor`.

Use machine-readable output when another tool consumes a run.

## Long-lived protection

`anvil watch` observes saves until Ctrl-C. The per-user daemon can serve
validation across runs. Inspect its state with:

```text
anvil intercept status
```

Do not assume that a running daemon means pre-write validation is active. Use:

```text
anvil start --verify
```

for the end-to-end protection state.

## Project and user state

Project configuration and baselines stay with the project. Credentials, daemon
state, caches, and detailed activity live in the user-level anvil home. The
current public beta does not upload those activity records.

See [configuration](../operations/config.md) and
[local data and security](../operations/security.md).

## Next step

Set up [save-time validation](../guides/save-time-validation.md).
