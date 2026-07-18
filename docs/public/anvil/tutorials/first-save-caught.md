---
id: first-save-caught
title: Catch a saved change
description:
  Prove that the save-time watcher analyses a supported file after it is saved.
---

# Catch a saved change

**For:** users who want visible editor-independent save-time feedback

**Time:** 10 minutes

**Outcome:** a save produces a finding or explicit clean result in the watcher

## 1. Start the watcher

From the project root:

```text
anvil watch
```

Wait for the ready message and leave the terminal open. Source, findings, and
repository metadata stay on your machine.

## 2. Create a safe temporary file

In another terminal, follow step 1 of the
[ten-minute protection tutorial](../first-gate.md) to create
`anvil-docs-tutorial/check.ts`.

Save the file from your editor. Success means the watcher names it and reports
the deliberate broad suppression or unsafe type.

## 3. Fix and save

Follow step 3 of the same tutorial, save again, and confirm a clean result.

## 4. Clean up

Remove the temporary file, then press Ctrl-C in the watcher terminal.

## If no event appears

Run `anvil check anvil-docs-tutorial/check.ts --format plain`. If the named
check works, use
[watcher troubleshooting](../operations/troubleshooting.md#the-watcher-shows-no-saves).

## Next step

Add [Git hooks](../operations/git-hooks.md).
