---
name: code-review
description:
  Comprehensive code review procedures covering correctness, security,
  performance, and maintainability with language-specific checklists
---

# Code Review Skill

This skill provides systematic code review procedures, patterns, and checklists
to ensure high-quality code reviews across different languages and frameworks.

## Capabilities

1. **Systematic Review Process** - Structured approach to code review
2. **Security Pattern Detection** - Identify common vulnerabilities
3. **Performance Analysis** - Spot performance bottlenecks
4. **Maintainability Assessment** - Evaluate code quality and patterns
5. **Language-Specific Guidance** - Tailored checklists for different stacks

## When to Use This Skill

Invoke this skill when:

- Performing code reviews for PRs
- Conducting pre-merge quality checks
- Reviewing code before release
- Auditing existing codebase
- Training on code quality standards

## Review Methodology

### Priority Framework

Review in this order for maximum impact:

1. **🔴 Correctness** - Does it work? Does it meet requirements?
2. **🔴 Security** - Is it safe? Are there vulnerabilities?
3. **🟡 Performance** - Will it scale? Are there bottlenecks?
4. **🟡 Maintainability** - Can others work with it? Is it clear?
5. **🟢 Style** - Does it follow conventions? Is it consistent?

### Review Process

#### Phase 1: Context Gathering (Parallel)

Run these operations simultaneously:

```bash
# In one message with multiple tool calls:
1. Read changed files
2. Read related/similar files for pattern matching
3. Run automated checks (tests, lint, type-check)
4. Search for related code patterns
```

**Example:**

```bash
# These should all run in parallel, not sequentially
npm test
npm run lint
npm run type-check
grep -r "similar_pattern" src/
```

#### Phase 2: Analysis

For each file changed:

1. **Understand Intent** - What is this trying to do?
2. **Check Correctness** - Does it actually do that?
3. **Identify Risks** - What could go wrong?
4. **Verify Tests** - Is it tested?
5. **Assess Patterns** - Does it fit existing patterns?

#### Phase 3: Feedback

Structure comments using the framework:

**Critical Issues (🔴 Must Fix - Blocking)**

- Security vulnerabilities
- Correctness bugs
- Breaking changes without migration
- Missing critical validation

**Important Issues (🟡 Should Fix - Non-blocking but serious)**

- Performance problems
- Maintainability concerns
- Missing tests for important paths
- Pattern violations

**Minor Issues (🟢 Consider - Nice to have)**

- Style inconsistencies
- Minor optimizations
- Documentation improvements
- Naming suggestions

#### Phase 4: Decision

```markdown
## ✅ APPROVED | ❌ CHANGES REQUESTED

**Overall:** [1-2 sentence summary] **Risk Level:** Low | Medium | High **Test
Coverage:** Adequate | Needs Improvement | Excellent **Decision:** Approve |
Request Changes | Needs Discussion
```

## Comment Framework

### Critical Issue Template

```
File: [file path]:[line number]
Issue: [Clear description of the problem]
Risk: [What could happen if not fixed]
Fix: [Specific solution or pattern to use]
Example: [Code snippet or reference to existing pattern]
```

**Example:**

```
File: src/api/users.ts:45
Issue: SQL injection vulnerability in search query
Risk: Attacker can execute arbitrary SQL, access/modify all data
Fix: Use parameterized queries
Example: See src/api/products.ts:23 for correct pattern
```

### Important Issue Template

```
File: [file path]:[line number]
Issue: [Description]
Impact: [Performance/maintainability impact]
Suggestion: [How to improve]
```

**Example:**

```
File: src/utils/search.ts:78
Issue: O(n²) complexity in duplicate detection
Impact: Will be slow with large datasets (>1000 items)
Suggestion: Use Set for O(n) complexity: `new Set(items).size !== items.length`
```

### Positive Feedback Template

Always include positive feedback:

```markdown
### 🎯 Good Patterns

- [Specific thing done well]
- [Another good practice]
- [Positive observation]
```

**Example:**

```markdown
### 🎯 Good Patterns

- Excellent separation of concerns in service layer
- Comprehensive error handling with user-friendly messages
- Well-structured tests covering edge cases
- Clear documentation and type annotations
```

