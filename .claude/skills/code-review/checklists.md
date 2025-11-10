# Language-Specific Review Checklists

This file provides detailed review checklists for different programming
languages and frameworks. Load the relevant section when reviewing code in that
language.

## JavaScript/TypeScript

### General JavaScript/TypeScript

**Type Safety (TypeScript)**

- [ ] No `any` types (or justified with comment)
- [ ] Interfaces/types defined for complex objects
- [ ] Function parameters and return types annotated
- [ ] Proper use of generics where applicable
- [ ] No type assertions (`as`) without justification
- [ ] Enums used appropriately (or const objects)

**Modern JavaScript**

- [ ] Uses `const`/`let` instead of `var`
- [ ] Arrow functions for callbacks
- [ ] Template literals instead of string concatenation
- [ ] Destructuring where appropriate
- [ ] Spread operator instead of `Object.assign`
- [ ] Optional chaining (`?.`) for nested properties
- [ ] Nullish coalescing (`??`) instead of `||` where appropriate

**Async/Await**

- [ ] Promises handled with async/await not `.then()`
- [ ] Error handling with try/catch
- [ ] No missing `await` keywords
- [ ] No synchronous operations in async functions
- [ ] Parallel operations use `Promise.all()`

```typescript
// ❌ Bad
async function fetchData() {
  const user = await getUser();
  const posts = await getPosts(); // Could be parallel
  return { user, posts };
}

// ✅ Good
async function fetchData() {
  const [user, posts] = await Promise.all([getUser(), getPosts()]);
  return { user, posts };
}
```

**Error Handling**

- [ ] Errors properly caught and handled
- [ ] Custom error types for different scenarios
- [ ] No silent failures
- [ ] Errors logged appropriately
- [ ] User-friendly error messages

**Common Pitfalls**

- [ ] No comparison with `==` (use `===`)
- [ ] No truthy/falsy bugs (`if (count)` fails for 0)
- [ ] Array methods return new arrays (immutability)
- [ ] Closures don't capture loop variables incorrectly
- [ ] `this` binding handled correctly

```javascript
// ❌ Bad - truthy/falsy bug
if (count) {
  // Fails when count is 0
  doSomething();
}

// ✅ Good
if (count > 0) {
  doSomething();
}

// ❌ Bad - this binding
setTimeout(obj.method, 1000); // `this` will be wrong

// ✅ Good
setTimeout(() => obj.method(), 1000);
setTimeout(obj.method.bind(obj), 1000);
```

### Node.js Specific

**Async Operations**

- [ ] No blocking synchronous operations (`fs.readFileSync` in routes)
- [ ] Streams used for large files
- [ ] Event emitters cleaned up
- [ ] Promises rejected on errors, not thrown

**Security**

- [ ] Environment variables for secrets
- [ ] Input validation and sanitization
- [ ] SQL parameterized queries
- [ ] CSRF protection enabled
- [ ] Helmet.js or equivalent for headers
- [ ] Rate limiting on public endpoints

**Performance**

- [ ] Connection pooling for databases
- [ ] Caching where appropriate
- [ ] Pagination for large datasets
- [ ] Efficient database queries (no N+1)

### React Specific

**Component Structure**

- [ ] Functional components (not classes unless needed)
- [ ] Proper hook usage (rules of hooks)
- [ ] Component single responsibility
- [ ] Props interface defined
- [ ] Default props specified where appropriate

**Hooks**

- [ ] `useState` for local state
- [ ] `useEffect` cleanup functions
- [ ] Dependency arrays complete and correct
- [ ] Custom hooks for reusable logic
- [ ] `useMemo`/`useCallback` for optimization (not premature)

```typescript
// ❌ Bad - missing dependency
useEffect(() => {
  fetchData(userId);
}, []); // Missing userId dependency

// ✅ Good
useEffect(() => {
  fetchData(userId);
}, [userId]);

// ❌ Bad - missing cleanup
useEffect(() => {
  const interval = setInterval(poll, 1000);
}, []);

// ✅ Good
useEffect(() => {
  const interval = setInterval(poll, 1000);
  return () => clearInterval(interval);
}, []);
```

**Performance**

