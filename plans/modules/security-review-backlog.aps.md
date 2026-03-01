<!--
APS Module: Security Review Backlog
====================================
Unfixed or partially fixed security findings from the 2026-02-06 adversarial
code review (REVIEW.md) that were not tracked by the cli-hardening module.
These are cross-package security items — not CLI-specific.

Scope: SECB (security backlog)
-->

# Security Review Backlog — Adversarial Review Findings

| ID   | Owner | Status      |
| ---- | ----- | ----------- |
| SECB | —     | In Progress (2/8) |
<!-- Complete: SECB-001, SECB-003 -->

## Purpose

Track unfixed and partially fixed security findings from the 2026-02-06
adversarial code review (`plans/reviews/REVIEW.md`). The cli-hardening module
addressed CLI-specific findings (66 tasks, all complete). This module covers
the remaining cross-package findings that were either missed or partially
remediated.

## In Scope

- Path traversal in bundle manifests and validation paths
- File size limits and DoS protections in adapter parsers
- Regex denial-of-service in adapter parsers
- Information disclosure via file existence probing
- Command parser detection bypass
- Input validation (email, metadata schemas)
- Prototype pollution via untyped metadata records

## Out of Scope

- CLI-specific findings (tracked in cli-hardening.aps.md, all complete)
- MCP server findings (all fixed per REVIEW.md)
- VS Code extension findings (all fixed per REVIEW.md)
- Runtime H1-H5 findings (all fixed per REVIEW.md)
- New feature work or architecture changes

## Interfaces

**Depends on:**

- `packages/anvil/policy` — bundle verification
- `packages/adapters` — format parsers and file discovery
- `packages/aps` — validator file link probing
- `packages/anvil/runtime` — command parser
- `apps/website` — waitlist API
- `packages/anvil/contracts` — Zod schemas

**Exposes:**

- Hardened input validation across all packages
- Consistent DoS protection at parser boundaries
- Reduced information disclosure surface

## Prior Art

- **cli-hardening.aps.md** (Complete, 66/66) addressed all CLI and MCP server
  critical/high findings
- **REVIEW.md** items marked with ✅ were fixed during the original hardening
  pass
- Several findings below were partially fixed but not annotated in REVIEW.md

## Ready Checklist

Change status to **Ready** when:

- [x] All findings verified against current codebase (2026-03-01)
- [x] Already-fixed items identified and annotated
- [ ] Priority ordering agreed
- [ ] No overlap with in-flight work

---

## Tasks

### Already Fixed (SECB-001, SECB-003)

### SECB-001: Annotate Policy H1 bundle manifest path traversal as fixed

- **Intent:** Update REVIEW.md to reflect that Policy H1 (path traversal via
  bundle manifest filenames) has been fixed
- **Expected Outcome:** REVIEW.md has a ✅ annotation on Policy H1 noting the
  dual-layer path containment added at `bundle-verifier.ts:305-328` (rejects
  absolute paths and `..`-relative paths, then verifies resolved path stays
  within bundle directory)
- **Validation:** Manual review of REVIEW.md and source code
- **Files:** `plans/reviews/REVIEW.md`,
  `packages/anvil/policy/src/bundle-verifier.ts`
- **Dependencies:** None (code fix already in place)
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Notes:** Fixed silently — dual-layer defence: pre-check rejects absolute
  and `..` paths, post-join `resolve()` containment check against canonical
  bundle root.
- **Origin:** REVIEW.md Policy H1

---

### SECB-003: Annotate Adapters H3 regex DoS as mostly fixed

- **Intent:** Update REVIEW.md to reflect that Adapters H3 (regex DoS) has been
  substantially mitigated
- **Expected Outcome:** REVIEW.md annotation on Adapters H3 noting: BMAD utils
  now uses bounded quantifiers `{1,200}` on all broad patterns; SpecKit parser
  has 2MB input cap limiting blast radius; one residual lazy `[\s\S]*?` in
  SpecKit code-block regex at ~line 300
- **Validation:** Manual review of REVIEW.md and source code
- **Files:** `plans/reviews/REVIEW.md`, `packages/adapters/src/bmad/utils.ts`,
  `packages/adapters/src/speckit/parser.ts`
- **Dependencies:** None (code fixes already in place)
- **Confidence:** high
- **Priority:** Low
- **Status:** Complete
- **Notes:** BMAD fully fixed with bounded quantifiers. SpecKit mostly fixed
  with 2MB cap + bounded patterns. One residual lazy `[\s\S]*?` in code-block
  regex — low risk given input cap.
- **Origin:** REVIEW.md Adapters H3

---

### Open Findings (SECB-002, SECB-004 through SECB-008)

### SECB-002: Add file size limit to generic adapter parser

- **Intent:** Prevent DoS via massive content in the generic format adapter
  parser, completing the partial fix from Adapters H2
- **Expected Outcome:** `packages/adapters/src/generic/parser.ts` enforces a
  `MAX_INPUT_SIZE` check (2MB, consistent with SpecKit and BMAD parsers) at
  the entry point before parsing; oversized content produces a clear error
