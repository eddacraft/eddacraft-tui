---
name: git-workflow
description:
  Git operations including commits, PRs, and changelogs using industry best
  practices and Conventional Commits
---

# Git Workflow Skill

This skill provides comprehensive git operations following industry best
practices, including Conventional Commits, semantic versioning, and
comprehensive PR documentation.

## Capabilities

1. **Commit Message Generation** - Create Conventional Commit messages from
   staged changes
2. **Pull Request Creation** - Generate comprehensive PRs with complete context
3. **Changelog Generation** - Build release notes from git history

## When to Use This Skill

Invoke this skill when:

- Creating commits from staged changes
- Preparing to create a pull request
- Generating release changelogs
- Need guidance on git best practices
- Want to ensure consistent commit/PR formatting

## Usage Patterns

### 1. Generate Commit Message

**Process:**

1. **Analyze Staged Changes**

   ```bash
   git diff --cached --stat
   git diff --cached
   ```

2. **Run Analysis Script** (if available)

   ```bash
   .claude/skills/git-workflow/scripts/analyze-diff.sh
   ```

3. **Determine Commit Type**
   - Reference `commit-patterns.md` for type selection
   - Common types: feat, fix, docs, style, refactor, test, chore
   - Check for breaking changes

4. **Identify Scope**
   - Extract from changed file paths
   - Examples: `api`, `ui`, `auth`, `db`
   - Keep scope concise (1-2 words)

5. **Craft Message** Format: `type(scope): subject`
   - Subject: imperative mood, lowercase, no period
   - Max 72 characters for subject
   - Add body for complex changes
   - Include breaking changes if applicable

**Example Output:**

```
feat(auth): add OAuth2 Google provider

Implement OAuth2 authentication flow for Google sign-in.
Includes token refresh and user profile fetching.

BREAKING CHANGE: Session format changed, users must re-authenticate
Closes #123
```

### 2. Create Pull Request

**Process:**

1. **Analyze Branch Changes**

   ```bash
   git log origin/main..HEAD --oneline
   git diff origin/main...HEAD --stat
   ```

2. **Gather Context**
   - All commits since branch diverged (NOT just latest)
   - Changed files and their purposes
   - Test coverage status
   - Breaking changes or migrations needed

3. **Structure PR Documentation**
   - Reference `pr-templates.md` for formatting
   - Include: What, Why, How, Risks, Testing, Checklist
   - Add screenshots for UI changes
   - Document migration steps for breaking changes

4. **Generate GitHub CLI Command**
   ```bash
   gh pr create --title "type(scope): description" --body "$(cat <<'EOF'
   [PR body content]
   EOF
   )"
   ```

**Key Principles:**

- PR title follows Conventional Commit format
- Summarize ALL commits, not just the latest
- Include complete test plan
- Document risks and mitigations
- Make review checklist actionable

### 3. Generate Changelog

**Process:**

1. **Determine Version Range**

   ```bash
   # Since last tag
   git describe --tags --abbrev=0
   git log $(git describe --tags --abbrev=0)..HEAD

   # Between specific tags
   git log v1.0.0..v1.1.0
   ```

2. **Collect and Categorize Commits**
   - Parse commit messages
   - Group by type (feat, fix, docs, chore, refactor, test)
   - Extract breaking changes
   - Identify issue references

3. **Format Changelog**
   - Follow Keep a Changelog format
   - Add highlights for major features
   - Document breaking changes prominently
   - Include upgrade notes
   - Credit contributors

4. **Output Structure**

   ```markdown
   ## [Version] - YYYY-MM-DD

   ### Highlights

   - Major features and breaking changes

   ### Features

   - Individual features

   ### Bug Fixes

   - Individual fixes

   ### Breaking Changes

   - Migration guide

   ### Credits

   - Contributors
   ```

## Progressive Disclosure

### Basic Level

- Simple commit messages (type: subject)
- Basic PR structure (What, Why, How)
- Simple changelog (grouped by type)

### Intermediate Level

- Scoped commits (type(scope): subject)
- PR with testing and risk sections
- Changelog with highlights and upgrade notes

### Advanced Level

- Multi-paragraph commit bodies
- Breaking change documentation
- Comprehensive PRs with screenshots and migration guides
- Changelog with semantic versioning and contributor attribution

## Integration with Agents

This skill works seamlessly with:

- **reviewer agent** - Analyzes changes and determines commit type/scope
- **tester agent** - Documents test coverage for PRs
- **docs-writer agent** - Formats final messages and documentation

Agents provide the "persona" and context, this skill provides the "procedure"
and standards.

## Quality Checklist

Before finalizing any git operation:

**Commits:**

- [ ] Type is correct (see commit-patterns.md)
- [ ] Scope is appropriate (if applicable)
- [ ] Subject is imperative, lowercase, <72 chars
- [ ] Breaking changes are documented
- [ ] Issue references included

**Pull Requests:**

- [ ] Title follows Conventional Commit format
- [ ] All commits since branch diverged are summarized
- [ ] What/Why/How sections are complete
- [ ] Risks identified and mitigations documented
- [ ] Test plan is comprehensive
- [ ] Breaking changes documented with migration steps
- [ ] Checklist items are actionable

**Changelogs:**

- [ ] Version follows semantic versioning
- [ ] Changes grouped by type
- [ ] Highlights section for major items
- [ ] Breaking changes prominently documented
- [ ] Upgrade notes included
- [ ] Contributors credited

## Reference Files

- `commit-patterns.md` - Detailed commit type reference and examples
- `pr-templates.md` - PR structure templates and examples
- `scripts/analyze-diff.sh` - Git diff analysis helper

## Best Practices

1. **Commits should tell a story** - Each commit is a chapter
2. **PRs should provide complete context** - Reviewers shouldn't need to ask
   "why?"
3. **Changelogs should guide users** - Focus on user impact, not implementation
   details
4. **Be consistent** - Use the same patterns across all git operations
5. **Reference issues** - Link commits and PRs to issue tracker

## Common Patterns

### Monorepo Scopes

```
feat(api): add user endpoint
feat(web): add user profile page
feat(mobile): add profile screen
```

### Breaking Changes

```
feat(auth)!: migrate to JWT

BREAKING CHANGE: Cookie-based auth removed.
Users must obtain JWT tokens via /auth/token endpoint.

Migration: Update client to store tokens in localStorage
```

### Dependency Updates

```
chore(deps): update dependencies

- Update React 17 -> 18
- Update TypeScript 4.9 -> 5.0
- Security: patch vulnerable packages
```

### Documentation

```
docs(readme): add installation guide

Includes:
- Prerequisites
- Step-by-step setup
- Troubleshooting section
```

## Tips for Success

- **Small, focused commits** are easier to review and revert
- **Descriptive PR titles** appear in changelogs and git history
- **Test coverage** in PRs shows you've validated changes
- **Breaking changes** should always include migration steps
- **Scope usage** helps teams understand impact at a glance

---

**Skill Version:** 1.0 **Compatible With:** Claude Code, Claude.ai, Claude Agent
SDK **Last Updated:** 2025-11-08
