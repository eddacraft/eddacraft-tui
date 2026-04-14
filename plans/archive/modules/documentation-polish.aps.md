<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Documentation Polish

| Scope | Owner | Priority | Status   |
| ----- | ----- | -------- | -------- |
| DOCS  | —     | high     | Complete |

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
- Demo material (code examples in README)
- Error message audit and improvement
- Troubleshooting guide for common issues

## In Scope

- Quick Start Guide rewrite
- User Guide with complete command reference
- Demo examples in README
- Error message audit and improvement
- Troubleshooting guide update
- README refresh

## Out of Scope

- API documentation (auto-generated from types)
- Architecture deep-dives (existing docs are adequate)
- Contributor guide (post-v1)
- Internationalisation
- Demo GIF/video (descoped — code examples sufficient)

## Interfaces

**Depends on:**

- All v1.0 features complete and stable
- CLI commands finalised

**Exposes:**

- `apps/docs-site/docs/anvil/quickstart.md` — 5-minute guide
- `apps/docs-site/docs/anvil/` — Complete documentation
- `apps/docs-site/docs/anvil/operations/troubleshooting.md` — Common issues
- `README.md` — Project overview with demo examples

## Acceptance Criteria

- [x] New user can run `anvil check` on their project within 5 minutes
- [x] All CLI commands documented with examples
- [x] README shows Anvil in action with code examples
- [x] Error messages include actionable next steps
- [x] Troubleshooting covers common setup issues
- [x] README has clear value proposition and quick start

## Tasks

### DOCS-001: Quick Start Guide

- **Intent:** Rewrite quickstart for v1.0 `anvil check` workflow
- **Expected Outcome:** User achieves first value in 5 minutes
- **Scope:** `apps/docs-site/docs/anvil/quickstart.md`
- **Non-scope:** Advanced features
- **Files:**
  - `apps/docs-site/docs/anvil/quickstart.md`
- **Dependencies:** —
- **Validation:** New user test (someone unfamiliar tries the guide)
- **Confidence:** high
- **Status:** Complete

### DOCS-002: User Guide command reference

- **Intent:** Document all CLI commands with examples
- **Expected Outcome:** Complete reference for every command and flag
- **Scope:** `apps/docs-site/docs/anvil/`
- **Non-scope:** Architecture explanation
- **Files:**
  - `apps/docs-site/docs/anvil/` (various command docs)
- **Dependencies:** DOCS-001
- **Validation:** Every `--help` output has corresponding docs
- **Confidence:** high
- **Status:** Complete

### DOCS-003: Demo material creation

- **Intent:** Create compelling demo showing Anvil in action
- **Expected Outcome:** README with code examples showing real issue detection
- **Scope:** `README.md`
- **Non-scope:** Animated GIF/video (descoped)
- **Files:**
  - `README.md` — Code examples embedded
- **Dependencies:** DOCS-001
- **Validation:** Demo clearly shows value proposition
- **Confidence:** high
- **Status:** Complete (code examples instead of GIF)

### DOCS-004: Error message audit

- **Intent:** Review all error messages for actionability
- **Expected Outcome:** Every error tells user what to do next
- **Scope:** Error messages throughout codebase
- **Non-scope:** Rewriting core logic
- **Files:**
  - Various files with error messages
- **Dependencies:** —
- **Validation:** Spot-check error messages
- **Confidence:** high
- **Status:** Complete

### DOCS-005: Troubleshooting guide

- **Intent:** Document solutions to common setup issues
- **Expected Outcome:** Common issues have clear solutions
- **Scope:** `apps/docs-site/docs/anvil/operations/troubleshooting.md`
- **Non-scope:** Edge cases
- **Files:**
  - `apps/docs-site/docs/anvil/operations/troubleshooting.md`
- **Dependencies:** DOCS-001
- **Validation:** Covers issues seen in testing
- **Confidence:** high
- **Status:** Complete

### DOCS-006: README refresh

- **Intent:** Update README with v1.0 features and demo
- **Expected Outcome:** Clear value prop, quick start link, demo examples
- **Scope:** `README.md`
- **Non-scope:** Detailed documentation (link to docs-site)
- **Files:**
  - `README.md`
- **Dependencies:** DOCS-003
- **Validation:** README passes "5-second test" (value clear immediately)
- **Confidence:** high
- **Status:** Complete

## Decisions

**D-DOCS-001:** Code examples over animated GIF

- **Rationale:** Code examples are more maintainable, load instantly, and work
  everywhere. GIFs require tooling to create and update, may become stale.
- **Alternatives:** Animated GIF, Asciinema, YouTube video
- **Trade-offs:** Less visual impact, but easier maintenance

**D-DOCS-002:** Documentation in docs-site, not root docs/

- **Rationale:** Single source of truth for public docs. Root `docs/` is now
  internal engineering documentation only.
- **Alternatives:** Duplicate in both locations
- **Trade-offs:** Users must visit docs-site for full docs, but README provides
  quick start

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

More info: https://docs.anvil.dev/troubleshooting#[section]
```

**Success metrics:**

- Time to first value: < 5 minutes
- Documentation NPS: > 40
- Support requests drop 50% after docs ship

## Completion Notes

Module completed as part of v1.0 release. Documentation now lives in:

- **Public docs:** `apps/docs-site/docs/anvil/`
- **Internal docs:** `docs/` (engineering reference only)
- **README:** Root `README.md` with quick start and examples

The demo GIF (originally DOCS-003) was descoped in favour of inline code
examples which are easier to maintain and update as the CLI evolves.
