---
id: policies
title: Policy tutorial
description: Inspect, install, validate, test, and gate a bundled policy pack.
---

# Policy tutorial

**For:** teams ready to add project-specific policy

**Time:** 20–30 minutes

**Outcome:** a starter policy pack is reviewed and validated before enforcement

A policy is a project rule evaluated by a gate. Begin with a bundled pack rather
than copying an unknown policy from another repository.

## 1. Discover available packs

```text
anvil policy install --list
```

Success lists `anvil-baseline`, the bundled starter pack used below.

## 2. Preview before writing

```text
anvil policy show anvil-baseline
```

This previews the pack without writing files. Read its purpose, inputs, and
included tests before continuing.

## 3. Install deliberately

```text
anvil policy install anvil-baseline
```

Success reports files created under `.anvil/policies/anvil-baseline/`. The
installer refuses to overwrite that directory; this tutorial never uses
`--force`.

## 4. Validate and test

```text
anvil policy validate .anvil/policies/anvil-baseline
anvil policy test .anvil/policies/anvil-baseline
```

Both commands must exit successfully. Validation checks the manifest and pack
structure; the test command runs the pack's included examples.

## 5. Run a focused gate

```text
anvil gate --only-checks policy --format plain
```

## Recovery

If the pack is not appropriate, first review the exact tutorial-owned path:

```text
git status --short -- .anvil/policies/anvil-baseline
```

Only if the install step above created that directory, remove it.

macOS or Linux:

```bash
rm -r .anvil/policies/anvil-baseline
```

Windows PowerShell:

```powershell
Remove-Item -Recurse .\.anvil\policies\anvil-baseline
```

Do not delete the whole `.anvil` directory; it can contain unrelated project
configuration and evidence.

## Next step

Add the proven command to the [team workflow](../guides/team-flow.md).
