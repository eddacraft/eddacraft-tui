---
id: plans
title: Plans and project checks
description:
  Understand when anvil needs a plan and when it can check code without one.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/plan.rs
  - crates/anvil-cli/src/commands/config.rs
  - crates/anvil-cli/src/commands/check.rs
verified_against: 0.9.0-beta
---

# Plans and project checks

You do not need a plan to begin using anvil.

## Planless checks

`anvil check` can scan named, changed, staged, or all supported source files for
checks that do not require project-specific policy.

```text
anvil check --changed --format plain
```

This is the simplest path for first use and ad-hoc scanning.

## Project configuration

Activation can create an anvil configuration file that selects checks and
records project settings. Use:

```text
anvil config show
```

to inspect the effective configuration, and:

```text
anvil config --help
```

to see the configuration operations in your installed CLI. `anvil config show`
reports a parsing error when the project configuration is invalid.

## Plan-aware gates

Some workflows evaluate a structured plan file or project policy. Pass a plan
only when your team already uses one:

```text
anvil gate path/to/work.aps.md --profile dev
```

If no plan is supplied, the gate runs against the configured project.

## Next step

Learn [how gates work](gates.md) or define
[architecture boundaries](../first-project.md).
