# Pull Request Templates Reference

This file provides templates and examples for creating comprehensive pull
requests. Load this when crafting PR descriptions.

## Standard PR Structure

```markdown
## What

[Concise summary of changes - 2-3 sentences]

## Why

[Motivation and context - what problem does this solve?]

## How

[Implementation approach and key technical decisions]

## Risks

[Potential impacts, edge cases, or concerns]

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

## Template Variations

### 1. Feature PR Template

```markdown
## What

Implements [feature name] that allows users to [capability].

## Why

Users/developers currently [pain point]. This feature addresses that by
[solution approach].

Closes #[issue-number]

## How

### Architecture

- [Component/module] handles [responsibility]
- [Component/module] manages [responsibility]
- Data flow: [brief description]

### Key Technical Decisions

1. **[Decision]**: [Rationale]
2. **[Decision]**: [Rationale]

### File Structure
```

src/ feature/ FeatureComponent.tsx # Main UI component FeatureService.ts #
Business logic types.ts # Type definitions tests/ feature/
FeatureComponent.test.tsx FeatureService.test.ts

```

## Implementation Details

### [Subsystem 1]

[Explanation of implementation]

### [Subsystem 2]

[Explanation of implementation]

## Risks

**High Risk:**
- [Risk]: [Mitigation]

**Medium Risk:**
- [Risk]: [Mitigation]

**Low Risk:**
- [Risk]: [Mitigation]

## Performance Impact

- [Metric]: [Before] → [After]
- Bundle size: [Change]
- API response time: [Change]

## Testing

### Unit Tests

- [ ] [Component] - [test scenarios]
- [ ] [Service] - [test scenarios]
- [ ] Edge cases covered

### Integration Tests

- [ ] [Workflow] tested end-to-end
- [ ] Error scenarios validated
- [ ] State management verified

### Manual Testing

- [ ] Tested in Chrome, Firefox, Safari
- [ ] Tested on mobile devices
- [ ] Tested with screen reader
- [ ] Verified keyboard navigation

### Test Coverage

```

Statements : 95% ( 200/210 ) Branches : 90% ( 45/50 ) Functions : 100% ( 30/30 )
Lines : 95% ( 190/200 )

```

## Screenshots/Videos

### Before

[Screenshot of old behavior]

### After

[Screenshot of new behavior]

### Demo

[Link to video or GIF demonstrating the feature]

## Documentation

- [ ] README updated
- [ ] API documentation updated
- [ ] Inline code comments added
- [ ] Architecture decision recorded (ADR)

## Migration Guide

N/A - No breaking changes

## Checklist

- [ ] Code follows project style guide
- [ ] All tests passing
- [ ] No console errors or warnings
- [ ] Accessibility requirements met (WCAG 2.1 AA)
- [ ] Security review completed
- [ ] Performance benchmarks met
- [ ] Database migrations tested (if applicable)

## Reviewers

@reviewer1 - For [specific aspect]
@reviewer2 - For [specific aspect]
```

### 2. Bug Fix PR Template

```markdown
## What

Fixes [bug description] where [symptom].

Fixes #[issue-number]

## Why

### Problem

Users experience [issue] when [scenario]. This causes [impact].

### Root Cause

[Technical explanation of what was wrong]

## How

### Solution

[Explanation of fix]

### Alternative Approaches Considered

1. **[Approach]**: Rejected because [reason]
2. **[Approach]**: Rejected because [reason]

## Risks

**Regression Risk:** [Low/Medium/High]

- [Specific concern and mitigation]

**Deployment Risk:** [Low/Medium/High]

- [Specific concern and mitigation]

## Testing

### Reproduction Steps (Before Fix)

1. [Step]
2. [Step]
3. **Expected:** [What should happen]
4. **Actual:** [What went wrong]

### Verification Steps (After Fix)

1. [Step]
2. [Step]
3. **Result:** [What happens now]

### Test Cases

- [ ] Original bug scenario
- [ ] Edge case: [scenario]
- [ ] Edge case: [scenario]
- [ ] Regression: Existing functionality still works

### Automated Tests

- [ ] Unit test covering the bug scenario
- [ ] Integration test for end-to-end flow
- [ ] Test for edge cases

## Checklist

