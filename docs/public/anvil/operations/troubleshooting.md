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

### Curl Installer Says Homebrew Owns Anvil

On macOS/Linux, the curl installer refuses to replace an existing
Homebrew-managed Anvil binary. This prevents two install methods from fighting
over the same command.

**Solution:**

```bash
brew upgrade eddacraft/tap/anvil
```

If you intentionally want the standalone installer instead, uninstall the
Homebrew formula first, then re-run the curl installer.

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

## Daemon and MCP Activation

For the full operator-facing detail, see the
[v0.7.0-beta release runbook](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md).
The most common pitfalls are summarised below.

### `ready_restart_required` after `anvil start --verify`

In `v0.7.1-beta`, `ready_restart_required` is no longer a generic "restart your
editor" dead end. If the daemon attests live enforcement for the current
worktree, `anvil start --verify` and `anvil status --verify` promote to
`protecting`.

If the state remains `ready_restart_required`, follow the repair hint:

- **Editor not restarted yet:** restart Cursor or Claude Code so it picks up the
  MCP config entry.
- **Daemon unreachable:** run `anvil start` in an interactive terminal — it
  auto-starts the per-user daemon (Linux and macOS) — then re-run
  `anvil start --verify`. In a headless session, or on Windows, run
  `anvil intercept start --foreground` in another terminal instead.
- **Worktree unenforced or stale:** run `anvil intercept status` and confirm the
  daemon knows about the repository path you are probing.
- **All surfaces quarantined:** clear the fence where supported with
  `anvil intercept unblock --worktree <PATH>`, or stop and restart the daemon on
  Windows.

Run with `ANVIL_LOG=warn` if you need the daemon-state reason in logs;
actionable activation failures now surface at that level.

### Starting and stopping the daemon

As of `v0.8.2-beta`, on Linux and macOS an interactive `anvil start` auto-starts
the per-user daemon in the background and an interactive `anvil watch` offers to
start one. This is the normal path on those platforms, and a daemon already
running is always reused. `anvil intercept start --foreground` remains the
low-level operator and debugging surface: output goes to the operator's terminal
and the daemon stays attached to the controlling TTY. It is the way to run the
daemon in a headless session, and the only launch mode on Windows until
background launch lands there.

If a start fails with "address already in use" or a stale-PID complaint, a prior
instance is the most likely cause. First ask anvil to stop the recorded daemon:

```bash
anvil intercept stop
```

If the daemon was started in the foreground, or the recorded process cannot be
stopped cleanly, stop it directly:

1. Press Ctrl-C in the controlling terminal of the foreground daemon (sends
   SIGINT to the shutdown handler).
2. If the controlling terminal is gone, send `SIGTERM` to the PID (`kill <PID>`)
   and wait 10 seconds.
3. Escalate to `SIGKILL` (`kill -9 <PID>`) only if SIGTERM did not unwind.

Always pair a `SIGKILL` with a directory cleanup of
`${XDG_RUNTIME_DIR:-$HOME/.local/state}/anvil` to clear the stale socket and PID
file — otherwise the next start refuses on the leftover state.

### `anvil intercept status` is available on every supported target

`anvil intercept status` queries the daemon over the UDS IPC on Unix and over
the named pipe on Windows (via `connect_owner_only_pipe_client`), printing
uptime / sessions / fences / latency on either OS. `--json` returns the same
`DaemonStatusV1` shape on Unix and Windows.

As of `v0.7.1-beta`, the MCP validation path also reaches the daemon on Windows
through owner-only named pipes. If an MCP client reports
`correlation.daemonStatus: unavailable`, use `anvil intercept status` to
distinguish daemon-down, stale, and unenforced-worktree states.

### Fences survive daemon restart

A fenced worktree stays fenced across daemon stop/start, daemon crashes, and
machine reboots. **Restart does not release fences** — that's by design.

On Unix, clear a fenced worktree directly:

```bash
anvil intercept unblock --worktree /absolute/path/to/repo
```

Windows does not support worktree-scoped unblock yet. If every Windows surface
is quarantined, stop any foreground daemon with Ctrl-C in its terminal, then
restart it:

```bash
anvil intercept start --foreground
```

