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

```text
anvil watch --json
```

when your installed version supports machine-readable watch events.

## Do not infer more than the output proves

- A ready watcher proves save-time coverage, not pre-write protection.
- A clean file result covers that analysed input, not the entire repository.
- A daemon status proves the service is reachable, not that an AI client is
  connected.
- A skipped file should name the coverage reason.

Use `anvil start --verify` for the end-to-end protection state.

## Next step

See [activation states](../guides/start-output-contracts.md).
