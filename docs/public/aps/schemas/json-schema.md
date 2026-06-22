---
id: json-schema
title: Document Structure
description: APS document types, required sections, and field reference.
sidebar_position: 1
---

# Document Structure

| Type      | Authority | Owner   | Status | Freshness                                              |
| --------- | --------- | ------- | ------ | ------------------------------------------------------ |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream           |
| ------------------------------------------------------------------------- | -------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

APS is markdown-native. There is no separate binary format — documents are
validated by structure and field conventions enforced by `aps lint`.

## Document types

| Type        | File pattern              | Executable? | Key sections                          |
| ----------- | ------------------------- | ----------- | --------------------------------------- |
| Index       | `index.aps.md`            | No          | Overview, Modules, Milestones           |
| Module      | `modules/*.aps.md`        | If Ready    | Purpose, Work Items                     |
| Action Plan | `execution/*.actions.md`  | Yes         | Actions with checkpoints                |
| Issues      | `issues.md`               | No          | Issues (ISS-NNN), Questions (Q-NNN)     |
| Release     | `releases/v*.md`          | No          | Release Theme, What Ships               |
| Design      | `designs/*.design.md`     | No          | Problem, Approach, Decisions            |

## Index structure

```markdown
# Plan Title

## Overview
[One paragraph]

## Problem & Success Criteria
**Problem:** [...]
**Success Criteria:**
- [ ] [...]

## Modules
| Module | ID | Owner | Status | Priority | Dependencies |
| [...]  |    |       |        |          |              |
```

Required sections (lint E004): `## Modules`

## Module structure

```markdown
# Module Title

| ID   | Owner | Priority | Status |
| ---- | ----- | -------- | ------ |
| AUTH | @you  | high     | Draft  |

## Purpose
[Why this module exists]

## In Scope
- [...]

## Out of Scope _(optional)_
- [...]

## Interfaces _(optional)_
**Depends on:** [...]
**Exposes:** [...]

## Work Items
### AUTH-001: [Title]
- **Intent:** [...]
- **Expected Outcome:** [...]
- **Validation:** `[command]`
```

Required sections (lint E001, E002, E003): metadata table, `## Purpose`,
`## Work Items`

## Work item fields

### Required

| Field              | Format   | Description                         |
| ------------------ | -------- | ----------------------------------- |
| ID                 | `PREFIX-NNN` | Unique identifier               |
| Intent             | string   | One-sentence outcome                |
| Expected Outcome   | string   | Testable or observable result       |
| Validation         | string   | Command or condition to verify      |

### Optional

| Field        | Format                              | Description                    |
| ------------ | ----------------------------------- | ------------------------------ |
| Status       | enum                                | See status vocabulary below      |
| Dependencies | list of IDs                         | Upstream work items            |
| Confidence   | `low` \| `medium` \| `high`         | Uncertainty level              |
| Scope        | string                              | What will change               |
| Non-scope    | string                              | What will not change           |
| Files        | list of paths                       | Best-effort affected files       |
| Risks        | list                                | Potential risks                  |
| Learning     | string                              | Captured by `aps complete`       |
| Packages     | list                                | Affected packages (monorepo)   |

### Status vocabulary

Canonical: `Draft`, `Ready`, `In Progress`, `Complete`, `Blocked`

Aliases (normalised internally): `Proposed` → `Draft`, `Done` → `Complete`

Terminal compaction: `Merged`, `Released`, `Shipped` (treated as Complete for
dependency checks)

## Action plan structure

```markdown
# Action Plan: AUTH-001

| Field     | Value                    |
| --------- | ------------------------ |
| Work Item | AUTH-001 — Login endpoint |
| Status    | Draft                    |

## Actions

### Action 1 — [Verb] [target]

**Purpose**
[Why this action exists]

**Produces**
[Concrete artefacts]

**Checkpoint**
[Observable state — max ~12 words]

**Validate**
`[command]` _(optional)_

**Wave** 1 _(optional)_
```

## Issues structure

```markdown
# Development Issues

## Issues

### ISS-001: [Title]

| Field      | Value    |
| ---------- | -------- |
| Status     | Open     |
| Severity   | medium   |
| Discovered | AUTH-002 |
| Module     | AUTH     |

**Context:** [...]

## Questions

### Q-001: [Title]

| Field      | Value    |
| ---------- | -------- |
| Status     | Open     |
| Priority   | low      |
| Discovered | planning |
```

Required sections (lint E010, E011): `## Issues`, `## Questions`

## Release structure

```markdown
# v0.4.0

| Field  | Value      |
| ------ | ---------- |
| Target | 2026-06-01 |
| Status | Shipped    |

## Release Theme
[One paragraph]

## What Ships
| Capability | Module | Work Items |
| [...]      |        |            |
```

Required sections (lint R003, R004): `## Release Theme`, `## What Ships`

## Anvil repository extension

The Anvil product repository uses a richer operating-model lifecycle for
active planning. This is repository guidance, not the portable APS package
contract:

```text
Draft → Proposed → Ready → In Progress → Merged → Released/Shipped → Complete
```

Work items may also carry release reconstruction metadata (`changeType`,
`releaseIntent`, `releaseScope`, `releaseNote`). These fields are specific to
Anvil's release operating model and are not required by `aps lint` in generic
APS projects.

---

**Next:** [Schema examples →](./examples.md)
