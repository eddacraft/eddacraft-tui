# Code Review: [PR/Feature Name]

## Review Summary

**Verdict:** ✅ APPROVED | ❌ CHANGES REQUESTED | ⚠️ APPROVED WITH COMMENTS

**Risk Level:** Low | Medium | High **Estimated Impact:** Minimal | Moderate |
Significant

## Quick Stats

- Files changed: X
- Lines added/removed: +XXX/-YYY
- Test coverage: XX%
- Build status: ✅ Passing | ❌ Failing

## High-Priority Issues

### 🚨 Must Fix Before Merge

| File:Line        | Issue                       | Suggested Fix                | Severity |
| ---------------- | --------------------------- | ---------------------------- | -------- |
| `src/api.ts:45`  | SQL injection vulnerability | Use parameterized query      | Critical |
| `src/auth.ts:23` | Missing authorization check | Add role check before access | Critical |

### ⚠️ Should Fix

| File:Line         | Issue                     | Suggested Fix                   | Severity |
| ----------------- | ------------------------- | ------------------------------- | -------- |
| `src/util.ts:12`  | Performance issue in loop | Use Map instead of nested loops | Medium   |
| `src/model.ts:89` | Missing validation        | Add input bounds checking       | Medium   |

### 💭 Consider (Nits)

| File:Line          | Comment                                      |
| ------------------ | -------------------------------------------- |
| `src/config.ts:5`  | Consider extracting magic number to constant |
| `src/logger.ts:34` | Typo in comment: "recieve" → "receive"       |

## Checklist Review

### Code Quality

- [ ] **Readability:** Code is clear and self-documenting
- [ ] **Patterns:** Follows existing codebase patterns
- [ ] **DRY:** No unnecessary duplication
- [ ] **Naming:** Variables and functions clearly named
- [ ] **Comments:** Complex logic is documented

### Functionality

- [ ] **Requirements:** Meets acceptance criteria
- [ ] **Edge Cases:** Handles error conditions
- [ ] **Backwards Compatibility:** No breaking changes (or documented)
- [ ] **Feature Flags:** New features can be toggled if needed

### Testing

- [ ] **Test Coverage:** New code has tests
- [ ] **Test Quality:** Tests are meaningful, not just coverage
- [ ] **Edge Cases:** Tests cover error paths
- [ ] **Mocks:** External dependencies properly mocked

### Performance

- [ ] **Efficiency:** No obvious performance issues
- [ ] **Database:** Queries are optimised (indexes used)
- [ ] **Caching:** Appropriate caching strategy
- [ ] **Memory:** No memory leaks or excessive allocations

### Security

- [ ] **Input Validation:** All inputs validated
- [ ] **Authentication:** Proper auth checks
- [ ] **Authorization:** Correct permission checks
- [ ] **Secrets:** No hardcoded secrets or keys
- [ ] **Dependencies:** No vulnerable dependencies
- [ ] **Logging:** No PII in logs

### Observability

- [ ] **Logging:** Appropriate log levels used
- [ ] **Metrics:** Key operations have metrics
- [ ] **Errors:** Error handling with context
- [ ] **Tracing:** Distributed tracing where applicable

## Positive Feedback

<!-- Acknowledge good practices -->

- ✨ Clean separation of concerns in service layer
- ✨ Good test coverage for edge cases
- ✨ Well-structured error handling

## Code Examples

### Issue Example

```typescript
// ❌ Current implementation - vulnerable to injection
const query = `SELECT * FROM users WHERE id = ${userId}`;

// ✅ Suggested fix
const query = 'SELECT * FROM users WHERE id = ?';
db.query(query, [userId]);
```

### Pattern to Follow

```typescript
// Good pattern from existing codebase
// Reference: src/services/existing-service.ts:123
export class ServiceName {
  constructor(private readonly deps: Dependencies) {}
  // ...
}
```

## Dependencies Check

- **New dependencies:** List any new packages
- **Updated dependencies:** Note version changes
- **Security scan:** Results from npm audit/safety check

## Migration & Deployment Notes

- [ ] Database migrations required
- [ ] Configuration changes needed
- [ ] Feature flag setup
- [ ] Rollback plan documented

## Follow-up Actions

### For Coder

- Fix critical issues listed above
- Add missing test cases for [specific scenarios]
- Update documentation for [new feature]

### For Security Auditor

- Review authentication flow changes in `auth.ts`
- Verify PII handling in new user endpoints

### For Tester

- Add integration test for [specific flow]
- Verify performance under load for [endpoint]

## Additional Context

<!-- Any relevant information about the review -->

- Related to ticket: #XXX
- Follows up on: [previous PR/decision]
- Blocked by: [external dependency]

## Review Tools Used

```bash
# Static analysis
npm run lint
npm run type-check

# Security scan
npm audit
grep -r "TODO\|FIXME\|XXX" src/

# Test execution
npm test -- --coverage
```
