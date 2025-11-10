# Conventional Commit Patterns Reference

This file provides detailed reference for Conventional Commit types, scopes, and
formatting. Load this when you need specific examples or edge cases.

## Commit Structure

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

## Commit Types

### feat - New Features

**When to use:** Adding new functionality that users/developers can access

**Examples:**

```
feat(api): add user authentication endpoint
feat(ui): add dark mode toggle
feat(cli): add --verbose flag for debugging
feat(db): add user preferences table
```

**With breaking changes:**

```
feat(api)!: change authentication to JWT

BREAKING CHANGE: Cookie-based auth removed.
Migrate to JWT tokens via /auth/token endpoint.
```

### fix - Bug Fixes

**When to use:** Correcting existing functionality that wasn't working as
intended

**Examples:**

```
fix(auth): prevent token expiration edge case
fix(ui): correct button alignment in mobile view
fix(api): handle null values in user profile
fix(build): resolve TypeScript compilation error
```

**With issue reference:**

```
fix(parser): handle malformed JSON input

Properly catch and report JSON parsing errors instead of crashing.

Fixes #456
```

### docs - Documentation

**When to use:** Changes to documentation only (no code changes)

**Examples:**

```
docs(readme): add installation instructions
docs(api): update endpoint documentation
docs(contributing): add PR template guidelines
docs(adr): document database choice decision
```

### style - Code Style

**When to use:** Formatting, whitespace, missing semicolons (no code behavior
change)

**Examples:**

```
style(api): format with prettier
style(tests): fix indentation
style(ui): organize imports alphabetically
```

**Note:** Don't confuse with UI styling - that's usually `feat` or `fix`

### refactor - Code Refactoring

**When to use:** Code restructuring without changing behavior or adding features

**Examples:**

```
refactor(auth): extract validation logic to helper
refactor(db): migrate to connection pooling
refactor(ui): consolidate duplicate components
refactor(api): simplify error handling flow
```

**With performance impact:**

```
refactor(parser): optimize token processing

Reduce memory allocation in hot path.
Performance improvement: 40% faster on large files.
```

### test - Tests

**When to use:** Adding or updating tests (no production code changes)

**Examples:**

```
test(auth): add unit tests for token validation
test(api): add integration tests for user endpoints
test(ui): add snapshot tests for components
test(e2e): add checkout flow tests
```

### chore - Maintenance

**When to use:** Build process, tooling, dependencies, configuration

**Examples:**

```
chore(deps): update dependencies
chore(ci): add GitHub Actions workflow
chore(config): update ESLint rules
chore(build): optimize webpack configuration
```

**Dependency updates:**

```
chore(deps): update to React 18

- react: 17.0.2 -> 18.2.0
- react-dom: 17.0.2 -> 18.2.0
- Updated tests for new rendering behavior
```

### perf - Performance

**When to use:** Code changes that improve performance

**Examples:**

```
perf(api): add response caching
perf(ui): lazy load dashboard components
perf(db): add index on user_email column
perf(parser): optimize regex compilation
```

**With metrics:**

```
perf(search): implement full-text search indexing

Reduces search time from 2s to 50ms on 10k records.
```

### build - Build System

**When to use:** Changes to build system, external dependencies, project
configuration

**Examples:**

```
build(webpack): add code splitting
build(docker): optimize image size
build(npm): add publish script
build(vite): migrate from webpack
```

### ci - Continuous Integration

**When to use:** CI/CD pipeline, automation, deployment configuration

**Examples:**

```
ci(github): add PR validation workflow
ci(deploy): add staging environment
ci(test): run tests in parallel
ci(security): add dependency scanning
```

### revert - Revert Previous Commit

**When to use:** Reverting a previous commit

**Format:**

```
revert: <reverted commit subject>

This reverts commit <hash>.

Reason: <why it's being reverted>
```

**Example:**

```
revert: feat(api): add user deletion endpoint

This reverts commit 1234567.

Reason: Found critical security issue in implementation.
Will re-implement with proper authorization checks.
```

## Scope Guidelines

### Choosing Scopes

Scopes should be:

- **Short** (1-2 words)
- **Consistent** across commits
- **Meaningful** to your team
- **Hierarchical** when appropriate

### Common Scope Patterns

**By Feature/Module:**

```
feat(auth): ...
feat(billing): ...
feat(dashboard): ...
```

**By Layer:**

```
fix(api): ...
fix(ui): ...
fix(db): ...
fix(cli): ...
```

**By Package (Monorepo):**

```
feat(@company/api): ...
feat(@company/web): ...
feat(@company/mobile): ...
```

**By Component:**

```
fix(button): ...
fix(modal): ...
fix(form): ...
```

### When to Skip Scope

Scope is optional for:

- Changes affecting multiple areas: `chore: update dependencies`
- Small repositories with obvious context: `docs: fix typo`
- Initial commits: `feat: initial commit`

## Subject Guidelines

### Writing Great Subjects

**Do:**

- ✅ Use imperative mood: "add feature" not "added feature"
- ✅ Keep it short: <72 characters (50 is ideal)
- ✅ Lowercase first letter
- ✅ No period at the end
- ✅ Be specific: "fix login error" not "fix bug"

