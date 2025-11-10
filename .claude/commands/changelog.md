---
description: Generate a release changelog since the last tag
---

# Changelog Generation

Create comprehensive release notes from Git history:

1. **Change Collection** - Gather commits since last tag or specified reference
2. **Categorization** - Group changes by type (feat, fix, docs, chore, refactor)
3. **Highlights Extraction** - Identify breaking changes and notable features
4. **Upgrade Notes** - Document migration requirements and impacts
5. **Credits Attribution** - Acknowledge contributors and issue references

## Agent Sequence

- **reviewer**: Analyzes commit history, categorizes changes, and identifies
  breaking changes
- **docs-writer**: Formats changelog with highlights, upgrade notes, and proper
  attribution

## Usage

```
/changelog
/changelog v1.0.0..HEAD
/changelog --since="2024-01-01"
```

## Output Format

```markdown
## [Version] - YYYY-MM-DD

### Highlights

- Major feature or breaking change
- Notable improvement

### Features

- feat: description (#123)
- feat(scope): description

### Bug Fixes

- fix: description (#456)
- fix(scope): description

### Documentation

- docs: description

### Maintenance

- chore: description
- refactor: description

### Breaking Changes

- Description of breaking change
- Migration instructions

### Upgrade Notes

- Step-by-step upgrade guide
- Compatibility notes

### Credits

Thanks to @contributor1, @contributor2 for their contributions!
```

Generates release-ready changelog following Keep a Changelog format with
semantic versioning.