- [ ] Bug is reproducible before fix
- [ ] Bug is fixed after change
- [ ] No regressions introduced
- [ ] Tests added to prevent recurrence
- [ ] Documentation updated (if applicable)
```

### 3. Refactoring PR Template

````markdown
## What

Refactors [component/module] to [improvement goal].

## Why

### Current State

[Description of current implementation and its problems]

Problems:

- [Problem 1]
- [Problem 2]
- [Problem 3]

### Desired State

[Description of improved implementation]

Benefits:

- [Benefit 1]
- [Benefit 2]
- [Benefit 3]

## How

### Changes

1. **[Change category]**
   - [Specific change]
   - [Specific change]

2. **[Change category]**
   - [Specific change]
   - [Specific change]

### Before/After Comparison

**Before:**

```typescript
// Old implementation
```
````

**After:**

```typescript
// New implementation
```

### Behavior Preservation

**Guaranteed:** No behavior changes

- [Evidence/test coverage]

**Changed:** [Any intentional behavior changes]

- [Specific change and justification]

## Risks

**Code Review Complexity:** [Assessment]

- [Large PR mitigation strategy]

**Merge Conflicts:** [Assessment]

- [Coordination with other PRs]

## Testing

### Test Strategy

✅ **All existing tests passing**

- Confirms behavior preservation

### Additional Tests

- [ ] [New test for previously uncovered scenario]
- [ ] [New test for edge case]

### Performance

- [ ] No performance regression
- [ ] [Specific improvements if applicable]

## Checklist

- [ ] No behavior changes (or documented if intentional)
- [ ] All tests passing
- [ ] Code complexity reduced
- [ ] Maintainability improved
- [ ] No new technical debt introduced

````

### 4. Documentation PR Template

```markdown
## What

Updates documentation for [topic/component].

## Why

- [Current state of documentation]
- [Gap or improvement needed]

## Changes

### Added

- [New section/content]
- [New example]

### Updated

- [Modified section and why]
- [Corrected information]

### Removed

- [Outdated content]
- [Redundant information]

## Checklist

- [ ] Technical accuracy verified
- [ ] Links tested and working
- [ ] Code examples tested
- [ ] Screenshots updated (if applicable)
- [ ] Spelling and grammar checked
- [ ] Follows documentation style guide
````

### 5. Breaking Change PR Template

````markdown
## What

**⚠️ BREAKING CHANGE:** [Summary of breaking change]

## Why

### Motivation

[Why this breaking change is necessary]

### Benefits

- [Benefit 1]
- [Benefit 2]

### Costs

- [Migration effort required]
- [Affected users/systems]

## How

### What's Changing

**Before:**

```typescript
// Old API/behavior
```
````

**After:**

```typescript
// New API/behavior
```

### Breaking Changes

1. **[Change 1]**
   - **Impact:** [Who/what is affected]
   - **Migration:** [How to update]

2. **[Change 2]**
   - **Impact:** [Who/what is affected]
   - **Migration:** [How to update]

## Migration Guide

### Step-by-Step

1. **[Step]**

   ```bash
   # Commands or code changes
   ```

2. **[Step]**

   ```typescript
   // Example migration
   ```

3. **[Step]**
   - [Instructions]

### Compatibility

- **Minimum version:** [version]
- **Deprecation period:** [timeline]
- **Support for old API:** [None/Limited/Until version X]

### Automated Migration

```bash
# If applicable
npm run migrate
```

## Risks

**High Risk:**

- [Major migration effort]
- [Mitigation: Comprehensive guide, examples, support]

**Medium Risk:**

- [Partial compatibility issues]
- [Mitigation: Detection tools, warnings]

## Testing

### Backward Compatibility

- [ ] Old code fails with clear error messages
- [ ] Migration guide tested on sample projects
- [ ] All examples in docs updated

### New Functionality

- [ ] Full test coverage of new API
- [ ] Integration tests passing
- [ ] Performance validated

## Communication Plan

- [ ] Release notes drafted
- [ ] Migration guide published
- [ ] Blog post prepared (if major)
- [ ] Users notified via [channel]
- [ ] Deprecated API warnings in place

## Checklist

- [ ] All breaking changes documented
- [ ] Migration guide complete and tested
- [ ] Version bumped appropriately (major version)
- [ ] CHANGELOG updated
- [ ] Deprecation warnings added (if gradual migration)
- [ ] Team/community notified

````

### 6. Security Fix PR Template

```markdown
## What

**🔒 Security Fix:** [Brief description]

## Why

### Vulnerability

**Type:** [XSS/SQL Injection/CSRF/etc.]
**Severity:** [Critical/High/Medium/Low]
**CVSS Score:** [If applicable]

**Affected Versions:** [version range]

### Impact

- [What an attacker could do]
- [What data/functionality is at risk]

### Discovery

- Reported by: [Person/Team/Scanning tool]
- Date: [YYYY-MM-DD]
- Reference: [CVE number if applicable]

## How

### Fix

[Technical explanation of the fix]

### Before (Vulnerable):

```typescript
// Vulnerable code (if safe to share)
````

### After (Secure):

```typescript
// Fixed code
```

## Testing

- [ ] Exploit attempt fails with fix in place
- [ ] Existing functionality still works
- [ ] Security test added to prevent regression
- [ ] Penetration testing completed (if applicable)

## Disclosure

**Coordinated Disclosure:** [Yes/No] **Public Disclosure Date:** [YYYY-MM-DD]
**Advisory:** [Link when published]

## Checklist

- [ ] Vulnerability confirmed and understood
- [ ] Fix validated by security team
- [ ] No information leaked in commit history
- [ ] Security advisory prepared
- [ ] Affected users will be notified
- [ ] CVE requested (if applicable)

````

### 7. Performance Improvement PR Template

