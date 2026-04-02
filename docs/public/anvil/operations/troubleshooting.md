---
id: troubleshooting
title: Troubleshooting
description: Common issues and how to resolve them.
sidebar_position: 3
---

# Troubleshooting

Common issues and solutions for Anvil.

## Installation Issues

### "Command not found: anvil"

The `anvil` binary isn't in your PATH.

**macOS / Linux:**

```bash
# Re-run the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or add the install directory to your PATH manually
export PATH="$HOME/.eddacraft/bin:$PATH"
```

If you installed via Homebrew, run `brew link eddacraft/tap/anvil`.

**Windows (PowerShell):**

```powershell
# Re-run the installer
irm https://install.eddacraft.ai/windows | iex

# Or add the install directory to your PATH manually
$env:Path = "$env:USERPROFILE\.eddacraft\bin;$env:Path"

# To persist across sessions, add to your profile:
Add-Content $PROFILE '$env:Path = "$env:USERPROFILE\.eddacraft\bin;$env:Path"'
```

### Migrating from the Node.js package

If you previously used `@eddacraft/anvil-cli` via npm/pnpm, remove it and
install the native binary:

**macOS / Linux:**

```bash
# Remove the old package
pnpm remove @eddacraft/anvil-cli
# or: npm uninstall @eddacraft/anvil-cli

# Install the native binary
curl -fsSL https://install.eddacraft.ai | sh
```

**Windows (PowerShell):**

```powershell
# Remove the old package
npm uninstall -g @eddacraft/anvil-cli

# Install the native binary
irm https://install.eddacraft.ai/windows | iex
```

See [The Switch to Rust](/anvil/releases/rust-rewrite) for full migration
details.

## Configuration Issues

### "Configuration file not found"

No `.anvilrc` in project root.

**Solution:**

```bash
anvil init
```

### "Invalid configuration"

Config syntax error.

**Solution:**

```bash
# Validate config
anvil doctor

# Common issues:
# - Missing commas
# - Trailing commas (not allowed in JSON)
# - Wrong types (string vs array)
```

### Boundaries Not Working

Boundary patterns not matching files.

**Debugging:**

```bash
# Test pattern matching
anvil check src/api/users.ts --verbose
```

**Common issues:**

