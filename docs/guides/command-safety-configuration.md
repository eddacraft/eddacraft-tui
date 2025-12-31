# Command Safety Configuration Reference

Configure Command Safety validation via `.anvilrc` in your project root.

## Configuration Schema

```json
{
  "commandSafety": {
    "enabled": true,
    "strict": false,
    "rules": {
      "disabled": [],
      "overrides": [],
      "custom": []
    },
    "workingDirectory": {
      "allowDeleteInCwd": false,
      "tempDirPatterns": ["/tmp", "/var/tmp"]
    },
    "output": {
      "verbose": true,
      "showSuggestions": true,
      "showReferences": true
    }
  }
}
```

## Options

### `enabled`

**Type:** `boolean`  
**Default:** `true`

Enable or disable the command safety check entirely.

```json
{
  "commandSafety": {
    "enabled": false
  }
}
```

### `strict`

**Type:** `boolean`  
**Default:** `false`

In strict mode, additional warnings are raised for potentially risky operations
that are normally allowed.

```json
{
  "commandSafety": {
    "strict": true
  }
}
```

## Rules Configuration

### `rules.disabled`

**Type:** `string[]`  
**Default:** `[]`

Disable specific rules by ID. Disabled rules are completely ignored.

```json
{
  "commandSafety": {
    "rules": {
      "disabled": ["git-clean-force", "git-stash-drop"]
    }
  }
}
```

### `rules.overrides`

**Type:** `array`  
**Default:** `[]`

Override the action or severity of existing rules.

```json
{
  "commandSafety": {
    "rules": {
      "overrides": [
        {
          "id": "git-push-force",
          "action": "warn"
        },
        {
          "id": "git-reset-hard",
          "action": "disable"
        },
        {
          "id": "rm-rf-with-recursive",
          "severity": "error"
        }
      ]
    }
  }
}
```

**Override fields:**

| Field      | Type                                              | Description                    |
| ---------- | ------------------------------------------------- | ------------------------------ |
| `id`       | `string`                                          | Rule ID to override (required) |
| `action`   | `"block"` \| `"warn"` \| `"allow"` \| `"disable"` | New action                     |
| `severity` | `"error"` \| `"warning"` \| `"info"`              | New severity level             |

### `rules.custom`

**Type:** `array`  
**Default:** `[]`

Add project-specific custom rules.

```json
{
  "commandSafety": {
    "rules": {
      "custom": [
        {
          "id": "no-npm-install-global",
          "category": "shell",
          "command": "npm",
          "subcommand": "install",
          "flags": {
            "dangerous": ["-g", "--global"]
          },
          "action": "warn",
          "severity": "warning",
          "reason": "Global npm installs can affect system state",
          "suggestion": "Use npx or local dependencies instead"
        }
      ]
    }
  }
}
```

**Custom rule fields:**

| Field        | Type                                                 | Required | Description                 |
| ------------ | ---------------------------------------------------- | -------- | --------------------------- |
| `id`         | `string`                                             | Yes      | Unique rule identifier      |
| `category`   | `"git"` \| `"filesystem"` \| `"shell"` \| `"custom"` | Yes      | Rule category               |
| `command`    | `string`                                             | Yes      | Base command to match       |
| `subcommand` | `string`                                             | No       | Subcommand to match         |
| `flags`      | `object`                                             | No       | Flag matching config        |
| `args`       | `object`                                             | No       | Argument matching config    |
| `action`     | `"block"` \| `"warn"` \| `"allow"`                   | Yes      | Action when matched         |
| `severity`   | `"error"` \| `"warning"` \| `"info"`                 | Yes      | Severity level              |
| `reason`     | `string`                                             | Yes      | Human-readable explanation  |
| `suggestion` | `string`                                             | No       | Safe alternative suggestion |
| `references` | `string[]`                                           | No       | Documentation links         |

## Flag Matching

### `flags.required`

At least one of these flags must be present (OR logic).

```json
{
  "flags": {
    "required": ["-f", "--force"]
  }
}
```

### `flags.requiredAll`

All of these flags must be present (AND logic).

```json
{
  "flags": {
    "requiredAll": ["-r", "-f"]
  }
}
```

### `flags.forbidden`

None of these flags may be present.

```json
{
  "flags": {
    "forbidden": ["--dry-run", "-n"]
  }
}
```

### `flags.dangerous`

These flags trigger the rule when present.

```json
{
  "flags": {
    "dangerous": ["--hard", "--force"]
  }
}
```

## Argument Matching

### `args.pattern`

Regex pattern to match against arguments.

```json
{
  "args": {
    "pattern": "^\\.\\.$"
  }
}
```

### `args.position`

Check only a specific argument position (0-indexed).

```json
{
  "args": {
    "pattern": "^drop$",
    "position": 0
  }
}
```

## Working Directory Configuration

### `workingDirectory.allowDeleteInCwd`

**Type:** `boolean`  
**Default:** `false`

Allow `rm -rf` in the current working directory.