If the daemon state is corrupted or a killed daemon leaves stale local state,
remove `${XDG_DATA_HOME:-$HOME/.local/share}/anvil` before restarting. That
destroys all fence state for the user, so prefer `unblock --worktree` where it
is supported.

### macOS interrupt ladder fences instead of signalling

The macOS implementation of `current_process_start_time` returns `None` in
`v0.6.0-beta`. Per the AD-7 fence-on-failure invariant, every interrupt decision
that needs to verify the leader's start time falls through to a fence. Operators
on macOS should expect:

- Fenced worktrees rather than signal ladders for interrupted sessions
- `anvil intercept status` showing `fenced: true` more often than on Linux
- Recovery via daemon stop + fence-directory removal (no worktree-scoped CLI
  recovery in v1)

This is expected v1 behaviour, tracked outside the release.

### Windows CI regressions on feature branches are silent

The Windows cross-compile job is gated to pushes to `main` and release-class PRs
(head `release/*` or `hotfix/*`). Normal `feat/*` / `fix/*` PRs do not fire the
Windows matrix, so a Windows-only regression on a feature branch is invisible in
CI until merge to `main`. If you're triaging a Windows bug against a pre-merge
artefact, run the Windows test matrix locally before rooting the bug at the
operator's environment:

```bash
cargo test --workspace --target x86_64-pc-windows-msvc -- --test-threads=1
```

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

3. Check whether the file is under Anvil's built-in local-noise ignore policy.
   Watch and audit skip local tool state, agent worktrees, generated folders,
   and caches such as `.claude`, `.opencode`, `.gemini`, `.serena`,
   `.worktrees`, `node_modules`, `target`, and `dist` by default.

4. Increase debounce:

   ```bash
   anvil watch --source --debounce 500
   ```

5. Check file system events:

   ```bash
   # Linux: /proc/sys/fs/inotify/max_user_watches
   cat /proc/sys/fs/inotify/max_user_watches
   # Increase if needed:
   echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf

   # macOS: generally not an issue (uses FSEvents)

   # Windows: generally not an issue (uses ReadDirectoryChangesW)
   ```

### Watch Starts but Does Not Show Existing Findings

The initial watch scan is baseline/readiness state. It builds the graph and
watcher state without treating existing repository contents as new save-time
violations. Save or edit a file to test the live path.

### Watch Falls Back to Plain Output

`anvil watch` opens the TUI only when stdin and stdout are both terminals. In
pipes, CI, redirected output, or non-interactive shells, it falls back to plain
output so the process remains scriptable. Use `--json` for NDJSON output or
`--no-tui` when you want plain output explicitly.

### High CPU Usage

anvil consuming too much CPU.

**Solutions:**

1. Increase debounce time
2. Narrow the watch scope with `--file`, or use `--exclude` with glob patterns
   for generated/build directories
3. Disable `validateOnType` in VS Code
4. Check for circular watch triggers

### Memory Issues

Out of memory errors.

**Solutions:**

- Reduce watch scope with `anvil watch --file <path>`
- Exclude large generated directories with glob patterns (for example
  `anvil watch --source --exclude "node_modules/**,dist/**,.next/**"`)
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

2. If the same finding appears repeatedly, keep a suppression inventory for
   review using `anvil export` outputs or your team's own report, but use inline
   `@anvil-ignore` comments for current scan suppression.

### Missing Issues

anvil not catching problems it should.

**Check:**

```bash
# Verify check is enabled
cat .anvilrc

# Run verbose
anvil --verbose check --all
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

### GitHub Actions Timeout

The workflow step running Anvil is taking too long.

**Solutions:**

```yaml
- name: Run anvil
  timeout-minutes: 10
  run: anvil gate --profile ci
```

Or:

- Enable caching
- Reduce check scope
- Parallelise with matrix

### PR Comments Not Appearing

The Anvil workflow runs, but your custom comment step does not post anything.

**Check:**

- The workflow grants `pull-requests: write` if it posts PR comments
- The comment step runs only on `pull_request` events
- The comment step reads the path where you wrote `anvil --json gate` output

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
anvil --verbose check --all
```

### Log Collection

Collect logs for bug reports:

```bash
# macOS / Linux
anvil --verbose check --all 2>&1 | tee anvil.log
```

```powershell
# Windows (PowerShell)
anvil --verbose check --all 2>&1 | Tee-Object anvil.log
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
