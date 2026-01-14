---
name: code-review
description:
  Comprehensive code review methodology, PR review, quality assessment
---

# Code Review Skill

## Overview

Systematic approach to reviewing code for quality, security, performance, and
maintainability.

## When to Apply

- Pull request reviews
- Pre-commit validation
- Code audits
- Pair programming
- Technical debt assessment

## Review Dimensions

### 1. Correctness

**Questions:**

- Does the code do what it's supposed to?
- Are edge cases handled?
- Is error handling appropriate?
- Are there any obvious bugs?

**Checks:**

```
[ ] Logic is correct
[ ] Boundary conditions handled
[ ] Null/undefined checks present
[ ] Error paths covered
```

### 2. Security

**Questions:**

- Is input validated?
- Are there injection risks?
- Is authentication/authorization correct?
- Are secrets handled properly?

**OWASP Top 10 Checklist:**

```
[ ] No SQL injection vulnerabilities
[ ] No XSS vulnerabilities
[ ] No CSRF vulnerabilities
[ ] No insecure direct object references
[ ] No hardcoded credentials
[ ] Proper authentication
[ ] Proper authorization
[ ] No sensitive data exposure
[ ] No security misconfigurations
[ ] No vulnerable dependencies
```

### 3. Performance

**Questions:**

- Are there obvious inefficiencies?
- Is the algorithm appropriate?
- Are there memory leaks?
- Are resources properly cleaned up?

**Checks:**

```
[ ] Efficient algorithms used
[ ] No unnecessary loops
[ ] Proper caching where needed
[ ] Resources released
[ ] No N+1 queries
```

### 4. Maintainability

**Questions:**

- Is the code readable?
- Is it well-structured?
- Are names meaningful?
- Is complexity appropriate?

**Checks:**

```
[ ] Clear naming conventions
[ ] Appropriate comments
[ ] No code duplication
[ ] Single responsibility
[ ] Reasonable function length
```

### 5. Testing

**Questions:**

- Is there adequate test coverage?
- Do tests verify the right things?
- Are edge cases tested?

**Checks:**

```
[ ] Unit tests exist
[ ] Tests are meaningful
[ ] Edge cases covered
[ ] Tests are maintainable
```

## Review Process

### Step 1: Understand Context

```
1. Read PR description
2. Understand the goal
3. Check related issues
4. Review overall approach
```

### Step 2: High-Level Review

```
1. Scan all changed files
2. Understand the structure
3. Identify key changes
4. Note areas of concern
```

### Step 3: Detailed Review

```
For each file:
1. Read line by line
2. Check each dimension
3. Note specific issues
4. Suggest improvements
```

### Step 4: Integration Review

```
1. Check interactions
2. Verify consistency
3. Test manually if needed
4. Run automated checks
```

## Feedback Format

### Severity Levels

```
CRITICAL: Must fix before merge
  - Security vulnerabilities
  - Data loss risks
  - Breaking changes

MAJOR: Should fix
  - Bugs
  - Performance issues
  - Significant maintainability concerns

MINOR: Nice to fix
  - Style improvements
  - Minor refactoring
  - Documentation gaps

NIT: Optional
  - Personal preferences
  - Micro-optimizations
```

### Comment Format

```markdown
**[SEVERITY]** file.ts:42

Issue description

Suggestion: \`\`\`typescript // Recommended code \`\`\`

Why: Explanation of the improvement
```

## Review Output Template

```markdown
## Code Review: PR #123

### Summary

Brief overview of changes and overall assessment

### Decision: APPROVED / NEEDS_CHANGES / REJECTED

### Critical Issues

1. [CRITICAL] file:line - Description

### Major Suggestions

1. [MAJOR] file:line - Description

### Minor Notes

1. [MINOR] file:line - Description

### Positive Observations

- Well-implemented aspects

### Questions

- Clarifications needed
```
