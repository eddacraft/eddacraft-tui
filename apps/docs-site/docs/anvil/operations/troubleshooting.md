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

The CLI isn't in your PATH.

**Solution (global install):**
```bash
pnpm add -g @anvil/cli
# or
npm install -g @anvil/cli
```

**Solution (local install):**
```bash
pnpm anvil run
# or
npx anvil run
```

### "Cannot find module '@anvil/cli'"

Dependencies not installed.

**Solution:**
```bash
pnpm install
```

### Node Version Error

Anvil requires Node.js 20+.

**Solution:**
```bash
node --version  # Check version
nvm use 20      # Switch to Node 20
```

## Configuration Issues

### "Configuration file not found"

No `anvil.config.json` in project root.

**Solution:**
```bash
anvil init
```

### "Invalid configuration"

Config syntax error.

**Solution:**
```bash
# Validate config
anvil config validate

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
anvil debug boundaries src/api/users.ts
```

**Common issues:**
- Glob pattern mismatch (`src/api` vs `src/api/**`)
- Wrong path separator on Windows
- Pattern doesn't include file extension

## Runtime Issues

### Watch Mode Not Detecting Changes

Files changing but Anvil not responding.

**Solutions:**
1. Check ignore patterns:
   ```bash
   anvil config show | grep ignore
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
   # macOS: fs.inotify limits
   # Linux: /proc/sys/fs/inotify/max_user_watches
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
     "suppressions": [{
       "pattern": "src/types/external.ts",
       "checks": ["AP-003"],
       "reason": "External type definitions"
     }]
   }
   ```

### Missing Issues

Anvil not catching problems it should.

**Check:**
```bash
# Verify check is enabled
anvil config show | grep -A5 antiPatterns

# Run verbose
anvil run --verbose
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
- `--ci` flag is present
- `failOnWarnings` setting if using warnings
- Config is being read (check logs)

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

## Getting Help

### Debug Mode

Run with debug output:
```bash
DEBUG=anvil:* anvil run
```

### Log Collection

Collect logs for bug reports:
```bash
anvil run --verbose 2>&1 | tee anvil.log
```

### Filing Issues

Include:
- Anvil version: `anvil --version`
- Node version: `node --version`
- OS and version
- Config file (sanitised)
- Error message and stack trace
- Steps to reproduce

File at: [github.com/EddaCraft/anvil-001/issues](https://github.com/EddaCraft/anvil-001/issues)

---

**Back to:** [Configuration →](/docs/anvil/operations/config)
