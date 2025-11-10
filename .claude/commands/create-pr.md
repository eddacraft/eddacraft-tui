---
description: Draft a pull request title and body with complete context
---

# Pull Request Creation

Generate comprehensive pull request documentation:

1. **Change Summary** - Analyze branch changes and create concise title
2. **Context Building** - Document what, why, and how of the changes
3. **Risk Assessment** - Identify potential impacts and breaking changes
4. **Test Coverage** - Document testing approach and validation steps
5. **Review Checklist** - Create actionable review items and merge criteria

## Agent Sequence

- **reviewer**: Analyzes changes, identifies risks, and suggests review focus
  areas
- **tester**: Documents test coverage and validation requirements
- **docs-writer**: Crafts the final PR title and body with all sections

## Usage

```
/create-pr
```

## Output Format

```bash
gh pr create -t "<title>" -b "<body>" --label "<labels>"
```

### Body Template

```markdown
## What

[Summary of changes]

## Why

[Motivation and context]

## How

[Implementation approach]

## Risks

[Potential impacts and mitigations]

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests passing
- [ ] Manual testing completed
- [ ] Performance impact assessed

## Checklist

- [ ] Code follows project conventions
- [ ] Documentation updated
- [ ] Breaking changes documented
- [ ] Security review completed
```

Produces ready-to-execute GitHub CLI command with comprehensive PR
documentation.