```json
{
  "commandSafety": {
    "workingDirectory": {
      "allowDeleteInCwd": true
    }
  }
}
```

### `workingDirectory.tempDirPatterns`

**Type:** `string[]`  
**Default:** `["/tmp", "/var/tmp"]`

Additional patterns to treat as temporary directories (allowed for deletion).

```json
{
  "commandSafety": {
    "workingDirectory": {
      "tempDirPatterns": ["/tmp", "/var/tmp", "/scratch"]
    }
  }
}
```

## Output Configuration

### `output.verbose`

**Type:** `boolean`  
**Default:** `true`

Include detailed reasons in output messages.

### `output.showSuggestions`

**Type:** `boolean`  
**Default:** `true`

Show safe alternative suggestions.

### `output.showReferences`

**Type:** `boolean`  
**Default:** `true`

Include reference documentation links.

```json
{
  "commandSafety": {
    "output": {
      "verbose": true,
      "showSuggestions": true,
      "showReferences": false
    }
  }
}
```

## Default Rule IDs

### Git Rules

| ID                          | Command                       | Action |
| --------------------------- | ----------------------------- | ------ |
| `git-reset-hard`            | `git reset --hard`            | block  |
| `git-reset-merge`           | `git reset --merge`           | warn   |
| `git-checkout-discard`      | `git checkout --`             | block  |
| `git-checkout-all`          | `git checkout .`              | warn   |
| `git-restore-worktree`      | `git restore` (no --staged)   | block  |
| `git-clean-force`           | `git clean -f`                | warn   |
| `git-push-force`            | `git push --force`            | block  |
| `git-branch-force-delete`   | `git branch -D`               | warn   |
| `git-stash-drop`            | `git stash drop`              | warn   |
| `git-stash-clear`           | `git stash clear`             | warn   |
| `git-rebase-abort`          | `git rebase --abort`          | warn   |
| `git-merge-abort`           | `git merge --abort`           | warn   |
| `git-checkout-branch`       | `git checkout -b`             | allow  |
| `git-restore-staged`        | `git restore --staged`        | allow  |
| `git-push-force-with-lease` | `git push --force-with-lease` | allow  |
| `git-branch-safe-delete`    | `git branch -d`               | allow  |
| `git-clean-dry-run`         | `git clean -n`                | allow  |

### Filesystem Rules

| ID                       | Command               | Action |
| ------------------------ | --------------------- | ------ |
| `rm-rf-root`             | `rm -rf /`            | block  |
| `rm-rf-home`             | `rm -rf ~`            | block  |
| `rm-rf-current-dir`      | `rm -rf .`            | block  |
| `rm-rf-parent-traversal` | `rm -rf ..`           | block  |
| `rm-rf-system-dirs`      | `rm -rf /etc`, etc.   | block  |
| `rm-rf-root-glob`        | `rm -rf /*`           | block  |
| `rm-rf-tmp-dir`          | `rm -rf /tmp/*`       | allow  |
| `rm-rf-build-dirs`       | `rm -rf node_modules` | allow  |
| `rm-rf-with-recursive`   | `rm -r` (strict mode) | warn   |
| `rmdir-force`            | `rmdir -p`            | warn   |
| `mv-overwrite`           | `mv -f /`             | warn   |
| `chmod-recursive-777`    | `chmod -R 777`        | warn   |
| `chmod-777-sensitive`    | `chmod /etc`          | block  |
| `chown-recursive-root`   | `chown -R root`       | warn   |
| `dd-block-device`        | `dd of=/dev/sda`      | block  |
| `mkfs-any`               | `mkfs`                | block  |
| `mkfs-ext4`              | `mkfs.ext4`           | block  |
| `mkfs-xfs`               | `mkfs.xfs`            | block  |
| `mkfs-btrfs`             | `mkfs.btrfs`          | block  |

## Example Configurations

### Relaxed Development Mode

```json
{
  "commandSafety": {
    "enabled": true,
    "strict": false,
    "rules": {
      "overrides": [
        { "id": "git-push-force", "action": "warn" },
        { "id": "git-reset-hard", "action": "warn" }
      ]
    }
  }
}
```

### Strict Production Mode

```json
{
  "commandSafety": {
    "enabled": true,
    "strict": true,
    "rules": {
      "overrides": [
        { "id": "git-clean-force", "action": "block" },
        { "id": "git-stash-drop", "action": "block" }
      ]
    }
  }
}
```

### Custom Project Rules

```json
{
  "commandSafety": {
    "rules": {
      "custom": [
        {
          "id": "no-docker-system-prune",
          "category": "shell",
          "command": "docker",
          "subcommand": "system",
          "args": { "pattern": "^prune$", "position": 0 },
          "action": "warn",
          "severity": "warning",
          "reason": "docker system prune removes all unused data",
          "suggestion": "Use docker image prune or docker container prune for targeted cleanup"
        }
      ]
    }
  }
}
```

## Related

- [Command Safety User Guide](./command-safety.md)
- [Gate Configuration](../../cli/README.md#configuration)