- [ ] Key props for lists
- [ ] Avoid inline function definitions in render
- [ ] Avoid creating objects in render
- [ ] Component memoization (`memo`) where beneficial
- [ ] Virtualization for long lists

**State Management**

- [ ] State lifted to appropriate level
- [ ] Context not overused (performance)
- [ ] No prop drilling (use context/state management)
- [ ] Immutable state updates

**Accessibility**

- [ ] Semantic HTML elements
- [ ] ARIA labels where needed
- [ ] Keyboard navigation support
- [ ] Focus management for modals/dialogs
- [ ] Alt text for images

## Python

### General Python

**Style (PEP 8)**

- [ ] 4 spaces for indentation
- [ ] snake_case for functions/variables
- [ ] PascalCase for classes
- [ ] UPPER_CASE for constants
- [ ] Proper docstrings
- [ ] Line length ≤ 88 chars (Black formatter)

**Type Hints**

- [ ] Function parameters annotated
- [ ] Return types annotated
- [ ] Complex types use `typing` module
- [ ] Optional types specified
- [ ] Type checking passes (mypy)

```python
# ❌ Bad - no type hints
def process_data(data):
    return data['value']

# ✅ Good
from typing import Dict, Optional

def process_data(data: Dict[str, int]) -> Optional[int]:
    return data.get('value')
```

**Modern Python**

- [ ] f-strings for formatting
- [ ] Pathlib instead of os.path
- [ ] Context managers (`with`) for resources
- [ ] List/dict comprehensions where readable
- [ ] Dataclasses for data containers
- [ ] Walrus operator (`:=`) where beneficial

**Error Handling**

- [ ] Specific exception types caught
- [ ] Exceptions don't escape without handling
- [ ] Custom exceptions for domain errors
- [ ] Logging before re-raising
- [ ] No bare `except:` clauses

```python
# ❌ Bad
try:
    risky_operation()
except:  # Too broad
    pass # Silent failure

# ✅ Good
try:
    risky_operation()
except SpecificError as e:
    logger.error(f"Operation failed: {e}")
    raise
```

**Common Pitfalls**

- [ ] No mutable default arguments
- [ ] Proper equality checks (`is` vs `==`)
- [ ] Late binding in closures handled
- [ ] No circular imports
- [ ] Global state minimized

```python
# ❌ Bad - mutable default argument
def add_item(item, items=[]):
    items.append(item)
    return items

# ✅ Good
def add_item(item, items=None):
    if items is None:
        items = []
    items.append(item)
    return items
```

### Django/Flask Specific

**Django**

- [ ] Querysets evaluated efficiently
- [ ] `select_related`/`prefetch_related` for FKs
- [ ] Indexes defined on queried fields
- [ ] Forms used for validation
- [ ] CSRF middleware enabled
- [ ] Permissions checked
- [ ] Migrations tested

**Flask**

- [ ] Request data validated
- [ ] SQL injection prevention
- [ ] Session security configured
- [ ] CORS configured properly
- [ ] Error handlers defined

**Database**

- [ ] Transactions used appropriately
- [ ] No raw SQL without parameterization
- [ ] Migrations reversible
- [ ] Proper indexing strategy

## Rust

### General Rust

**Ownership & Borrowing**

- [ ] No unnecessary clones
- [ ] Borrowing rules followed
- [ ] Lifetimes specified where needed
- [ ] Move semantics understood
- [ ] Slices used instead of owned types where possible

**Error Handling**

- [ ] `Result<T, E>` for fallible operations
- [ ] `Option<T>` for nullable values
- [ ] `?` operator used appropriately
- [ ] Custom error types implemented
- [ ] `unwrap()` justified or absent

```rust
// ❌ Bad - unwrap can panic
fn get_user(id: i32) -> User {
    database.find(id).unwrap()
}

// ✅ Good
fn get_user(id: i32) -> Result<User, DbError> {
    database.find(id)
}
```

**Safety**

- [ ] No `unsafe` without justification and documentation
- [ ] Unsafe code properly audited
- [ ] Interior mutability used correctly
- [ ] No data races possible

**Performance**

- [ ] Iterators instead of for loops where appropriate
- [ ] Zero-copy operations where possible
- [ ] Unnecessary allocations avoided
- [ ] Proper sizing of collections

