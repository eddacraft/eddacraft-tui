---
name: evidence-gate
description: >-
  Executor-side gate: run fresh verification commands and record an evidence
  block before any success claim, land, or handoff to independent verification.
  Use whenever about to say tests pass, bug fixed, or ready to PR.
---

# Evidence gate

**No completion claims without fresh verification evidence.**

This is the **executor** self-check. Independent adversarial verification is
`verify-loop` (separate context, read-only). Both may be required by `dev-loop`.

## When

- About to claim green, fixed, complete, or ready to land.
- Before `land-branch` or opening/updating a PR.
- After `build-tdd` or `debug` before reporting success.
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

For full-suite verification in sandboxed or worktree contexts, start from a
hermetic command environment unless the project policy provides one:

```bash
export HOME="${PWD}/.home"
export XDG_CONFIG_HOME="${PWD}/.xdg/config"
export XDG_CACHE_HOME="${PWD}/.xdg/cache"
export XDG_RUNTIME_DIR="${PWD}/.xdg/runtime"
export TMPDIR="${PWD}/.tmp"
export CARGO_TARGET_DIR="${PWD}/.target"
export PNPM_HOME="${PWD}/.pnpm-home"
mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR" "$TMPDIR" "$CARGO_TARGET_DIR" "$PNPM_HOME"
```

Do not use a shared Cargo target or global package-manager home if it is outside
the writable root or may become read-only during verification.

### 3. Read

Capture exit code, failure count, and a one-line summary per command.

### 4. Map claim → evidence

| Claim            | Requires                        | Not enough             |
| ---------------- | ------------------------------- | ---------------------- |
| Tests pass       | 0 failures in run output        | Previous run           |
| Bug fixed        | Original symptom path passes    | Code changed           |
| Requirements met | ReadyItem behaviours checked    | "Tests pass" alone     |
| Ready to PR      | Repo-mandated local gates green | Single unit file green |

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
- **not-supported** → back to `build-tdd` or `debug`. Do not land.

## Exit

```markdown
## Exit

- Decision: supported | not-supported | blocked
- Next: verify-loop | land-branch | build-tdd | debug | stop
- Notes:
```

## Non-goals

- Not independent/adversarial verification (`verify-loop`).
- Not writing findings dossiers for council (that is review skills).
- Not fixing failures (hand off to build/debug).
