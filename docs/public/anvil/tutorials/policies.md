---
id: policies
title: Policy tutorial
description: Inspect, install, validate, test, and gate a bundled policy pack.
owner: CPACKS
upstream:
  - crates/anvil-cli/src/commands/policy/install.rs
  - crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline
  - crates/anvil-cli/src/commands/policy/validate.rs
  - crates/anvil-cli/src/commands/policy/test_run.rs
verified_against: 0.9.7-beta
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

## What this pack does not do

`anvil-baseline` is a **starter pack**, not compliance coverage. Read this
before you rely on it.

**It is advisory only.** Every finding is a warning. The pack never fails a
gate, whatever severity its manifest declares — blocking comes from anvil's
enforcement posture, not from Rego severity.

**It is not a compliance control set.** There is no OWASP, SOC 2, ISO 27001, or
GDPR mapping in this pack, and installing it is not evidence for any of them.
anvil makes no compliance claim.

**What it actually inspects** is the working-tree diff, and only two things:

| Policy            | Flags                                                                                                                                                                                               |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `change-scope`    | Change sets past 10 files, and again past 25.                                                                                                                                                       |
| `sensitive-paths` | GitHub Actions workflow and action definitions, files ending `.env` or containing `.env.`, and paths whose _name_ contains `secret`, `credential`, `token`, `password`, `apikey`, or ends `id_rsa`. |

**What it does not inspect:**

- **File contents.** `sensitive-paths` matches path names only. It cannot tell
  whether a file holds a credential. Credential _detection_ is the separate
  `secret` check — see the [check catalogue](../reference/checks.md).
- **Anything outside the diff.** A pre-existing problem in an unchanged file is
  invisible to this pack.
- **Data flow.** No taint tracking, no reachability, no cross-file analysis.

**Known false positives.** The `sensitive-paths` name heuristics fire on
ordinary files — `token_store.rs`, `password_field.tsx`, `secrets.example.md`.
That is deliberate: the softer wording tells you to confirm rather than act, and
no finding from this pack is blocking.

**There is no way to suppress an individual pack finding at the policy gate
today.** [`anvil exception`](../reference/policy.md#exceptions) records a
scoped, attributed exception, but it is applied at the pre-push check — not by
`anvil gate --only-checks policy`, the gate this tutorial runs. Until that
changes, treat a heuristic hit as a prompt to confirm. If a pattern is
persistently wrong for your repository, edit your installed copy under
`.anvil/policies/anvil-baseline/` and re-run `anvil policy validate`.

**Thresholds are fixed.** The 10- and 25-file limits are constants in the pack's
Rego. They are not configurable per project today.

## Next step

Add the proven command to the [team workflow](../guides/team-flow.md).

## Related definitions

- [Policy model](../concepts/policy-model.md)
- [Policy command reference](../reference/policy.md)
- [Check catalogue: `policy`](../reference/checks.md#policy)
- [Introduction baseline](../concepts/baseline.md)
- [How anvil evaluates a project](../concepts/evaluation-model.md)
