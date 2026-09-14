# HTML/CSS Support & Multi-Language Foundation Status Review (2026-02-05)

## Question

Are the features listed in `plans/index.aps.md` as **v1.3 HTML/CSS Support &
Multi-Language Foundation** still outstanding?

## Determination

Yes — they are still outstanding in the current codebase state.

## Evidence

### 1) Index marks HTML/CSS tasks as planned, not complete

`plans/index.aps.md` lists all `HTMLCSS-001` through `HTMLCSS-007` tasks as
`Planned`, and lists multi-language modules as `Placeholder`.

### 2) Core anti-pattern registry still stops at AP-007

The built-in anti-pattern catalogue in
`packages/anvil/core/src/antipattern/patterns.ts` currently defines AP-001
through AP-007 only, with no AP-008 through AP-013 HTML/CSS patterns added.

### 3) CLI source analysis still uses hard-coded JS/TS extensions

`apps/anvil-cli/src/commands/check.ts` still uses a hard-coded
`ANALYSABLE_EXTENSIONS` list of JS/TS variants (`.ts`, `.tsx`, `.js`, `.jsx`,
`.mjs`, `.cjs`), with no HTML/CSS extensions.

### 4) VS Code embedded analysis also limits to JS/TS extensions

`packages/vscode-extension/src/services/embeddedAnalysis.ts` only considers
JS/TS extensions as analysable and does not include `.html`, `.css`, `.scss`,
or `.less`.

### 5) Suppression parser does not include HTML comment syntax

`packages/anvil/core/src/suppression/parser.ts` recognises `//` and `/* */`
comment forms for suppressions, but no `<!-- @anvil-ignore ... -->` extraction
or HTML-aware file-scope handling.

## Conclusion

Based on planning metadata plus implementation checks, the v1.3 HTML/CSS work
and the dependent multi-language foundation are still outstanding.
