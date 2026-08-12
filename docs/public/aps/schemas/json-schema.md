---
id: json-schema
title: Document format reference
description: Look up required sections and fields for APS Markdown documents.
sidebar_position: 1
owner: DOCSYNC
---

# Document format reference

APS uses Markdown headings, tables, and labelled list fields. It is not a JSON
file format; the CLI parser is the executable contract.

## Index

An index file is named `index.aps.md` and requires a `Modules` section. A useful
index also states the overview, problem, success criteria, constraints, risks,
and decisions.

```markdown
# Project name

## Problem & Success Criteria

**Problem:** The problem to solve.

**Success Criteria:**

- [ ] An observable result.

## Modules

| Module                        | ID   | Status | Dependencies |
| ----------------------------- | ---- | ------ | ------------ |
| [auth](./modules/auth.aps.md) | AUTH | Ready  | —            |
```

## Module

A module needs an ID and status metadata table, a `Purpose` section, and a
`Work Items` section.

```markdown
# Authentication

| ID   | Owner | Priority | Status |
| ---- | ----- | -------- | ------ |
| AUTH | @team | high     | Ready  |

## Purpose

Own user authentication.

## Work Items
```

`In Scope`, `Out of Scope`, `Interfaces`, `Constraints`, review date, decisions,
and notes are optional but useful when they remove ambiguity.

## Work item

The heading ID uses an uppercase prefix, a hyphen, and three digits. Every
active item requires `Intent`, `Expected Outcome`, and `Validation`.

```markdown
### AUTH-001: Add login

- **Status:** Ready
- **Intent:** Allow registered users to authenticate.
- **Expected Outcome:** Valid credentials create a session.
- **Validation:** `npm test -- auth`
- **Dependencies:** CORE-001
- **Confidence:** medium
- **Non-scope:** Password recovery.
- **Files:** src/auth, test/auth.test.ts
- **Packages:** api, core
```

The status, dependencies, confidence, non-scope, files, and package fields are
optional to the basic parser, but orchestration requires a meaningful status.

## Action plan

```markdown
# Action Plan: AUTH-001

| Field     | Value    |
| --------- | -------- |
| Work Item | AUTH-001 |
| Status    | Draft    |

## Actions

### Action 1 — Add the login endpoint

**Purpose** Expose the authorised login behaviour.

**Produces** A tested endpoint.

**Checkpoint** Valid credentials create a session.

**Validate** `npm test -- auth`
```

## Status handling

The orchestration lifecycle is `Draft` → `Ready` → `In Progress` → `Complete`,
with `Blocked` available when a named condition prevents progress. `Proposed`
and `Done` are accepted aliases. Lint also recognises imported historical items
labelled merged, released, or shipped as terminal.

## Release file

Files under `plans/releases/` use `v<version>.md` and require a metadata table
with target and status, plus `Release Theme` and `What Ships` sections.

See [copyable fragments](examples.md) and the
[validation rules](../spec/determinism.md).
