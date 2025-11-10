---
description: Scan and improve documentation quality across the repository
---

# Prime Documentation

Comprehensive documentation review and improvement workflow:

1. **Documentation Scan** - Analyze README, templates, ADRs, and runbooks
2. **Gap Analysis** - Identify missing sections, outdated content, and
   inconsistencies
3. **Quality Review** - Check clarity, completeness, and maintainability
4. **Improvement Plan** - Propose fixes and enhancements with priority
5. **Implementation Guide** - Provide diffs or outlines for documentation
   updates

## Agent Sequence

- **docs-writer**: Scans all documentation, identifies gaps, and proposes
  improvements
- **reviewer**: Reviews proposed changes for clarity, accuracy, and completeness

## Usage

```
/prime-docs
```

Produces actionable documentation improvements with ready-to-apply diffs or
detailed outlines for the docs-writer to implement.