## Automated Checks

### Security Scans

```bash
# Dependency vulnerabilities
npm audit
pip-audit (Python)
cargo audit (Rust)

# Secret scanning
grep -rE "(password|secret|api_key|token)" --exclude-dir=node_modules
grep -rE "['\"][A-Za-z0-9]{32,}['\"]" src/ # Potential hardcoded tokens

# Common vulnerabilities
grep -r "innerHTML\|dangerouslySetInnerHTML" src/ # XSS risk
grep -r "eval(" src/ # Code injection risk
grep -r "require.*req\." src/ # Dynamic require() risk
```

### Complexity Analysis

```bash
# Find large files (>300 lines often need refactoring)
find src -name "*.ts" -exec wc -l {} + | awk '$1 > 300' | sort -rn

# Find long functions (basic heuristic)
grep -n "function\|const.*=.*(" src/**/*.ts | # Find function definitions
  while read line; do
    # Count lines until next function
  done

# TODO/FIXME count
grep -rc "TODO\|FIXME\|XXX\|HACK" src/ | grep -v ":0$"
```

### Pattern Verification

```bash
# Check consistency of similar patterns
grep -r "useEffect" src/ --include="*.tsx" | wc -l
grep -r "componentDidMount" src/ --include="*.tsx" | wc -l
# If both are present, inconsistent React patterns

# Check error handling patterns
grep -r "try {" src/ | wc -l
grep -r "\.catch(" src/ | wc -l
# Should see consistent approach

# Check test coverage
npm test -- --coverage
```

## Quality Dimensions

### 1. Correctness

**Questions to Ask:**

- Does it meet the stated requirements?
- Are edge cases handled?
- Is error handling comprehensive?
- Could this fail in production? How?

**Common Issues:**

- Off-by-one errors
- Null/undefined handling
- Type mismatches (in dynamically typed languages)
- Race conditions in async code
- Incorrect algorithm logic

**Check:**

```javascript
// ❌ Off-by-one error
for (
  let i = 0;
  i <= array.length;
  i++ // Should be <
)
  // ❌ No null check
  user.profile.name; // user or profile might be null

// ❌ Type mismatch
const count = '5' + 3; // "53" not 8

// ✅ Correct
for (let i = 0; i < array.length; i++) user?.profile?.name ?? 'Unknown';
const count = parseInt('5') + 3;
```

### 2. Security

**Questions to Ask:**

- Is user input validated and sanitized?
- Are there authentication/authorization checks?
- Could this expose sensitive data?
- Are dependencies up to date and secure?

**Common Vulnerabilities:**

- SQL/NoSQL injection
- Cross-Site Scripting (XSS)
- Cross-Site Request Forgery (CSRF)
- Insecure deserialization
- Missing authentication
- Information disclosure
- Hardcoded secrets

**See:** `security-patterns.md` for detailed vulnerability patterns

### 3. Performance

**Questions to Ask:**

- Will this scale to production data volumes?
- Are there unnecessary database queries?
- Could this block the event loop?
- Is caching appropriate?

**Common Issues:**

- N+1 query problems
- Missing database indexes
- Synchronous operations in async context
- Memory leaks
- Inefficient algorithms
- Missing pagination
- Unnecessary re-renders (React)

**See:** `performance-patterns.md` for detailed patterns

### 4. Maintainability

**Questions to Ask:**

- Can another developer understand this?
- Is it following project conventions?
- Is there unnecessary complexity?
- Are abstractions appropriate?

**Code Smells:**

- Magic numbers: `if (status === 3)` → `if (status === Status.ACTIVE)`
- Deep nesting: >3 levels of indentation
- Long functions: >50 lines
- God objects: Classes doing too much
- Duplicate code: Copy-pasted logic
- Poor naming: `data`, `temp`, `doStuff()`

### 5. Testing

**Questions to Ask:**

- Are the changes tested?
- Do tests cover edge cases?
- Are tests meaningful (not just for coverage)?
- Would failures be caught by tests?

**Test Quality:**

- Unit tests for business logic
- Integration tests for APIs
- E2E tests for critical flows
- Edge cases covered
- Error scenarios tested
- Mock usage appropriate

