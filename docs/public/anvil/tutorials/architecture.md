---
id: architecture
title: Architecture tutorial
description:
  Model an existing layered project and prove that dependency direction is
  validated.
owner: ARCHCFG
upstream:
  - crates/anvil-cli/src/commands/architecture.rs
  - crates/anvil-architecture/src/validator.rs
  - crates/anvil-cli/src/commands/gate.rs
verified_against: 0.9.0-beta
---

# Architecture tutorial

**For:** projects with meaningful directory boundaries

**Time:** 20–30 minutes

**Outcome:** a validated architecture definition and a reproducible gate

## 1. Create the first definition

Follow [define architecture boundaries](../first-project.md) and adapt the
example to directories that already exist.

## 2. Validate and inspect

```text
anvil architecture validate
anvil architecture show
```

Do not continue until the displayed layers and dependency directions match your
intent.

## 3. Run the focused gate

```text
anvil gate --only-checks architecture --format plain
```

Success is either a pass or findings that name real dependency edges.

## 4. Prove a boundary carefully

On a disposable branch, add one import from a lower-level layer to a layer it
must not depend on. Re-run the focused gate and confirm the new edge is named.
Then revert that deliberate import.

Do not use production code for this experiment when reverting it would be risky.

## Common mistakes

- Patterns do not match the actual source root.
- Every directory is modelled as a layer before responsibilities are clear.
- Dependency direction is reversed.
- A finding is suppressed before checking whether the architecture is wrong.

## Next step

Capture and compare change over time with the [drift tutorial](drift.md).

## Related definitions

- [Architecture boundaries](../concepts/boundaries.md)
- [Check catalogue: `import-boundaries`](../reference/checks.md#import-boundaries)
- [How anvil evaluates a project](../concepts/evaluation-model.md)
- [Define architecture boundaries](../first-project.md)
