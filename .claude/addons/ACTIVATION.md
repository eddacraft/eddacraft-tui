# Addon Activation Guide

This guide explains how to activate and use addons in your Claude Code projects.

## Quick Start (90% of users should use this)

### Step 1: Choose your addons

Browse available addons in `.claude/addons/`:

**Hook Addons** (in `.claude/addons/hooks/`):

- `hooks/safety` - Universal safety checks (recommended for all projects)
- `hooks/node-typescript` - Node.js/TypeScript formatting, linting, and type
  checking
- `hooks/python-modern` - Python formatting with Ruff, Black, and mypy
- `hooks/agent-quality` - Quality gates for specific Claude agents (coder,
  reviewer, security-auditor)
- `hooks/git-workflow` - Git commit standards and PR helpers

**Command Addons** (in `.claude/addons/commands/`):

- `git-workflow` - Git workflow commands (commit, create-pr, changelog)
- `repository` - Repository management commands (prime-repo, prime-docs)

### Step 2: Create activation file

#### For Node.js/TypeScript Projects

```bash
echo '{"addons": ["hooks/safety", "hooks/node-typescript"]}' > .claude-local/active-addons.json
```

#### For Python Projects

```bash
echo '{"addons": ["hooks/safety", "hooks/python-modern"]}' > .claude-local/active-addons.json
```

#### For Any Project (Safety Only)

```bash
echo '{"addons": ["hooks/safety"]}' > .claude-local/active-addons.json
```

#### With Git Workflow Commands

```bash
echo '{"addons": ["hooks/safety", "hooks/git-workflow", "commands/git-workflow"]}' > .claude-local/active-addons.json
```

#### With Repository Commands

```bash
echo '{"addons": ["hooks/safety", "commands/repository"]}' > .claude-local/active-addons.json
```

### Step 3: Verify activation

Ask Claude: "What addons are active?" or "Test my hooks" to confirm they're
working.

## That's It! 🎉

Your hooks will now automatically run when you use Claude Code. For example:

- When you edit a TypeScript file, it will be formatted with Prettier
- When you commit, tests will run automatically
- When you use destructive commands, you'll see safety warnings

---

## Advanced Usage (Optional)

### Using Multiple Addons Together

Addons are designed to work together. Common combinations:

**Full Node.js Stack with Quality Gates:**

```json
{
  "addons": [
    "hooks/safety",
    "hooks/node-typescript",
    "hooks/agent-quality",
    "hooks/git-workflow",
    "commands/git-workflow",
    "commands/repository"
  ]
}
```

**Python with Security Focus:**

```json
{
  "addons": ["hooks/safety", "hooks/python-modern", "hooks/agent-quality"]
}
```

**Minimal with Agent Support:**

```json
{
  "addons": ["hooks/safety", "hooks/agent-quality"]
}
```

### Custom Addons

#### Adding Your Own Hooks

Create `.claude-local/custom-hooks.json`:

```json
{
  "extends": ["hooks/node-typescript"],
  "hooks": {
    "post-tool-use": [
      {
        "name": "my-custom-check",
        "matcher": {
          "tool": "Edit",
          "file_patterns": ["*.tsx"]
        },
        "command": "echo 'Running my custom check'",
        "timeout": 5000
      }
    ]
  }
}
```

Then reference it:

```json
{
  "addons": ["hooks/safety", ".claude-local/custom-hooks"]
}
```

### Environment-Based Profiles

Create different profiles for different environments:

`.claude-local/profiles/development.json`:

```json
{
  "addons": ["hooks/safety", "hooks/node-typescript", "commands"]
}
```

`.claude-local/profiles/production.json`:

```json
{
  "addons": ["hooks/safety"]
}
```

Activate with:

```bash
cp .claude-local/profiles/development.json .claude-local/active-addons.json
```

### Monorepo Setup

```json
{
  "addons": ["hooks/safety", "hooks/node-typescript", "custom/monorepo-tools"],
  "addon_config": {
    "workspace_paths": ["packages/*", "apps/*"]
  }
}
```

### CI/CD Integration

```json
{
  "addons": ["hooks/safety", "hooks/git-workflow", "custom/ci-checks"]
}
```

### Minimal Setup

```json
{
  "addons": ["hooks/safety"]
}
```

## Disabling Addons

### Temporarily Disable All

```bash
mv .claude-local/active-addons.json .claude-local/active-addons.json.bak
```

### Disable Specific Addon

Edit `.claude-local/active-addons.json` and remove the addon from the list.

### Override Specific Hooks

Create `.claude-local/overrides.json`:

```json
{
  "disabled_hooks": ["typescript-check", "test-before-commit"]
}
```

## Verification

Ask Claude to verify your addon configuration:

- "What addons are currently active?"
- "Show me the active hooks"
- "Test the safety hooks"

## Troubleshooting

### Hooks Not Running

1. Check if file exists: `cat .claude-local/active-addons.json`
2. Verify addon path is correct
3. Check if required files exist (see addon.json)
4. Look for error messages in Claude's output

### Performance Issues

1. Reduce timeout values in hooks
2. Disable expensive checks (like build-check)
3. Use fewer addons simultaneously

### Conflicts Between Addons

1. Check for duplicate hook names
2. Ensure only one formatter per language
3. Order matters - last addon wins for conflicts

## Best Practices

1. **Start Simple**: Begin with just `safety` hooks
2. **Add Gradually**: Add one addon at a time
3. **Project-Specific**: Use `.claude-local/` for project preferences
4. **Document**: Note which addons your project uses in README
5. **Test**: Verify hooks work with your specific tooling
6. **Customize**: Create project-specific hooks as needed