- **Validation:** Unit test with content exceeding 2MB throws an error;
  existing tests continue to pass
- **Files:** `packages/adapters/src/generic/parser.ts`
- **Dependencies:** None (SpecKit and BMAD already have this pattern)
- **Confidence:** high
- **Priority:** High
- **Status:** Ready
- **Notes:** file-discovery.ts already has `MAX_FILE_SIZE_BYTES` (2MB).
  SpecKit and BMAD parsers already check `MAX_INPUT_SIZE`. Generic parser is
  the only remaining entry point without a size guard.
- **Origin:** REVIEW.md Adapters H2

---

### SECB-004: Restrict APS validator file probing to project directory

- **Intent:** Prevent information disclosure via the APS validator's
  `accessSync()` call on user-controlled link paths
- **Expected Outcome:** `packages/aps/src/validator/index.ts` restricts the
  `validateFileExists()` function to only probe paths within the project
  base directory; links pointing outside the project (absolute paths, `../`
  escapes) are rejected with a validation error without probing the filesystem;
  the error message does not reveal whether the external path exists
- **Validation:** Unit test with a link to `/etc/passwd` produces a
  "path escapes project directory" error without checking filesystem existence
- **Files:** `packages/aps/src/validator/index.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** High
- **Status:** Ready
- **Notes:** `resolvePath()` already rejects absolute paths and validates
  resolved paths stay within `baseDir` for module path resolution (H1 fix).
  The same pattern should be applied to link validation.
- **Origin:** REVIEW.md APS H2

---

### SECB-005: Harden command parser interpreter detection

- **Intent:** Document the limitations of regex-based command detection and
  add defence-in-depth to reduce false negatives
- **Expected Outcome:** `extractInterpreterCommand()` at
  `command-parser.ts:160-176` either: (a) adds detection for common evasion
  patterns (string concatenation, hex encoding, template literals, `eval`),
  or (b) the function is documented as best-effort with a code comment and
  the gate check result includes a confidence indicator; the parser does not
  claim to detect all subprocess invocations
- **Validation:** Existing tests pass; if (a), new tests cover at least 3
  evasion patterns
- **Files:** `packages/anvil/runtime/src/gate/parsers/command-parser.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Ready
- **Notes:** Full detection of obfuscated subprocess calls is an unsolvable
  problem at the regex level. The pragmatic fix is either expanding the
  heuristic or clearly documenting the limitation so consumers don't treat
  the result as authoritative.
- **Origin:** REVIEW.md Runtime H6

---

### SECB-006: Replace weak email regex in waitlist endpoint

- **Intent:** Replace the permissive email regex with proper validation to
  reject malformed addresses
- **Expected Outcome:** `apps/website/app/api/waitlist/route.ts` uses
  `z.string().email()` (Zod) or an equivalent strict validator instead of
  the manual regex; addresses like `a@b.c` are rejected; a maximum length
  is enforced
- **Validation:** Unit test confirming `a@b.c` is rejected and
  `user@example.com` is accepted
- **Files:** `apps/website/app/api/waitlist/route.ts`
- **Dependencies:** None (Zod is already a project dependency)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Ready
- **Origin:** REVIEW.md Website M1

---

### SECB-007: Tighten z.unknown() in APS contract metadata schemas

- **Intent:** Reduce prototype pollution risk and improve type safety in APS
  contract metadata fields
- **Expected Outcome:** The `z.record(z.string(), z.unknown())` patterns at
  `aps.schema.ts` lines 32, 57, 67, 153 are replaced with either:
  (a) `z.record(z.string().refine(k => !['__proto__', 'constructor', 'prototype'].includes(k)), z.unknown())`
  to reject dangerous keys, or (b) a more specific schema that types the
  expected metadata structure; the `.strict()` at root level is preserved
- **Validation:** Unit test confirming `__proto__` key in metadata is rejected;
  existing tests pass
- **Files:** `packages/anvil/contracts/src/schemas/aps.schema.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Ready
- **Origin:** REVIEW.md Contracts M1/M2

---

### SECB-008: Fix residual SpecKit code-block regex DoS

- **Intent:** Replace the lazy `[\s\S]*?` in the SpecKit code-block regex with
  a bounded or non-backtracking alternative
- **Expected Outcome:** The code-block extraction regex at
  `speckit/parser.ts:~300` uses a bounded pattern (e.g., character class with
  length limit, or a non-regex parser for fenced code blocks) that cannot
  cause catastrophic backtracking regardless of input
- **Validation:** Existing parser tests pass; a test with a 1MB input
  containing unclosed code fences completes in under 100ms
- **Files:** `packages/adapters/src/speckit/parser.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Ready
- **Notes:** The 2MB input cap limits the practical blast radius. This is a
  defence-in-depth improvement rather than an urgent fix.
- **Origin:** REVIEW.md Adapters H3 (residual)