**Don't:**

- ❌ Past tense: "added feature"
- ❌ Too vague: "update code"
- ❌ Too long: runs off screen
- ❌ Capitalize: "Add Feature"
- ❌ End with period: "fix bug."

**Examples:**

```
❌ Fixed the bug in the user authentication system
✅ fix(auth): prevent token expiration race condition

❌ Added new feature
✅ feat(api): add user profile endpoints

❌ Update Dependencies
✅ chore(deps): update react to v18

❌ Refactored code.
✅ refactor(parser): extract validation logic
```

## Body Guidelines

### When to Add a Body

Add a body when:

- Change requires explanation beyond the subject
- Multiple files affected in non-obvious ways
- Implementation has tradeoffs
- Migration steps needed
- Performance impact to document

### Body Best Practices

```
feat(api): add rate limiting middleware

Implement token bucket algorithm for API rate limiting.
Default: 100 requests per minute per user.

Configuration:
- RATE_LIMIT_REQUESTS: max requests
- RATE_LIMIT_WINDOW: time window in seconds

Endpoints exempt from limits:
- /health
- /metrics
```

**Structure:**

1. **Context** - Why this change is needed
2. **Implementation** - How it works (if not obvious)
3. **Details** - Configuration, caveats, alternatives considered

## Footer Guidelines

### Breaking Changes

**Format:**

```
BREAKING CHANGE: <description>

<migration guide>
```

**Example:**

```
feat(api)!: migrate to v2 authentication

BREAKING CHANGE: API authentication changed from API keys to JWT tokens.

Migration:
1. Obtain JWT token: POST /auth/login
2. Replace header: "X-API-Key" -> "Authorization: Bearer <token>"
3. Tokens expire after 24h, implement refresh logic

See docs/migration/v2-auth.md for details
```

### Issue References

**Formats:**

```
Fixes #123
Closes #456
Refs #789
See also #111, #222
```

**Examples:**

```
fix(api): handle edge case in user deletion

Properly cascade deletions to related entities.

Fixes #456
Closes #789
```

**Multiple issues:**

```
feat(ui): add user management dashboard

Implements user CRUD interface with role management.

Closes #123, #124, #125
```

## Complex Examples

### Large Feature with Multiple Changes

```
feat(checkout): implement multi-step checkout flow

Add 3-step checkout process:
1. Shipping information
2. Payment details
3. Order review

Features:
- Form validation with real-time feedback
- Address autocomplete via Google Maps API
- Payment processing via Stripe
- Order confirmation emails

Breaking changes:
- Old /checkout endpoint removed
- Cart structure changed (see migration guide)

BREAKING CHANGE: Cart item structure changed.
Old: { id, quantity }
New: { productId, quantity, selectedOptions }

Migration: Update client code to use new structure.

Closes #456, #457, #458
```

### Security Fix

```
fix(auth): patch session fixation vulnerability

Regenerate session ID after authentication to prevent
session fixation attacks. Sessions now invalidated on
logout and role changes.

Security: Prevents attacker-controlled session IDs.
No user action required.

Fixes CVE-2024-XXXXX
```

### Performance Optimization

```
perf(db): add composite index on frequent queries

Analysis showed 80% of queries filter by (tenant_id, created_at).
Added composite index to optimize these queries.

Before: ~2000ms average
After: ~50ms average

Migration: Run `npm run migrate` to add index.
```

### Dependency Update with Breaking Changes

```
chore(deps): update TypeScript 4.9 -> 5.0

Breaking changes in TypeScript 5.0:
- Stricter enum checks
- Decorators metadata changes

Changes made:
- Updated decorator usage in auth module
- Fixed enum comparisons in 3 locations
- Updated @types packages

All tests passing. No runtime behavior changes.
```

## Commit History Examples

### Good Commit History

```
feat(api): add user endpoint
feat(ui): add user list page
test(api): add user endpoint tests
docs(api): document user endpoints
fix(ui): correct pagination in user list
```

### Poor Commit History

```
Update stuff
Fix
WIP
More changes
Final fixes
Actually final
```

## Tips

1. **Write commits as you work**, not at the end
2. **Commits should be atomic** - one logical change per commit
3. **Subject line is the headline** - make it count
4. **Body explains the "why"** - code shows the "what"
5. **Reference issues** - create traceability
6. **Breaking changes are loud** - use `!` and BREAKING CHANGE
7. **Scopes create patterns** - your team will thank you

## Tools

### Validate Commits

```bash
# commitlint
npm install -g @commitlint/cli @commitlint/config-conventional
echo "feat(api): add endpoint" | commitlint

# conventional-changelog
npm install -g conventional-changelog-cli
conventional-changelog -p angular -i CHANGELOG.md -s
```

### Git Hooks

```bash
# Use prepare-commit-msg hook to suggest format
# See hooks/prepare-commit-msg in this repo
```

---

**Reference Version:** 1.0 **Standard:** Conventional Commits 1.0.0 **Last
Updated:** 2025-11-08
