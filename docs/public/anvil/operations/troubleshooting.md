---
id: troubleshooting
title: Troubleshooting
description: Common issues and how to resolve them.
sidebar_position: 3
---

# Troubleshooting

Common issues and solutions for anvil.

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

If you installed via Scoop, run `scoop reset anvil`.

**Windows (PowerShell):**

```powershell
# Re-run the installer
irm https://install.eddacraft.ai/windows | iex

# Or add the install directory to your PATH manually
$env:Path = "$env:USERPROFILE\.eddacraft\bin;$env:Path"

# To persist across sessions, add to your profile:
Add-Content $PROFILE '$env:Path = "$env:USERPROFILE\.eddacraft\bin;$env:Path"'
```

### Installer Completed but `anvil` Still Fails

If the installer finished successfully but `anvil` still does not run:

- confirm `~/.eddacraft/bin` (macOS/Linux) or `%USERPROFILE%\.eddacraft\bin`
  (Windows) is on your `PATH`
- open a new terminal session after install
- run `anvil --version` to confirm the binary resolves
- if needed, re-run the installer to replace a partial or stale binary

See [The Switch to Rust](/anvil/releases/rust-rewrite) for background on the
Rust binary.

### Updater Not Finding a New Release

If `anvil update` reports that you are already current but you expect a newer
beta:

```bash
anvil --version
anvil update
```

If the updater still cannot see the release:

- re-run the install script to pick up the latest published artefact
- verify that the GitHub release exists for your platform
- on Windows, try `winget upgrade eddacraft.anvil` or `scoop update anvil`

## Diagnostics & AI Guardrail Issues

### `anvil doctor` warns "git-repo: not a git repository"

Running `anvil doctor` outside a Git repository now produces a structured
warning rather than failing the whole run. This is expected when you point anvil
at a non-Git directory.

**Solutions:**

- If the directory is intentionally not version-controlled (a scratch area,
  artefact dump, or playground), the warning is informational and can be
  ignored.
- If the directory _should_ be a Git repository, run `git init` and re-run
  `anvil doctor`.
- If you're scripting against `anvil doctor --json`, branch on
  `notifications[].class` rather than the process exit code; the warning is
  surfaced as a notification and does not fail the run.

### `anvil gate --profile ai` Fails with a Configuration Error

The AI guardrail profile treats missing or invalid governance configuration as
blocking on purpose, so agent and MCP consumers see a deterministic error rather
than silently skipping checks.

**Solutions:**

- Inspect the JSON envelope: `anvil gate --profile ai` defaults to JSON, and the
  diagnostic payload identifies the specific config key or file that failed
  validation.
- Make sure your project has the AI guardrail config in place. Run
  `anvil doctor` to confirm `.anvilrc` and supporting config files parse
  cleanly.
- If you only need the legacy gate behaviour for now, run
  `anvil gate --profile ci` instead while you fix the AI config.

### `anvil mcp-config --write` Prompts Before Overwriting

`anvil mcp-config --write` prompts before overwriting an existing client
configuration. This is the path-safety layer; it is not a bug.

**Solutions:**

- Review the diff against your existing config, then confirm the prompt to apply
  the atomic write.
- In CI or non-interactive environments, run `anvil mcp-config --verify` instead
  so the command exits non-zero on drift without trying to write.
- Use `--workspace <path>` if the auto-detected workspace root is not the
  project you want recorded in the generated config.

### Scan Threads Saturating CPU

By default, first-run scans, `check`, `gate`, and `audit` cap their thread pool
at `min(num_cpus, 4)` so the parallel walk does not starve TUI, editor, or watch
processes.

**Solutions:**

- On a dedicated CI runner, raise the cap with `ANVIL_SCAN_THREADS=8` (or the
  number of cores you want to dedicate).
- On a laptop, leave the default in place; if you still see contention, set
  `ANVIL_SCAN_THREADS=2` or `1` to fall back to a smaller pool.

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

Files changing but anvil not responding.

**Solutions:**

1. Check the watch scope you started:

   ```bash
   anvil watch --source
   anvil watch --plans
   anvil watch --all
   ```

2. If you passed `--exclude`, use glob patterns such as `dist/**` or
   `node_modules/**`. Bare names only match the exact path.

3. Increase debounce:

   ```bash
   anvil watch --source --debounce 500
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

anvil consuming too much CPU.

**Solutions:**

1. Increase debounce time
2. Narrow the watch scope with `--file`, or use `--exclude` for obvious
   generated/build directories by name
3. Disable `validateOnType` in VS Code
4. Check for circular watch triggers

### Memory Issues

Out of memory errors.

**Solutions:**

- Reduce watch scope with `anvil watch --file <path>`
- Exclude large generated directories by name with `--exclude` (for example
  `node_modules,dist,.next`)
- Check for very large files being scanned
- Check `inotify` limits on Linux (see File Watching section above)
- If RSS exceeds expected bounds (~30-50MB for a medium project), file a bug

## Gate Check Issues

### False Positives

anvil flagging code that's actually fine.

**Solutions:**

1. Add specific suppression with explanation:

   ```typescript
   // @anvil-ignore AP-003 Using any for JSON.parse result
   ```

2. Add pattern suppression in `.anvil/suppressions.json`:

   ```json
   {
     "suppressions": [
       {
         "pattern_id": "AP-003",
         "file": "src/types/external.ts",
         "reason": "External type definitions",
         "scope": "file",
         "expires_at": "2026-07-01T00:00:00Z"
       }
     ]
   }
   ```

   `expires_at` is optional — omit it for a permanent suppression. Additional
   fields are ignored by the parser.

### Missing Issues

anvil not catching problems it should.

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

anvil passing when it should fail.

**Check:**

- Prefer `anvil gate --profile ci` so CI-specific profile settings are applied
  (bare `anvil gate` runs all checks by default)
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

anvil uses forward slashes (`/`) for glob patterns on all platforms. Do not use
backslashes in `.anvilrc` boundary patterns, even on Windows:

```json
{
  "pattern": "src/api/**"
}
```

Using `src\\api\\**` will not match.

### Antivirus Interference

Some antivirus software (Windows Defender, Norton, etc.) can slow down watch
mode by scanning files that anvil accesses. If watch mode is unusually slow:

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

anvil can work with long paths (> 260 characters) on Windows as long as Windows
long-path support is enabled and your tooling supports it. If you encounter
path-related errors in deeply nested `node_modules`:

1. Enable long paths in Windows (requires admin):
   ```powershell
   New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" `
     -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
   ```
2. Restart your terminal

### WSL vs Native Windows

anvil works in both native Windows (PowerShell/cmd) and WSL. If using WSL:

- Use the Linux installer (`curl ... | sh`), not the Windows PowerShell one
- File watching across the WSL/Windows boundary (e.g. `/mnt/c/`) is slow — keep
  your project inside the WSL filesystem (`~/projects/`) for best performance

## Getting Help

### Debug Mode

Run with verbose output:

```bash
anvil check --all --verbose
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

- anvil version: `anvil --version`
- OS, version, and architecture:
  - macOS / Linux: `uname -a`
  - Windows: `[System.Environment]::OSVersion` and `$env:PROCESSOR_ARCHITECTURE`
    in PowerShell
- Config file (sanitised)
- Error message and stack trace
- Steps to reproduce

File at:
[github.com/eddacraft/anvil/issues](https://github.com/eddacraft/anvil/issues)

---

**Previous:** [Security model](/anvil/operations/security) | **See also:**
[Configuration reference](/anvil/operations/config)
