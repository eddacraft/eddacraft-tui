# Command Safety Validation

| Type  | Authority     | Owner | Status | Freshness                                                                                                                                            |
| ----- | ------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | CMDSH | Live   | Last reviewed 2026-05-25 against `crates/anvil-checks/tests/command_safety_validation.rs` and `plans/archive/modules/command-safety-surfaces.aps.md` |

| Upstream                                                                                                                                                                     | Downstream                                                                 |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `plans/archive/modules/command-safety-surfaces.aps.md`, `plans/specs/2026-04-21-command-safety-surfaces-design.md`, `crates/anvil-checks/tests/command_safety_validation.rs` | `docs/guides/command-safety-configuration.md`, `anvil gate`, `anvil check` |

Anvil's Command Safety check prevents data loss from destructive shell commands
in plans by validating git and filesystem operations before execution.

## Overview

AI-generated plans may include destructive operations like `git reset --hard`,
`git push --force`, or `rm -rf ~/` that cause irreversible data loss. The
Command Safety check analyses shell commands semantically—distinguishing safe
variants from dangerous ones.

**Key features:**

- Semantic command analysis (not just regex matching)
- 36 default rules for git and filesystem operations
- Shell wrapper unwrapping (`bash -c`, `sudo`, `env`)
- Configurable via `.anvilrc`
- Clear blocking messages with safe alternatives

## Quick Start

Command Safety runs automatically as part of `anvil gate`:

```bash
anvil gate plan.md
```

To skip the check:

```bash
anvil gate plan.md --skip-command-safety
```

Or using the generic skip flag:

```bash
anvil gate plan.md --skip-checks=command-safety
```

## What Gets Blocked

### Git Operations

| Command                            | Action | Reason                                   |
| ---------------------------------- | ------ | ---------------------------------------- |
| `git reset --hard`                 | Block  | Destroys uncommitted changes permanently |
| `git checkout -- <file>`           | Block  | Discards uncommitted changes             |
| `git restore` (without `--staged`) | Block  | Discards working tree changes            |
| `git push --force`                 | Block  | Rewrites remote history                  |
| `git clean -f`                     | Warn   | Removes untracked files                  |
| `git branch -D`                    | Warn   | Force-deletes without merge check        |
| `git stash drop/clear`             | Warn   | Permanently deletes stashed changes      |

### Safe Alternatives (Allowed)

| Command                       | Why It's Safe                         |
| ----------------------------- | ------------------------------------- |
| `git checkout -b <branch>`    | Branch creation                       |
| `git restore --staged`        | Only unstages, preserves working tree |
| `git push --force-with-lease` | Safer force push with remote check    |
| `git branch -d`               | Safe delete with merge verification   |
| `git clean -n`                | Dry-run preview only                  |

### Filesystem Operations

| Command                     | Action | Reason                         |
| --------------------------- | ------ | ------------------------------ |
| `rm -rf /`                  | Block  | Would delete entire filesystem |
| `rm -rf ~` or `$HOME`       | Block  | Would delete home directory    |
| `rm -rf .`                  | Block  | Deletes current directory      |
| `rm -rf ../*`               | Block  | Parent directory traversal     |
| `rm -rf /etc`, `/usr`, etc. | Block  | System directories             |
| `dd of=/dev/sda`            | Block  | Overwrites entire disk         |
| `mkfs.*`                    | Block  | Formats and destroys disk data |

### Safe Filesystem Operations (Allowed)

| Command                              | Why It's Safe         |
| ------------------------------------ | --------------------- |
| `rm -rf /tmp/*`                      | Temporary directory   |
| `rm -rf node_modules`                | Reproducible artifact |
| `rm -rf dist`, `build`, `target`     | Build outputs         |
| `rm -rf .next`, `.cache`, `coverage` | Cache directories     |

## Shell Wrapper Detection

Command Safety unwraps shell wrappers to detect dangerous commands hidden
within:

```bash
# These are all detected and blocked:
bash -c "git reset --hard"
sudo git push --force
env VAR=1 bash -c "rm -rf ~"
sh -c 'git checkout -- .'
```

Wrapper detection supports:

- `bash -c`, `sh -c`
- `sudo`
- `env`
- `command`
- Nested combinations (up to depth 5)

## Output Examples

### Blocked Command

```
✗ Command safety check failed: 1 blocked, 0 warning(s)

Blocked 1 dangerous command(s):

1. git reset --hard
   Reason: git reset --hard permanently destroys uncommitted changes
   Suggestion: Use "git stash" first to preserve your work, or "git reset --soft" for a safer alternative
   Reference: https://git-scm.com/docs/git-reset
```

### Warning

```
⚠ 1 command(s) analysed: 1 warning(s)

Found 1 potentially dangerous command(s):

1. git clean -f
   Reason: git clean -f permanently removes untracked files
   Suggestion: Preview with "git clean -n" (dry-run) first
```

### All Passed

```
✓ All 5 command(s) passed safety check
```

## Configuration

See [Command Safety Configuration](./command-safety-configuration.md) for:

- Disabling specific rules
- Changing rule severity (block → warn)
- Adding custom rules
- Working directory restrictions

## How Commands Are Extracted

Command Safety analyses commands from `script_execute` changes in APS plans:

````json
{
  "type": "script_execute",
  "description": "```bash\ngit reset --hard HEAD~1\n```"
}
````

Code blocks with `bash`, `sh`, or `shell` language hints are parsed. Single-line
descriptions without code blocks are also checked.

## Performance

- Typical overhead: < 50ms per command
- Rules are matched by specificity (most specific rule wins)
- Compound commands (`&&`, `;`, `|`) are split and analysed individually

## Skipping the Check

### Per-Invocation

```bash
# Dedicated flag
anvil gate plan.md --skip-command-safety

# Generic skip flag
anvil gate plan.md --skip-checks=command-safety

# Profile (dev profile skips some checks)
anvil gate plan.md --profile=dev
```

### Environment Variable

```bash
export ANVIL_SKIP_GATES=command-safety
anvil gate plan.md
```

### Configuration File

```json
{
  "version": 1,
  "checks": [
    {
      "name": "command-safety",
      "enabled": false
    }
  ]
}
```

## Related

- [Command Safety Configuration](./command-safety-configuration.md)
- [Gate Command Reference](../architecture/overview.md#check-pipeline)
- [Quality Gates Overview](../architecture/overview.md#gate-layer)
