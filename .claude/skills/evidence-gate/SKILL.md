---
name: evidence-gate
description: >-
  Run fresh verification commands and record an evidence block before any
  success claim — before saying tests pass, the bug is fixed, work is
  complete, or ready to PR. Independent adversarial review of someone
  else's claim is `verify-loop`.
---

# Evidence gate

**No completion claims without fresh verification evidence.**

This is the **executor** self-check. Independent adversarial verification is
`verify-loop` (separate context, read-only). Both may be required by `dev-loop`.

## When

- About to claim green, fixed, complete, or ready to land.
- Before `land-branch` or opening/updating a PR.
- After `loop-build-tdd` or `loop-debug` before reporting success.
- Before ticking any test-plan checkbox.

## Hard rules

1. If you have not run the proving command **in this turn**, you cannot claim pass.
2. Read **full output** and **exit codes** — not vibes, not prior agent reports.
3. Partial checks are not full gates.
4. Inherited baseline failures must be named; they are not silent passes.
5. Never treat "should work" / "probably" / "looks good" as evidence.
6. Classify environment/tooling failures separately from product failures. A
   missing compiler, unwritable cache, full temp filesystem, package-manager
   registration write, or sandbox `EROFS` is `blocked: tooling-environment` until
   rerun in a hermetic writable setup.
7. Evidence is reusable only by exact identity, never by recency or a green
   label. Record target, base/head, tree digest, dirty state, environment
   digest, governing sources, argv, duration, output digests, and durable logs.

## Steps

### 1. Identify commands

In order of priority:

1. ReadyItem **Validation commands**
2. Repository policy / `CLAUDE.md` / CI-equivalent local gates
3. Focused tests for the changed surface

Common full gates (examples only — use the project's real commands):

- JS/TS: format + lint + typecheck + test
- Rust: `cargo test --workspace` (and project clippy/fmt if mandated)

### 2. Run fresh

Execute each command completely. Do not skip on confidence.

Before rerunning a full gate, inspect an existing v2 evidence bundle. Reuse is
allowed only when every identity field in rule 7 matches exactly and the log
references remain readable. A new repair head, dirty-state change, environment
change, changed argv, missing digest, or changed contract invalidates reuse.
Run focused checks after each repair; run the expensive full suite once per
immutable matching state, not once per orchestration stage.

For full-suite verification in sandboxed or worktree contexts, start from a
hermetic command environment unless the project policy provides one:

Create worktree-local directories for `HOME`, `XDG_CONFIG_HOME`,
`XDG_CACHE_HOME`, `XDG_RUNTIME_DIR`, `TMPDIR`, `CARGO_TARGET_DIR`, and
`PNPM_HOME`, then set those values through the harness command environment. In
PowerShell use `$env:NAME = <value>`; in a POSIX shell use
`export NAME=<value>`.

Do not use a shared Cargo target or global package-manager home if it is outside
the writable root or may become read-only during verification.

Adopt the repository's own cache convention when it has one (`.envrc`, worktree
hooks, `AGENTS.md`) rather than inventing a path. Keep `TMPDIR` and
`XDG_RUNTIME_DIR` short, because Unix socket paths are capped by `sun_len`; that
limit does not apply to build caches, so it is not a reason to put one on
`/tmp`. Put `CARGO_TARGET_DIR` and package-manager stores on disk under `$HOME`
— on most Linux hosts `/tmp` is a RAM-backed tmpfs, and nothing removes a target
directory when the run ends. Delete the hermetic directories you created, or
report the ones you deliberately kept.

### 3. Read

Capture structured argv (not a shell-rendered command), working directory, exit
code, duration, failure count, stdout/stderr digests, durable log reference, and
a one-line summary per command. Emit the v2 evidence bundle from
`dev-loop/references/evidence-bundle.schema.json`.

### 4. Map claim → evidence

| Claim            | Requires                        | Not enough             |
| ---------------- | ------------------------------- | ---------------------- |
| Tests pass       | 0 failures in run output        | Previous run           |
| Bug fixed        | Original symptom path passes    | Code changed           |
| Requirements met | ReadyItem behaviours checked    | "Tests pass" alone     |
| Ready to PR      | Repo-mandated local gates green | Single unit file green |

Scale evidence to the ReadyItem risk class:
[references/evidence-menu.md](references/evidence-menu.md). High and critical
classes add categories (rollback, migration dry-run, domain reconciliation);
the base gates alone do not support the claim there.

### 5. Emit evidence block

Use the shape in `references/contracts.md`:

```markdown
## Evidence

- Target:
- Claim:
- Commands:
  - `...` → exit N — summary
- Classification: product-failure | tooling-environment | inherited-baseline | pass
- Base..head:
- Result: supported | not-supported
- Notes:
```

### 6. Decide

- **supported** → may proceed to `verify-loop` (if required) or `land-branch`.
- **not-supported** → back to `loop-build-tdd` or `loop-debug`. Do not land.

## Exit

```markdown
## Exit

- Decision: supported | not-supported | blocked
- Next: verify-loop | land-branch | loop-build-tdd | loop-debug | stop
- Notes:
```

## Non-goals

- Not independent/adversarial verification (`verify-loop`).
- Not writing findings dossiers for council (that is review skills).
- Not fixing failures (hand off to build/debug).
