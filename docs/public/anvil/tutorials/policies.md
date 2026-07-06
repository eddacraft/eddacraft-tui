---
id: policies
title: Custom Policies
sidebar_position: 2
---

# Custom Policies

anvil evaluates custom Rego policies through an in-process engine (regorus).
This tutorial walks through installing a starter pack, validating it, and
running it through the gate.

For pack authoring detail see the repo guides
[`policy-validation`](https://github.com/eddacraft/anvil-001/blob/main/docs/guides/policy-validation.md)
and
[`opa-policy-testing`](https://github.com/eddacraft/anvil-001/blob/main/docs/guides/opa-policy-testing.md).

## Prerequisites

- **anvil** initialised in your project (`anvil init` or `anvil start`)
- No standalone OPA binary required for gate evaluation (optional for `opa test`
  during local Rego authoring)

## 1. Install a Starter Pack

List bundled packs, then install the baseline pack into `.anvil/policies/`:

macOS / Linux:

```bash
anvil policy install --list
anvil policy install anvil-baseline
```

Windows PowerShell:

```powershell
anvil policy install --list
anvil policy install anvil-baseline
```

The install copies a `pack.yaml` manifest plus member `.rego` policies and their
`*_test.rego` files. Preview without installing:

```bash
anvil policy show anvil-baseline
```

## 2. Validate the Pack

Admission checks run before gate evaluation — manifest schema, metadata,
structure, and pack tests:

macOS / Linux:

```bash
anvil policy validate .anvil/policies/
```

Windows PowerShell:

```powershell
anvil policy validate .anvil/policies/
```

Fix any reported errors before proceeding. `anvil policy validate` is the
supported test path for packs; `anvil policy test` only discovers test files
today (execution is not yet implemented).

## 3. Inspect Policies

macOS / Linux:

```bash
anvil policy list
anvil policy explain <policy-id>
```

Windows PowerShell:

```powershell
anvil policy list
anvil policy explain <policy-id>
```

## 4. Run Through the Gate

macOS / Linux:

```bash
anvil gate --only-checks policy
```

Windows PowerShell:

```powershell
anvil gate --only-checks policy
```

Example output:

```
Checking policies...
  [POLICY] change_scope
    new import crosses architecture boundary: src/ui/panel.rs -> src/db/pool.rs

1 policy warning found.
```

## 5. Authoring Your Own Pack

Add a `pack.yaml` beside your `.rego` files under `.anvil/policies/` (or a
subdirectory). Each member needs complete metadata (`id`, `title`, `severity`,
`owner`, `rationale`, `scope`, `tags`) and a sibling `*_test.rego` with `test_*`
rules. Validate after every change:

```bash
anvil policy validate .anvil/policies/
```

For Rego conventions and fixture layout, see
[`opa-policy-testing`](https://github.com/eddacraft/anvil-001/blob/main/docs/guides/opa-policy-testing.md).

## 6. Exceptions and Enforcement

Scoped, expiring exceptions live in `anvil/exceptions/store.json` and are
managed with `anvil exception grant|revoke|list|verify` — see
[`policy-exceptions`](https://github.com/eddacraft/anvil-001/blob/main/docs/guides/policy-exceptions.md).

Opt-in save-time enforcement (`warn`, `fence`, `interrupt`) is controlled by
`ANVIL_POLICY_ENFORCEMENT` (defaults to report-only). See
[`save-time-validation`](../guides/save-time-validation.md).

## Available Input

Gate policy evaluation receives a compact project snapshot:

- `input.workspace` — absolute workspace root
- `input.files` — policy-relevant workspace-relative files
- `input.changed_files` — files changed according to Git, when available
- `input.profile` — active gate profile, such as `default`, `dev`, or `ci`

Policies do not receive file contents by default. For ad-hoc single-file eval,
use `anvil policy eval` with an explicit input document.

---

**Next:** [Architecture Boundaries](/anvil/tutorials/architecture)
