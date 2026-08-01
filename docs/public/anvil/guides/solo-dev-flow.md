---
id: solo-dev-flow
title: Solo developer workflow
description:
  Add anvil to a personal project without creating unnecessary process.
---

# Solo developer workflow

**For:** one developer working locally

**Time:** 15 minutes to set up

**Outcome:** fast local feedback with an optional pre-push gate

1. Complete the [quickstart](../quickstart.md) once (`anvil start` activates the
   project).
2. On later days, turn protection on with bare `anvil` (daemon + already
   configured MCP). Use `anvil start` again only to change configuration.
3. Keep a watcher visible while editing: `anvil watch`.
4. Scan the final change: `anvil check --changed --format plain`.
5. Run a broader decision before pushing:
   `anvil gate --profile dev --format plain`.
6. Review the finding and fix the cause; do not add a suppression merely to make
   the result disappear.

## Keep the workflow light

Begin with advisory feedback. Add hooks only when the manual commands are useful
and predictable. A solo workflow should shorten the feedback loop, not add
ceremony.

## Success check

You should be able to explain which command covers saves, which covers the final
change, and how to stop or remove each integration.

## Next step

Use [Git hooks](../operations/git-hooks.md) when the manual pre-push step
becomes easy to forget.