**Modern Rust**

- [ ] 2021 edition features used
- [ ] `if let`/`while let` for pattern matching
- [ ] Turbofish syntax (`::<>`) avoided where possible
- [ ] Derive macros used appropriately

## Go

### General Go

**Style**

- [ ] `gofmt` formatted
- [ ] Exported names documented
- [ ] Package names lowercase, single word
- [ ] Receiver names consistent
- [ ] Interface names follow conventions (`-er` suffix)

**Error Handling**

- [ ] Errors checked and handled
- [ ] Error messages provide context
- [ ] Sentinel errors used appropriately
- [ ] Error wrapping with `%w` (Go 1.13+)
- [ ] No ignored errors

```go
// ❌ Bad - ignored error
data, _ := ioutil.ReadFile("file.txt")

// ✅ Good
data, err := ioutil.ReadFile("file.txt")
if err != nil {
    return fmt.Errorf("failed to read file: %w", err)
}
```

**Concurrency**

- [ ] Goroutines don't leak
- [ ] Channels closed by sender
- [ ] WaitGroups used for synchronization
- [ ] Context used for cancellation
- [ ] Race conditions avoided

**Common Pitfalls**

- [ ] No goroutine leaks
- [ ] Defer in loops handled correctly
- [ ] No accidental slice sharing
- [ ] Nil pointer checks
- [ ] Error shadowing avoided

```go
// ❌ Bad - goroutine leak
func process() {
    go func() {
        for {
            // Never exits
            doWork()
        }
    }()
}

// ✅ Good
func process(ctx context.Context) {
    go func() {
        for {
            select {
            case <-ctx.Done():
                return
            default:
                doWork()
            }
        }
    }()
}
```

## SQL/Database

### Query Quality

- [ ] Parameterized queries (no string concatenation)
- [ ] Proper indexes defined
- [ ] No SELECT \*
- [ ] JOINs efficient
- [ ] No N+1 queries
- [ ] Transactions used appropriately
- [ ] Connection pooling configured

### Schema

- [ ] Proper data types
- [ ] NOT NULL where appropriate
- [ ] Foreign keys defined
- [ ] Unique constraints where needed
- [ ] Indexes on frequently queried columns
- [ ] Migrations reversible

### Security

- [ ] No SQL injection possible
- [ ] Row-level security considered
- [ ] Sensitive data encrypted
- [ ] Proper access controls

## Testing

### Test Quality

- [ ] Tests actually test something
- [ ] Test names describe what's being tested
- [ ] Arrange-Act-Assert pattern
- [ ] One assertion per test (or related assertions)
- [ ] Tests are independent
- [ ] Tests are repeatable

### Coverage

- [ ] Happy path tested
- [ ] Edge cases tested
- [ ] Error cases tested
- [ ] Boundary conditions tested
- [ ] No flaky tests

### Test Patterns

- [ ] Mocks used appropriately
- [ ] Fixtures/factories for test data
- [ ] Integration tests for critical paths
- [ ] E2E tests for key workflows

```typescript
// ❌ Bad - testing implementation details
test('should set state to true', () => {
  component.setState({ isOpen: true });
  expect(component.state.isOpen).toBe(true);
});

// ✅ Good - testing behavior
test('should show modal when button is clicked', () => {
  fireEvent.click(screen.getByRole('button'));
  expect(screen.getByRole('dialog')).toBeInTheDocument();
});
```

## General Principles

### SOLID Principles

- [ ] Single Responsibility Principle
- [ ] Open/Closed Principle
- [ ] Liskov Substitution Principle
- [ ] Interface Segregation Principle
- [ ] Dependency Inversion Principle

### DRY (Don't Repeat Yourself)

- [ ] No code duplication
- [ ] Shared logic extracted
- [ ] Not over-abstracted (avoid wrong abstraction)

### KISS (Keep It Simple)

- [ ] Simplest solution that works
- [ ] No premature optimization
- [ ] No unnecessary complexity

### YAGNI (You Aren't Gonna Need It)

- [ ] No unused code
- [ ] No features for "maybe later"
- [ ] Build for today's requirements

---

**Reference Version:** 1.0 **Last Updated:** 2025-11-08
