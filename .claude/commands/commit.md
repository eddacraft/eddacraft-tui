---
description: Prepare a Conventional Commit message from staged changes
---

# Commit Message Generation

Create well-structured Conventional Commit messages from staged changes:

1. **Change Analysis** - Review staged files and understand modifications
2. **Type Classification** - Determine commit type (feat, fix, docs, style,
   refactor, test, chore)
3. **Scope Identification** - Extract relevant scope from changed files
4. **Message Crafting** - Create concise subject line following conventions
5. **Body & Footer** - Add context, breaking changes, and issue references as
   needed

## Agent Sequence

- **reviewer**: Analyzes staged changes and determines commit type, scope, and
  impact
- **docs-writer**: Crafts the final commit message following Conventional Commit
  standards

## Usage

```
/commit
```

## Output Format

```
type(scope): subject

body (optional)

BREAKING CHANGE: description (if applicable)
Closes #123 (if applicable)
```

Produces ready-to-paste git commit text following project conventions and best
practices.
