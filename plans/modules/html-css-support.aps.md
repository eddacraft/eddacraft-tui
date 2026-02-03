<!--
APS Module: HTML/CSS Support
=============================
Extends Anvil's analysis to HTML and CSS files.
See: plans/aps-rules.md
-->

# HTML/CSS Support

| ID     | Owner | Status |
| ------ | ----- | ------ |
| HTMLCSS | —     | Ready  |

## Purpose

Extend Anvil to analyse HTML and CSS files alongside TypeScript/JavaScript. This
is the first non-JS language extension and serves as the proving ground for
multi-language support. HTML/CSS is the simplest case because it requires no
complex module resolution or type system analysis — just regex-based pattern
detection, simple edge extraction, and a comment syntax addition for
suppressions.

## In Scope

- Add `.html`, `.htm`, `.css`, `.scss`, `.less` to analysable extensions
- Make extension list configurable via `.anvilrc` / `anvil.config.*`
- HTML-specific anti-pattern detectors (inline styles, inline scripts, inline
  event handlers, deprecated tags)
- CSS-specific anti-pattern detectors (`!important`, CSS `@import`)
- Edge detection for HTML/CSS dependencies (`<script src>`, `<link href>`,
  `@import url()`, `url()`)
- HTML comment suppression syntax (`<!-- @anvil-ignore ... -->`)
- Update VS Code extension to trigger analysis on HTML/CSS file saves

## Out of Scope

- Template language support (Handlebars, EJS, Pug, Jinja) — future
- CSS-in-JS analysis (styled-components, Emotion) — already covered by TS
- Sass/Less compilation or variable resolution
- HTML validation (W3C compliance) — use dedicated validators
- Accessibility analysis (a11y) — use dedicated tools like axe

## Interfaces

**Depends on:**

- `save-time-trust` — runner and warning schema
- `antipattern-library` — scanner infrastructure, `AntiPattern` type
- `architecture-safety` — edge detector, layer detector
- `suppressions` — suppression parser

**Exposes:**

- HTML/CSS anti-pattern definitions (AP-008 through AP-013)
- HTML/CSS edge extraction regexes
- HTML comment suppression support
- Configurable extension list (used by all commands)

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] All tasks defined
- [x] Low coupling to existing code — additive changes only

## Tasks

### HTMLCSS-001: Make analysable extensions configurable

- **Intent:** Replace hard-coded `ANALYSABLE_EXTENSIONS` with a configurable
  list so users can opt into HTML/CSS analysis (and future languages)
- **Expected Outcome:** Extensions loaded from config with sensible defaults;
  `anvil check` respects configured extensions; CLI `--extensions` flag overrides
- **Files:** `apps/anvil-cli/src/commands/check.ts`,
  `packages/anvil/runtime/src/gate/checks/antipattern.check.ts`,
  `packages/platform/config/src/schema.ts`
- **Dependencies:** None (foundational)
- **Validation:** `pnpm -F anvil-cli test -- --testNamePattern="extension"`
- **Confidence:** high
- **Notes:** Current hard-coded list at `check.ts:15`:
  `['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']`. Keep these as defaults.
  New config field: `extensions: string[]` in anvil config schema. The
  `antipattern.check.ts` DEFAULT_CONFIG also has a hard-coded extensions list
  at line 30 that needs to read from config.

### HTMLCSS-002: HTML anti-pattern detectors

- **Intent:** Add 4 HTML-specific anti-pattern definitions to the catalogue
- **Expected Outcome:** Scanner detects inline styles, inline scripts, inline
  event handlers, and deprecated HTML tags in `.html` files
- **Files:** `packages/anvil/core/src/antipattern/patterns.ts`,
  `packages/anvil/core/src/antipattern/patterns-html.ts`
