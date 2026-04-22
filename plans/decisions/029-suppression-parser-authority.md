# ADR-029: Suppression Parser Authority for New Surfaces

## Status

Proposed

## Date

2026-04-22

## Context

The [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
council review surfaced a parser-authority conflict (council finding C-025):

- The council-reviewer claimed
  `packages/anvil/core/src/suppression/parser.ts` is the suppression
  parser and is TS-comment-only.
- The kernel-maintainer claimed the Rust scanner's
  `parse_suppression` function in
  `crates/anvil-checks/src/antipattern/scanner.rs` already handles
  multiple comment styles (`//`, `#`, `/*`, `<!--`, `--`).

Both reviewers were right — there are two suppression parsers in two
layers. The spec needs a single authoritative answer for new Track 3
surfaces (which need `--`, `#`, `<!--` styles) and Track 4 packs (which
inherit substrate-language comment styles).

This ambiguity is gating: every Track 3 surface module's Ready Checklist
references "suppression-parser authority decision per council C-025" as a
prerequisite. Without the ADR, each surface module can plausibly point at
either parser.

ADR-026 (Rust scanner authoritative) already established that the Rust
scanner is the authoritative analysis surface and that the TS scanner
exists only for in-process IDE/MCP surfaces until a napi-rs migration
retires it. This ADR extends that direction to suppression parsing.

## Decision

For all new Track 3 governance surfaces and all new Track 4 semantic
packs, the **Rust suppression parser** —
`parse_suppression` in `crates/anvil-checks/src/antipattern/scanner.rs`
— is **authoritative**.

- New comment styles (e.g. `--` for SQL, `<!--` for HTML/markdown blocks)
  are added to the Rust parser, not the TS one.
- The TS suppression parser at `packages/anvil/core/src/suppression/parser.ts`
  receives **no new comment style additions**. It stays as-is to support
  the IDE/MCP surfaces that still use the TS scanner per ADR-026, and is
  retired alongside the TS scanner when the napi-rs migration completes.
- Suppression rule definitions (the `<ID>: <reason>` shape, the
  `@anvil-ignore-until DATE` extension from ADR-004) live as a single
  source-of-truth schema. Both parsers consume the same schema today;
  after retirement, only the Rust parser remains.
- The Track 5 markdown governance crate ([ADR-028](028-markdown-governance-crate.md))
  reuses the Rust suppression parser via the
  `crates/anvil-checks` dependency — it does not implement its own.

## Rationale

ADR-026 already chose Rust as the authoritative scanner. Suppression
parsing is part of the scanner's job (the suppression parser runs against
the same comment tokens the scanner extracts). Splitting authority would
create the exact dual-source-of-truth problem ADR-026 was meant to retire.

The TS suppression parser cannot be deleted yet because it is still
load-bearing for the IDE/MCP surfaces ADR-026 explicitly preserves until
napi-rs migration. But adding new comment styles to it would extend the
retirement window — every new style added is more work to migrate later.

The "no new styles in TS" rule is the one operational constraint that
prevents the migration window from drifting outward.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Rust scanner suppression parser is authoritative; no new styles in TS** *(chosen)* | Aligns with ADR-026; single source of truth for new work; bounded TS retirement window | Existing TS-only consumers (IDE/MCP) cannot use new comment styles until napi-rs migration; acceptable since IDE/MCP currently only see TS files |
| Both parsers stay; new styles added to both | No coordination problem during transition | Doubles the work of every new surface; perpetuates the dual-source-of-truth problem ADR-026 was designed to retire |
| Move TS parser to call into Rust via napi-rs immediately | Single parser implementation | Forces napi-rs migration to land before any Track 3 surface can ship — unacceptable schedule coupling |
| TS parser becomes authoritative; Rust parser delegates | Backwards-compatible with existing TS consumers | Reverses ADR-026; re-anchors authority in the layer being retired |

## Consequences

- **Positive:**
  - One place to add new comment styles for new surfaces.
  - Bounded TS retirement window — the parser cannot grow during
    retirement.
  - Track 3 surface modules' Ready Checklists unblock without further
    suppression-parser deliberation.
  - Markdown governance crate ([ADR-028](028-markdown-governance-crate.md))
    has a clear suppression parser to depend on.

- **Negative:**
  - IDE/MCP surfaces using the TS scanner cannot recognise new comment
    styles (e.g. SQL `--` suppressions) until napi-rs migration
    completes. Acceptable because those surfaces today only see TS files,
    which use `//` (already supported on both sides).
  - Anyone adding a new comment style to the TS parser by reflex needs
    to be reminded of this ADR. Mitigate via comment in the TS parser
    file pointing at this ADR.

- **Risks:**
  - Misaligned suppression behaviour between TS and Rust parsers during
    the transition (council C-025 raised this as the concrete risk).
    Mitigation: the suppression rule schema (`<ID>: <reason>`,
    `@anvil-ignore-until DATE`) stays shared; only comment-style
    additions are scoped to Rust.

- **Mitigations:**
  - Add a comment at the top of `packages/anvil/core/src/suppression/parser.ts`
    pointing at this ADR before the next Track 3 module starts.
  - Add an `anvil-checks` test that fails if a new comment-style enum
    variant lands without a corresponding test in the Rust scanner.

## References

- Related ADRs: ADR-004 (suppression syntax — defines the rule schema both
  parsers consume), ADR-026 (Rust scanner authoritative — the load-bearing
  parent decision), ADR-028 (markdown governance crate — consumes the
  Rust suppression parser)
- Spec: [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
  §16.5 minor finding, council finding C-025
- APS modules: every Track 3 surface module
  ([surface-sql-migrations](../modules/surface-sql-migrations.aps.md),
  [surface-github-actions](../modules/surface-github-actions.aps.md),
  [surface-dockerfile](../modules/surface-dockerfile.aps.md),
  [surface-shell](../modules/surface-shell.aps.md),
  [surface-env-files](../modules/surface-env-files.aps.md)) and
  [markdown-governance](../modules/markdown-governance.aps.md) reference
  this ADR as a Ready prerequisite
- Code: `parse_suppression` in
  `crates/anvil-checks/src/antipattern/scanner.rs`,
  `packages/anvil/core/src/suppression/parser.ts`
