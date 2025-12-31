<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Documentation Polish

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| DOCS  | —     | high     | Ready  |

## Purpose

Ensure documentation enables successful v1.0 adoption. Users must be able to get
value from Anvil within 5 minutes of discovering it. Poor documentation is the
silent killer of adoption.

**Problem:** Current documentation is:

- Stale (written for earlier architecture)
- Incomplete (missing command reference)
- Lacks compelling demo material
- Error messages don't guide users to solutions

**Solution:** Comprehensive documentation refresh covering:

- Quick Start Guide (5-minute path to value)
- Complete command reference
- Demo material (GIF/video showing real catches)
- Error message audit and improvement
- Troubleshooting guide for common issues

## In Scope

- Quick Start Guide rewrite
- User Guide with complete command reference
- Demo GIF/video creation
- Error message audit and improvement
- Troubleshooting guide update
- README refresh

## Out of Scope

- API documentation (auto-generated from types)
- Architecture deep-dives (existing docs are adequate)
- Contributor guide (post-v1)
- Internationalisation

## Interfaces

**Depends on:**

- All v1.0 features complete and stable
- CLI commands finalised

**Exposes:**

- `docs/QUICK_START.md` — 5-minute guide
- `docs/USER_GUIDE.md` — Complete reference
- `docs/TROUBLESHOOTING.md` — Common issues
- `README.md` — Project overview with demo
- Demo assets in `docs/assets/`

## Acceptance Criteria

- [ ] New user can run `anvil check` on their project within 5 minutes
- [ ] All CLI commands documented with examples
- [ ] Demo GIF shows Anvil catching a real issue
- [ ] Error messages include actionable next steps
- [ ] Troubleshooting covers top 10 setup issues
- [ ] README has clear value proposition and demo

## Tasks

### DOCS-001: Quick Start Guide

- **Intent:** Rewrite QUICK_START.md for v1.0 `anvil check` workflow
- **Expected Outcome:** User achieves first value in 5 minutes
- **Scope:** `docs/QUICK_START.md`
- **Non-scope:** Advanced features
- **Files:**
  - `docs/QUICK_START.md`
- **Dependencies:** —
- **Validation:** New user test (someone unfamiliar tries the guide)
- **Confidence:** high

### DOCS-002: User Guide command reference

- **Intent:** Document all CLI commands with examples
- **Expected Outcome:** Complete reference for every command and flag
- **Scope:** `docs/USER_GUIDE.md`
- **Non-scope:** Architecture explanation
- **Files:**
  - `docs/USER_GUIDE.md`
- **Dependencies:** DOCS-001
- **Validation:** Every `--help` output has corresponding docs
- **Confidence:** high

### DOCS-003: Demo material creation

- **Intent:** Create compelling demo showing Anvil in action
- **Expected Outcome:** GIF/video showing real issue detection
- **Scope:** `docs/assets/`, `README.md`
- **Non-scope:** Marketing copy
- **Files:**
  - `docs/assets/demo.gif`
  - `README.md` — Embed demo
- **Dependencies:** DOCS-001
- **Validation:** Demo clearly shows value proposition
- **Confidence:** medium (creative work)

### DOCS-004: Error message audit

- **Intent:** Review all error messages for actionability
- **Expected Outcome:** Every error tells user what to do next
- **Scope:** Error messages throughout codebase
- **Non-scope:** Rewriting core logic
- **Files:**
  - Various files with error messages
  - `docs/ERROR_MESSAGES.md` — Error reference
- **Dependencies:** —
- **Validation:** Spot-check 20 error messages
- **Confidence:** high

### DOCS-005: Troubleshooting guide

- **Intent:** Document solutions to common setup issues
- **Expected Outcome:** Top 10 issues have clear solutions
- **Scope:** `docs/TROUBLESHOOTING.md`
- **Non-scope:** Edge cases
- **Files:**
  - `docs/TROUBLESHOOTING.md`
- **Dependencies:** DOCS-001
- **Validation:** Covers issues seen in testing
- **Confidence:** high

### DOCS-006: README refresh

- **Intent:** Update README with v1.0 features and demo
- **Expected Outcome:** Clear value prop, quick start link, demo embed
- **Scope:** `README.md`
- **Non-scope:** Detailed documentation (link to docs/)
- **Files:**
  - `README.md`
- **Dependencies:** DOCS-003
- **Validation:** README passes "5-second test" (value clear immediately)
- **Confidence:** high

## Decisions

**D-DOCS-001:** Demo as GIF, not video

- **Rationale:** GIFs autoplay in GitHub, no hosting needed, universal support
- **Alternatives:** YouTube video, Loom
- **Trade-offs:** Lower quality, but higher engagement

**D-DOCS-002:** Single USER_GUIDE.md, not separate pages

- **Rationale:** Easy to search, print, share. Avoids navigation overhead.
- **Alternatives:** Docs site with separate pages
- **Trade-offs:** Large single file, but simpler maintenance

## Notes

**Quick Start structure:**

1. Install (30 seconds)
2. Run first check (60 seconds)
3. Understand output (60 seconds)
4. Configure baseline (120 seconds)
5. Celebrate first catch (instant)

**Error message pattern:**

```
Error: [WHAT] — [WHY]

[WHAT TO DO]

Example: ...

More info: docs/TROUBLESHOOTING.md#[section]
```

**Demo scenarios:**

1. Developer adds `as any` — Anvil catches it
2. Developer imports across boundary — Anvil warns
3. Developer suppresses with reason — Audit trail shown
4. PR shows Anvil check results — CI integration demo

**Success metrics:**

- Time to first value: < 5 minutes
- Documentation NPS: > 40
- Support requests drop 50% after docs ship
