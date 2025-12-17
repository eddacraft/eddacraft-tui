# APS Conventions

> Detailed conventions for Anvil Planning Spec documents.

## File Naming

### Leaf Specs

Leaf specs must use the `.aps.md` extension:

**Valid:**

- `auth.aps.md`
- `feature-payments.aps.md`
- `refactor-api.aps.md`

**Invalid:**

- `auth.md` (missing `.aps` marker)
- `auth.aps.txt` (wrong file extension)

### Index Files

Index files should use `APS.md` or `<name>.aps.md`:

**Recommended:**

- `APS.md` (standard index name)
- `system.aps.md` (named index)

**Also valid:**

- `plan.aps.md`
- `roadmap.aps.md`

### Directory Structure

Recommended structure for multi-module plans:

```
docs/planning/
  APS.md                 # Index file (or system.aps.md)
  modules/               # Leaf specs
    auth.aps.md
    payments.aps.md
    admin.aps.md
```

---

## Root File Discovery

The loader searches for index files in this order:

1. Explicit path provided to CLI (`anvil plan validate ./docs/planning/APS.md`)
2. `docs/planning/APS.md` (conventional location)
3. `APS.md` in repository root
4. Any `*.aps.md` file if only one exists

**Best practice:** Use `docs/planning/APS.md` as your index file.

---

## Heading Hierarchy

### Index Files

```markdown
# Plan Title (H1 - exactly one)

## Overview (H2 - optional)

## Modules (H2 - required)

### auth (H3 - module id)

### payments (H3 - module id)

## Open Questions (H2 - optional)

## Decisions (H2 - optional)
```

### Leaf Specs

```markdown
# Module Title (H1 - exactly one)

## Tasks (H2 - required)

### AUTH-001: Task title (H3 - task definition)

### AUTH-002: Another task (H3 - task definition)

## Dependencies (H2 - optional)

## Notes (H2 - optional)
```

**Rules:**

- Exactly one H1 per file
- Required sections must be H2
- Module IDs and tasks must be H3
- No heading levels beyond H3 in structured sections

---

## Link Rules

### Format

All links must use Markdown link syntax:

```markdown
[Link text](./path/to/file.md)
```

### Paths

- Must be **relative** to the current file
- Must point to files **within the repository**
- No absolute paths (`/home/...` or `C:\...`)
- No external URLs in structural links

**Valid:**

```markdown
[Auth module](./modules/auth.aps.md) [Related spec](../other-plan.aps.md)
```

**Invalid:**

```markdown
[Auth module](/docs/planning/modules/auth.aps.md) (absolute)
[External](https://example.com/spec.md) (external URL)
```

### Link Target Validation

The validator checks:

- Target file exists
- No circular references
- Broken links are reported with `file:line` location

---

## Task ID Format

### Structure

Task IDs follow the pattern: `<SCOPE>-<NUMBER>`

**Components:**

- **Scope:** UPPERCASE prefix (1-10 characters, alphanumeric)
- **Separator:** Single hyphen (`-`)
- **Number:** Zero-padded 3-digit number (001-999)

**Examples:**

- `AUTH-001` ✓
- `PAY-042` ✓
- `DB-999` ✓
- `ADMIN-123` ✓

**Invalid:**

- `auth-001` ✗ (lowercase scope)
- `AUTH_001` ✗ (underscore instead of hyphen)
- `AUTH001` ✗ (no separator)
- `AUTH-1` ✗ (not zero-padded)
- `AUTH-1234` ✗ (too many digits)

### Uniqueness

Task IDs must be **globally unique** across the entire plan graph.

The validator will report duplicate IDs with file:line references:

```
Error: Duplicate task ID 'AUTH-001'
  Found in:
    - docs/planning/modules/auth.aps.md:23
    - docs/planning/modules/admin.aps.md:45
```

### Numbering Conventions

**Sequential numbering:**

```markdown
AUTH-001: First task AUTH-002: Second task AUTH-003: Third task
```

**Gap numbering (recommended for long-lived plans):**

```markdown
AUTH-010: User model AUTH-020: Login endpoint AUTH-030: Password reset
```

Gap numbering allows insertion of tasks without renumbering.

---

## Field Format

### Key-Value Fields

Task and module metadata use bold key-value format:

```markdown
**Key:** Value
```

**Rules:**

- Key must be bold (surrounded by `**`)
- Key must end with colon (`:`)
- Value starts after the space following the colon
- Value continues to end of line

**Valid:**

```markdown
**Intent:** Create login endpoint **Scope:** AUTH **Tags:** security, api
```

**Invalid:**

```markdown
Intent: Create login endpoint (key not bold) **Intent** Create login endpoint
(no colon) **Intent:**Create login endpoint (no space after colon)
```

### Multi-Line Fields

Some fields support multi-line values:

```markdown
**Inputs:**

- User credentials
- Database connection
- Email service
```

Fields that support lists:

- **Inputs:**
- **Dependencies:** (also supports inline comma-separated)
- **Tags:** (also supports inline comma-separated)

### Inline Lists

Comma-separated values:

```markdown
**Tags:** security, api, high-risk **Scopes:** AUTH, DB **Dependencies:**
AUTH-001, DB-002
```

**Rules:**

- Comma-separated
- Optional space after comma
- No trailing comma

---

## Scope Naming

### Scope Format

Scopes are UPPERCASE alphanumeric identifiers:

**Valid:**

- `AUTH`
- `PAY`
- `DB`
- `ADMIN`
- `API`