## Language-Specific Guidance

This skill includes specialized checklists for:

- **JavaScript/TypeScript** - See `checklists.md#javascript`
- **Python** - See `checklists.md#python`
- **Rust** - See `checklists.md#rust`
- **Go** - See `checklists.md#go`
- **React** - See `checklists.md#react`

## Progressive Disclosure

### Quick Review (5-10 min)

- Run automated checks
- Scan for obvious security issues
- Verify tests exist and pass
- Check for major code smells

### Standard Review (20-30 min)

- Full correctness check
- Security pattern analysis
- Performance assessment
- Maintainability evaluation
- Provide specific feedback

### Deep Review (1+ hour)

- Architectural implications
- Cross-module impact analysis
- Security threat modeling
- Performance profiling
- Comprehensive refactoring suggestions

## Integration with Agents

This skill works with the **reviewer agent**:

- **Agent provides:** Persona, context, decision-making
- **Skill provides:** Checklists, patterns, procedures

The agent asks "What should I review?" and "What's the risk?" The skill answers
"Here's the systematic approach" and "Here are the patterns to check"

## Output Template

```markdown
## ❌ CHANGES REQUESTED | ✅ APPROVED

**Overall:** [Summary of changes and quality assessment] **Risk Level:** Low |
Medium | High **Test Coverage:** [Assessment]

### 🔴 Must Fix (Blocking)

1. **[Issue Category]** - `file.ts:line`
   - [Description]
   - [Fix suggestion]

### 🟡 Should Fix (Important)

1. **[Issue Category]** - `file.ts:line`
   - [Description]
   - [Suggestion]

### 🟢 Consider (Minor)

1. **[Observation]** - `file.ts:line`
   - [Suggestion]

### 🎯 Good Patterns

- [Positive feedback]
- [Good practice observed]

### 📋 Checklist

- [x] Correctness verified
- [x] Security reviewed
- [ ] Performance concerns identified
- [x] Tests adequate
- [x] Follows conventions

### 👉 Next Steps

**For Coder:**

- Fix critical security issue in auth.ts:45
- Add missing null check in utils.ts:23

**For Tester:**

- Add test for concurrent user updates
- Verify error messages are user-friendly
```

## Best Practices

1. **Be Specific** - "Line 45 needs null check" not "Fix null issues"
2. **Explain Why** - Help reviewee learn, don't just demand changes
3. **Provide Examples** - Show the correct pattern from existing code
4. **Balance Feedback** - Include positive observations
5. **Prioritize Issues** - Must fix vs. nice to have
6. **Be Constructive** - Suggest solutions, not just problems
7. **Consider Context** - Tight deadline vs. major refactor
8. **Automate When Possible** - Let tools catch style issues

## Common Mistakes to Avoid

❌ **Vague feedback**: "This could be better" ✅ **Specific feedback**: "Extract
this 50-line function into smaller, focused functions"

❌ **Nitpicking**: Commenting on every minor style deviation ✅ **Focus on
substance**: Let automated tools handle style

❌ **No positive feedback**: Only listing problems ✅ **Balanced**: Acknowledge
good patterns and improvements

❌ **Blocking on opinions**: "I would have done this differently" ✅ **Block on
issues**: Security, correctness, performance problems

## Reference Files

- `checklists.md` - Language and framework-specific review checklists
- `security-patterns.md` - Common security vulnerabilities and fixes
- `performance-patterns.md` - Performance anti-patterns and optimizations

## Tips for Effective Reviews

1. **Review your own code first** - Catch obvious issues
2. **Review incrementally** - Don't save all feedback for the end
3. **Use the right granularity** - Too detailed is exhausting, too high-level
   isn't helpful
4. **Consider the author's experience** - Adjust depth of feedback
5. **Follow up** - Verify fixes address the concerns
6. **Learn from reviews** - If you see a pattern repeatedly, update
   documentation/linting
7. **Time-box reviews** - Don't spend 2 hours on a 10-line change

---

**Skill Version:** 1.0 **Compatible With:** Claude Code, Claude.ai, Claude Agent
SDK **Last Updated:** 2025-11-08
