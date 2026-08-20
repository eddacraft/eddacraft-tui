---
id: first-project
title: Define architecture boundaries
description:
  Add and validate a small layered architecture definition for an existing
  project.
owner: ARCHCFG
upstream:
  - crates/anvil-architecture/src/definition.rs
  - crates/anvil-architecture/src/validator.rs
  - crates/anvil-cli/src/commands/architecture.rs
verified_against: 0.9.4-beta
---

# Define architecture boundaries

**For:** teams that want anvil to detect new dependency drift

**Time:** 15–30 minutes

**Outcome:** a validated architecture definition that matches your directory
layout

Complete the [quickstart](quickstart.md) first.

## 1. Identify real layers

Choose directories that already have distinct responsibilities. For example:

- `src/api` receives requests;
- `src/services` contains business logic;
- `src/storage` accesses data; and
- `src/shared` contains dependency-free helpers.

Do not invent a target architecture that the project does not yet follow.

## 2. Create the definition

Create `.anvil/architecture.yaml` in the project root:

```yaml
schema_version: '0.1.0'
template: layered
layers:
  api:
    patterns:
      - 'src/api/**'
    depends_on:
      - services
      - shared
  services:
    patterns:
      - 'src/services/**'
    depends_on:
      - storage
      - shared
  storage:
    patterns:
      - 'src/storage/**'
    depends_on:
      - shared
  shared:
    patterns:
      - 'src/shared/**'
    depends_on: []
```

Change the patterns to match your project. A layer may depend only on the layers
listed in `depends_on`.

The standalone file remains valid, and save-time watch enforcement reads it
directly. The unified home for this definition is the `architecture` section of
the project config (`.anvil.<ext>`) — written inline or delegated to this file
with an `architecture.source` line, which `anvil migrate architecture --apply`
adds for you. `anvil architecture validate` and `show` resolve the section
first, then fall back to the standalone file.

## 3. Validate the file

```text
anvil architecture validate
```

Success means anvil confirms the definition is valid. A schema or pattern error
must be fixed before running an architecture gate.

## 4. Inspect what anvil loaded

```text
anvil architecture show
```

Confirm that the displayed layers and paths match your intent.

## 5. Run the architecture check

```text
anvil gate --only-checks architecture --format plain
```

Existing edges may be adopted into the baseline. The useful signal is a new edge
that violates the declared direction after this point.

## Common problems

- **No files match a layer:** adjust its glob to the actual project root.
- **Everything is in one layer:** start with two meaningful responsibilities
  rather than forcing a detailed model.
- **A legitimate edge is rejected:** review the direction first; add it only
  when the dependency is intentionally part of the architecture.

## Next step

Read [checks, findings, and gates](concepts/gates.md), then add
[Git hooks](operations/git-hooks.md) or
[continuous integration](integrations/github.md).

## Related definitions

- [How anvil evaluates a project](concepts/evaluation-model.md)
- [Check catalogue](reference/checks.md)
- [Checks, findings, and gates](concepts/gates.md)