- Glob pattern mismatch (`src/api` vs `src/api/**`)
- Wrong path separator on Windows (use `/` in glob patterns, not `\`)
- Pattern doesn't include file extension

## Runtime Issues

### Watch Mode Not Detecting Changes

Files changing but Anvil not responding.

**Solutions:**

1. Check ignore patterns:

   ```bash
   cat .anvilrc | grep ignore
   ```

2. Check file extensions:

   ```json
   { "watch": { "extensions": [".ts", ".tsx"] } }
   ```

3. Increase debounce:

   ```json
   { "watch": { "debounceMs": 500 } }
   ```

4. Check file system events:

   ```bash
   # Linux: /proc/sys/fs/inotify/max_user_watches
   cat /proc/sys/fs/inotify/max_user_watches
   # Increase if needed:
   echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf

   # macOS: generally not an issue (uses FSEvents)

   # Windows: generally not an issue (uses ReadDirectoryChangesW)
   ```

### High CPU Usage

Anvil consuming too much CPU.

**Solutions:**

1. Increase debounce time
2. Add more patterns to ignore
3. Disable `validateOnType` in VS Code
4. Check for circular watch triggers

### Memory Issues

Out of memory errors.

**Solutions:**

```bash
# Increase Node memory
NODE_OPTIONS="--max-old-space-size=4096" anvil watch
```

Check for:

- Very large files being scanned
- Many files in watch scope
- Circular dependencies causing infinite loops

## Gate Check Issues

### False Positives

Anvil flagging code that's actually fine.

**Solutions:**

1. Add specific suppression with explanation:

   ```typescript
   // @anvil-ignore AP-003 Using any for JSON.parse result
   ```

2. Add pattern suppression in config:
   ```json
   {
     "suppressions": [
       {
         "pattern": "src/types/external.ts",
         "checks": ["AP-003"],
         "reason": "External type definitions"
       }
     ]
   }
   ```

### Missing Issues

Anvil not catching problems it should.

**Check:**

```bash
# Verify check is enabled
cat .anvilrc | grep -A5 antiPatterns

# Run verbose
anvil check --all --verbose
```

### Architecture Check Slow

Boundary validation taking too long.

**Solutions:**

1. Reduce `maxDepth`:

   ```json
   { "architecture": { "maxDepth": 5 } }
   ```

2. Add more specific patterns (less files to scan)

3. Ensure node_modules is ignored

## CI Issues

### Exit Code Always 0

Anvil passing when it should fail.

**Check:**

- Prefer `anvil gate --profile ci` so CI-specific profile settings are applied (bare `anvil gate` runs all checks by default)
- Config is being read (check `anvil gate --list-profiles`)
- Exit code `2` indicates gate failure (not `1`, which is a general error)

### GitHub Action Timeout

Action taking too long.

**Solutions:**

```yaml
- uses: eddacraft/anvil-action@v1
  timeout-minutes: 10
```

Or:

- Enable caching
- Reduce check scope
- Parallelise with matrix

### PR Comments Not Appearing

Action runs but no comment.

**Check:**

- `github_token` is provided
- Token has `pull-requests: write` permission
- `comment: true` is set

## VS Code Extension Issues

### Extension Not Loading

No diagnostics appearing.

**Solutions:**

1. Check Output panel → Anvil
2. Verify config file exists
3. Reload window: `Ctrl+Shift+P` → "Reload Window"
4. Check extension is enabled for workspace

### Diagnostics Stale

Old issues still showing.

**Solutions:**

```
Ctrl+Shift+P → "Anvil: Clear Cache"
```

Or restart VS Code.

## Windows-Specific Issues

### Path Separators in Configuration

Anvil uses forward slashes (`/`) for glob patterns on all platforms. Do not use
backslashes in `.anvilrc` boundary patterns, even on Windows:

```json
{
  "pattern": "src/api/**"
}
```

Using `src\\api\\**` will not match.

### Antivirus Interference

Some antivirus software (Windows Defender, Norton, etc.) can slow down watch
mode by scanning files that Anvil accesses. If watch mode is unusually slow:

1. Add your project directory to the antivirus exclusion list
2. Add `%USERPROFILE%\.eddacraft\` to the exclusion list
3. Add `anvil.exe` to the allowed programs list

### PowerShell Execution Policy

If the installer fails with an execution policy error:

```powershell
# Check current policy
Get-ExecutionPolicy

# Allow scripts for current user (if needed)
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Long Path Support

Anvil can work with long paths (> 260 characters) on Windows as long as Windows
long-path support is enabled and your tooling supports it. If you encounter
path-related errors in deeply nested `node_modules`:

1. Enable long paths in Windows (requires admin):
   ```powershell
   New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" `
     -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
   ```
2. Restart your terminal

### WSL vs Native Windows

Anvil works in both native Windows (PowerShell/cmd) and WSL. If using WSL:

- Use the Linux installer (`curl ... | sh`), not the Windows PowerShell one
- File watching across the WSL/Windows boundary (e.g. `/mnt/c/`) is slow — keep
  your project inside the WSL filesystem (`~/projects/`) for best performance

## Getting Help

### Debug Mode

Run with debug output:

```bash
# macOS / Linux
DEBUG=anvil:* anvil check --all
```

```powershell
# Windows (PowerShell)
$env:DEBUG="anvil:*"; anvil check --all
```

### Log Collection

Collect logs for bug reports:

```bash
# macOS / Linux
anvil check --all --verbose 2>&1 | tee anvil.log
```

```powershell
# Windows (PowerShell)
anvil check --all --verbose 2>&1 | Tee-Object anvil.log
```

### Filing Issues

Include:

- Anvil version: `anvil --version`
- OS, version, and architecture:
  - macOS / Linux: `uname -a`
  - Windows: `[System.Environment]::OSVersion` and `$env:PROCESSOR_ARCHITECTURE`
    in PowerShell
- Config file (sanitised)
- Error message and stack trace
- Steps to reproduce

File at:
[github.com/EddaCraft/anvil-001/issues](https://github.com/EddaCraft/anvil-001/issues)

---

**Back to:** [Configuration →](/anvil/operations/config)
