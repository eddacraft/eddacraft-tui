---
id: first-gate
title: Ten-minute protection tutorial
description:
  Create a safe temporary finding, see anvil detect it, fix it, and confirm the
  clean result.
sidebar_position: 3
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/check.rs
  - patterns/compiled/registry.json
verified_against: 0.9.0-beta
---

# Ten-minute protection tutorial

**For:** users who completed the quickstart

**Time:** about 10 minutes

**Outcome:** prove that anvil can detect a new problem and confirm its fix

This tutorial creates a reserved `anvil-docs-tutorial/` directory in your
project. It refuses to start if that path already exists, so it cannot overwrite
an existing file. Remove the directory at the end.

## Before you begin

- Complete [install and get first value](quickstart.md).
- Have approved beta access and complete `anvil auth login`.
- Run `anvil auth whoami` and confirm that it shows the identity you intend to
  use. The `anvil check` command in this tutorial requires authentication.
- Run from the root of a project.
- Confirm `anvil version` works.

## 1. Create a deliberate finding

macOS or Linux:

```bash
test ! -e anvil-docs-tutorial &&
  mkdir anvil-docs-tutorial &&
  printf '/* eslint-disable */\nexport const answer: any = 42;\n' \
    > anvil-docs-tutorial/check.ts
```

Windows PowerShell:

```powershell
if (Test-Path .\anvil-docs-tutorial) {
  throw '.\anvil-docs-tutorial already exists; choose another project or move it safely.'
}
New-Item -ItemType Directory -Path .\anvil-docs-tutorial | Out-Null
Set-Content -Path .\anvil-docs-tutorial\check.ts -Value @(
  '/* eslint-disable */'
  'export const answer: any = 42;'
)
```

The example contains two intentional escape hatches: a broad lint suppression
and an explicit `any` type.

## 2. Check the file

```text
anvil check anvil-docs-tutorial/check.ts --format plain
```

Success means the output names one or more findings for
`anvil-docs-tutorial/check.ts`. Rule IDs can differ as the catalogue evolves;
the explanation should identify the broad suppression or unsafe type.

If the result is clean, confirm that the file contains both lines exactly, then
compare the extension with the [support matrix](reference/support.md).

## 3. Fix the file

macOS or Linux:

```bash
printf 'export const answer: number = 42;\n' > anvil-docs-tutorial/check.ts
```

Windows PowerShell:

```powershell
Set-Content -Path .\anvil-docs-tutorial\check.ts -Value 'export const answer: number = 42;'
```

Run the same check again:

```text
anvil check anvil-docs-tutorial/check.ts --format plain
```

Success is an explicit clean result with no finding for the temporary file.

## 4. Remove the temporary file

macOS or Linux:

```bash
rm anvil-docs-tutorial/check.ts
rmdir anvil-docs-tutorial
```

Windows PowerShell:

```powershell
Remove-Item .\anvil-docs-tutorial\check.ts
Remove-Item .\anvil-docs-tutorial
```

Both directory-removal commands refuse to remove a non-empty directory. If you
added another file there during the tutorial, move it deliberately before
retrying cleanup.

## What you proved

You observed the complete loop: create a change, receive a finding, make a safe
correction, and verify the result.

## Next step

Use [protect AI-assisted writes](guides/agent-harness.md) or
[save-time validation](guides/save-time-validation.md) for your normal workflow.

## Related definitions

- [How anvil evaluates a project](concepts/evaluation-model.md)
- [What anvil can do](reference/what-anvil-can-do.md)
- [Check catalogue](reference/checks.md)
- [Glossary](concepts/glossary.md)