**Invalid:**

- `auth` (lowercase)
- `Auth` (mixed case)
- `AUTH_API` (underscore - use `AUTHAPI` or separate scopes)

### Scope Assignment

**Module level:**

```markdown
### auth

- **Scope:** AUTH
```

**Task level:**

```markdown
**Scopes:** AUTH, DB
```

**Note:** Task-level `Scopes` is plural, module-level `Scope` is singular.

### Multi-Scope Tasks

Tasks can declare multiple scopes:

```markdown
### AUTH-001: Migrate user table

**Intent:** Add email_verified column to users table **Scopes:** AUTH, DB
```

This task can modify both AUTH and DB code.

---

## Tag Conventions

### Format

Tags are lowercase, kebab-case:

**Recommended:**

```markdown
**Tags:** security, api, high-risk, needs-review
```

**Also valid:**

```markdown
**Tags:** security, api-v2, database-migration
```

**Avoid:**

```markdown
**Tags:** Security, API, HIGH_RISK (mixed case/underscores)
```

### Common Tags

Suggested tags for consistency:

**Type:**

- `api`, `ui`, `database`, `infrastructure`, `testing`

**Priority:**

- `high-priority`, `low-priority`, `nice-to-have`

**Risk:**

- `high-risk`, `breaking-change`, `security-critical`

**Process:**

- `needs-review`, `needs-testing`, `blocked`, `spike`

---

## Confidence Levels

### Values

Three confidence levels:

- **`low`** — Uncertain approach, may require experimentation
- **medium`** — Generally clear, some unknowns
- **`high`** — Well-understood, clear path forward

### Usage

Confidence helps prioritise and signal uncertainty:

```markdown
### AUTH-001: Implement login

**Confidence:** high **Intent:** Create standard JWT-based login

### AUTH-010: Add OAuth support

**Confidence:** low **Intent:** Evaluate OAuth providers and integrate best fit
```

**Low confidence** signals:

- May require research or spikes
- Approach not yet determined
- Higher risk of scope change

---

## Priority Levels

### Values

Three priority levels:

- **`low`** — Nice to have, non-critical
- **`medium`** — Important, normal priority
- **`high`** — Critical, must complete soon

### Module Priority

```markdown
### auth

- **Priority:** high
```

### Task Priority

Tasks inherit module priority unless overridden:

```markdown
### AUTH-001: Implement login

**Priority:** high (override module priority)
```

---

## Owner Format

### Syntax

Owners use `@username` format:

```markdown
**Owner:** @alice
```

### Team Ownership

Teams can be prefixed with `@team/`:

```markdown
**Owner:** @team/platform
```

### Multiple Owners

Use comma-separated list:

```markdown
**Owner:** @alice, @bob
```

---

## Dependency Format

### Task Dependencies

Reference task IDs:

```markdown
**Dependencies:** AUTH-001, DB-002
```

### Module Dependencies

Reference module IDs:

```markdown
### payments

- **Dependencies:** auth, database
```

### Dependency Resolution

Dependencies create a directed graph. The validator checks:

- All referenced IDs exist
- No circular dependencies

---

## Status Management

### Where Status Lives

Task status is **NOT** stored in planning docs. It lives in `.anvil/state.json`:

**Planning doc (source):**

```markdown
### AUTH-001: Implement login

**Intent:** Create login endpoint (no status field)
```

**State file (derived):**

```json
{
  "AUTH-001": {
    "status": "locked",
    "locked_at": "2025-12-17T10:30:00Z",
    "locked_by": "alice",
    "source": {
      "file": "docs/planning/modules/auth.aps.md",
      "line": 23
    }
  }
}
```

### Why Separate?

- Planning docs remain clean, version-controllable
- Status is execution state, not planning intent
- Avoids merge conflicts from concurrent work
- Single source of truth: `.anvil/state.json`

---

## Markdown Formatting

### Code Blocks

Use fenced code blocks with language identifiers:

````markdown
```typescript
const user = await db.users.findOne({ email });
```
````

### Lists

Use consistent list markers:

**Unordered:**

```markdown
- First item
- Second item
  - Nested item
```

**Ordered:**

```markdown
1. First step
2. Second step
3. Third step
```

### Emphasis

- **Bold** for field keys and emphasis: `**Intent:**`
- _Italic_ for subtle emphasis: `_Note:_`
- `Code` for identifiers: `` `AUTH-001` ``

---

## Best Practices

### Task Sizing

Keep tasks small and focused:

**Too large:**

```markdown
### AUTH-001: Implement entire auth system
```

**Better:**

```markdown
### AUTH-001: Create user model

### AUTH-002: Implement login endpoint

### AUTH-003: Add password reset flow
```

### Intent Clarity

Intents should be concise but clear:

**Too vague:**

```markdown
**Intent:** Fix auth
```

**Better:**

```markdown
**Intent:** Add email verification to signup flow
```

### Module Granularity

Group related functionality:

**Too granular:**

- `login.aps.md`
- `logout.aps.md`
- `signup.aps.md`

**Better:**

- `auth.aps.md` (contains login, logout, signup tasks)

---

## Validation Workflow

1. Write planning doc
2. Run `anvil plan validate`
3. Fix errors (required)
4. Consider warnings (recommended)
5. Commit planning doc
6. Lock tasks as needed: `anvil plan lock --task AUTH-001`

---

## Version History

- **v0.1** (2025-12-17) — Initial conventions
