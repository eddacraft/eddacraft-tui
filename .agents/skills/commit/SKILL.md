---
name: commit
description: Stage relevant changes and create a well-formed git commit using the repository's documented conventions, falling back to Conventional Commits only when the repository has no commit policy.
---

# Git Commit

Create a git commit for the current changes following these steps.

## 1. Review Changes

```console
git status
git diff --staged
git diff
```

## 2. Stage Changes

- Stage files individually by path (`git add <file> ...`). Never use
  `git add -A` or `git add .`.
- Don't stage files with secrets (.env, credentials, etc.)
- Don't stage generated files unless intentional
- If unrelated changes are already staged, unstage them or ask before
  proceeding
- If the working tree mixes unrelated changes, split them into separate
  commits or ask which to include

## 3. Commit Message Format

Read `AGENTS.md`, contributor documentation, recent accepted commits, and any
commit template first. Follow the repository's documented convention. When the
repository has no convention, use Conventional Commits as the fallback:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `ci`: CI configuration

### Guidelines

- Subject: imperative mood, lowercase, no period, 50 chars max
- Body: wrap at 72 chars, explain what and why (not how)
- Reference issues: `Fixes #123` or `Relates to #456`

## 4. Commit

Write the complete message to a temporary text file using the harness's file
tool, then let Git read it. This preserves formatting without shell-specific
HEREDOC or command-substitution syntax:

```console
git add <files>
git commit --file <message-file>
```

Remove the temporary message file after a successful commit.

### Failure Handling

- Pre-commit hook fails: fix the cause, re-stage (hooks may modify
  files), retry once. Never `--no-verify` without an explicit user
  request (see Rules below).
- Nothing to commit: report it; do not invent changes.

## Rules

- Never commit files containing secrets
- Never bypass hooks (`--no-verify`) unless the user explicitly asks
- Never amend a published commit unless the user explicitly asks
- Prefer multiple atomic commits over one large commit when changes are unrelated

## Output

Report the short hash, subject line, and files committed.