- **Dependencies:** HTMLCSS-001
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="html"`
- **Confidence:** high
- **Notes:** New patterns:
  - AP-008: Inline `style=""` attributes — `style\s*=\s*["']`
  - AP-009: Inline `<script>` blocks — `<script(?:\s[^>]*)?>(?!\s*<\/script>)`
  - AP-010: Inline event handlers — `\bon\w+\s*=\s*["']`
  - AP-011: Deprecated HTML tags — `<(?:font|center|marquee|blink|big|strike)\b`
  All patterns should use `allowlist: ['**/email/**']` since HTML emails
  legitimately use inline styles. All should be `optIn: true` initially.

### HTMLCSS-003: CSS anti-pattern detectors

- **Intent:** Add 2 CSS-specific anti-pattern definitions to the catalogue
- **Expected Outcome:** Scanner detects `!important` abuse and CSS `@import`
  (performance anti-pattern) in `.css`/`.scss`/`.less` files
- **Files:** `packages/anvil/core/src/antipattern/patterns.ts`,
  `packages/anvil/core/src/antipattern/patterns-css.ts`
- **Dependencies:** HTMLCSS-001
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="css"`
- **Confidence:** high
- **Notes:** New patterns:
  - AP-012: `!important` in CSS — `!\s*important`
  - AP-013: CSS `@import` — `@import\s+(?:url\()?["']`
  AP-012 allowlist: `['**/reset.css', '**/normalize.css']`.
  AP-013 severity: `info` (it's a performance concern, not a bug).

### HTMLCSS-004: HTML/CSS edge detection

- **Intent:** Extract dependency edges from HTML and CSS files so architecture
  boundary analysis works for HTML/CSS references
- **Expected Outcome:** Edge detector finds `<script src>`, `<link href>`,
  CSS `@import url()`, and `url()` references; edges feed into baseline
  comparison like JS imports do
- **Files:** `packages/anvil/core/src/architecture/edge-detector.ts`,
  `packages/anvil/core/src/architecture/edge-detector-html.ts`
- **Dependencies:** HTMLCSS-001
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="edge.*html"`
- **Confidence:** high
- **Notes:** New regexes alongside the existing 4 JS import regexes:
  - `<script[^>]+src\s*=\s*["']([^"']+)["']` → type: `import`
  - `<link[^>]+href\s*=\s*["']([^"']+)["']` → type: `import` (filter to
    rel="stylesheet" or .css extension)
  - `@import\s+(?:url\(\s*)?["']([^"']+)["']` → type: `import`
  - `url\(\s*["']?([^"')]+)["']?\s*\)` → type: `import` (skip data: URIs)
  The existing `resolveImportPath` already handles relative paths generically.
  External URLs (http/https) should be skipped — filter to relative paths only.

### HTMLCSS-005: HTML suppression comment syntax

- **Intent:** Add `<!-- @anvil-ignore ... -->` support to the suppression parser
- **Expected Outcome:** Suppressions in HTML comments are parsed identically to
  JS `//` and `/* */` comments; file-level, statement, and line scopes all work
- **Files:** `packages/anvil/core/src/suppression/parser.ts`
- **Dependencies:** None
- **Validation:** `pnpm -F anvil-core test -- --testNamePattern="suppression.*html"`
- **Confidence:** high
- **Notes:** Add one regex to `extractSuppressionComment` at `parser.ts:70`:
  ```
  /<!--\s*(@anvil-ignore[^-]*?)\s*-->/
  ```
  The `determineScope` function at line 39 needs a small update: check for
  `<!--` in addition to `//` and `/*` when determining file-level scope
  (line 47). CSS `/* */` already works — no change needed for CSS files.

### HTMLCSS-006: VS Code extension HTML/CSS trigger

- **Intent:** Update VS Code extension to run analysis on HTML/CSS file saves
- **Expected Outcome:** Saving an `.html` or `.css` file triggers anti-pattern
  detection and shows diagnostics in the Problems panel
- **Files:** `packages/vscode-extension/src/extension.ts`
- **Dependencies:** HTMLCSS-001, HTMLCSS-002, HTMLCSS-003
- **Validation:** Manual testing in VS Code
- **Confidence:** high
- **Notes:** The extension likely has a document selector filter that restricts
  to `typescript`/`javascript` language IDs. Add `html`, `css`, `scss`, `less`.

### HTMLCSS-007: Documentation and tests

- **Intent:** Document HTML/CSS support in docs-site and ensure test coverage
- **Expected Outcome:** Quick-start mentions HTML/CSS; pattern reference
  includes AP-008 through AP-013; all new code has >90% coverage
- **Files:** `apps/docs-site/docs/anvil/`, test files alongside source
- **Dependencies:** HTMLCSS-002, HTMLCSS-003, HTMLCSS-004, HTMLCSS-005
- **Validation:** `pnpm test` all green; docs build succeeds
- **Confidence:** high

## Execution

Steps: [../execution/HTMLCSS-001.steps.md](../execution/HTMLCSS-001.steps.md)

## Risks

| Risk                          | Impact | Mitigation                                    |
| ----------------------------- | ------ | --------------------------------------------- |
| False positives on templates  | Medium | Allowlists for email templates; opt-in default |
| CSS `url()` noise             | Medium | Skip external URLs, data: URIs, fonts          |
| Scope creep to template langs | Low    | Strict out-of-scope boundary                   |

## Decisions

- **D-001:** All HTML/CSS patterns are opt-in by default to avoid noise on
  projects that don't care about frontend patterns
- **D-002:** Edge detection skips external URLs (http/https) — only relative
  paths create architecture edges
- **D-003:** No AST/parser dependency — all detection is regex-based, consistent
  with existing TS/JS patterns