```markdown
## What

Improves performance of [component/operation] by [percentage/magnitude].

## Why

### Current Performance

- [Metric]: [Current value]
- **Problem:** [Description]

### Target Performance

- [Metric]: [Target value]
- **Goal:** [Description]

## How

### Optimization Strategy

[Explanation of approach]

### Changes

1. **[Optimization 1]**
   - [Technical details]
   - Impact: [Improvement]

2. **[Optimization 2]**
   - [Technical details]
   - Impact: [Improvement]

## Benchmarks

### Before

````

Operation: [name] Average: [time] P50: [time] P95: [time] P99: [time]

```

### After

```

Operation: [name] Average: [time] (↓ [percentage]) P50: [time] (↓ [percentage])
P95: [time] (↓ [percentage]) P99: [time] (↓ [percentage])

```

### Methodology

- **Tool:** [Benchmark tool used]
- **Dataset:** [Test data description]
- **Environment:** [Hardware/environment specs]
- **Runs:** [Number of iterations]

## Risks

**Code Complexity:** [Assessment]
- [How optimization affects maintainability]

**Memory Usage:** [Assessment]
- [Trade-offs between speed and memory]

## Testing

- [ ] Benchmarks confirm improvement
- [ ] All existing tests pass
- [ ] Edge cases still handled correctly
- [ ] No memory leaks introduced

## Checklist

- [ ] Performance improvement validated
- [ ] No regressions in functionality
- [ ] Code remains maintainable
- [ ] Documentation updated with performance characteristics
```

## GitHub CLI Command Format

### Basic PR

```bash
gh pr create \
  --title "feat(scope): description" \
  --body "$(cat <<'EOF'
[PR body from template]
EOF
)"
```

### PR with Labels and Reviewers

```bash
gh pr create \
  --title "feat(api): add user endpoints" \
  --body "$(cat <<'EOF'
[PR body]
EOF
)" \
  --label "enhancement,needs-review" \
  --reviewer "@user1,@user2" \
  --assignee "@me"
```

### Draft PR

```bash
gh pr create \
  --title "feat(wip): implement feature X" \
  --body "$(cat <<'EOF'
[PR body]
EOF
)" \
  --draft
```

## Best Practices

### 1. Title Format

Always use Conventional Commit format:

```
type(scope): description
```

Examples:

```
feat(api): add OAuth2 authentication
fix(ui): resolve mobile layout issues
docs(readme): update installation guide
```

### 2. Summary Section

Keep "What" concise (2-3 sentences) but complete:

- What changed
- High-level impact
- Related issues

### 3. Context is King

"Why" section should answer:

- What problem exists?
- Why is this solution chosen?
- What alternatives were considered?

### 4. Implementation Details

"How" section should:

- Explain non-obvious technical decisions
- Document architecture changes
- Provide code examples for complex logic

### 5. Risk Assessment

Be honest about risks:

- Potential for bugs
- Performance impacts
- Breaking changes
- Dependencies

### 6. Testing Coverage

Demonstrate thoroughness:

- Unit tests
- Integration tests
- Manual testing steps
- Edge cases covered

### 7. Visual Aids

Include when relevant:

- Screenshots for UI changes
- Videos/GIFs for interactions
- Architecture diagrams for complex changes
- Performance graphs for optimizations

### 8. Migration Guides

For breaking changes:

- Step-by-step instructions
- Before/after code examples
- Compatibility notes
- Timeline for deprecation

### 9. Checklist Hygiene

Make checklists:

- Actionable
- Complete before submitting
- Relevant to the PR type

### 10. Request Specific Reviews

Tag reviewers with context:

```
## Reviewers

@security-team - Please review authentication changes
@frontend-team - Review UI components
@tech-lead - Architecture decision approval
```

## Common Mistakes to Avoid

❌ **Vague titles**: "Update code", "Fix bug" ✅ **Specific titles**:
"fix(auth): prevent session fixation", "feat(api): add rate limiting"

❌ **No context**: Just code changes ✅ **Full context**: What, why, how, risks,
testing

❌ **Missing tests**: "Will add tests later" ✅ **Tests included**:
Comprehensive coverage

❌ **Massive PRs**: 50+ files changed ✅ **Focused PRs**: Single logical change

❌ **No screenshots**: UI changes without visuals ✅ **Screenshots**:
Before/after comparisons

❌ **Breaking changes buried**: Mentioned in passing ✅ **Breaking changes
highlighted**: ⚠️ warnings, migration guides

## Tips

1. **Write PR description before coding** - Clarifies intent
2. **Update description as you work** - Capture decisions
3. **Review your own PR first** - Catch obvious issues
4. **Link related PRs and issues** - Create traceability
5. **Use draft PRs for WIP** - Get early feedback
6. **Keep PRs focused** - Easier to review and merge
7. **Respond to all review comments** - Even if just acknowledging
8. **Update based on feedback** - Show you've addressed concerns

---

**Reference Version:** 1.0 **Last Updated:** 2025-11-08
