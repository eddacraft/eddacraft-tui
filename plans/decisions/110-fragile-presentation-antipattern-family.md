# ADR-110: Fragile-presentation anti-pattern family

## Status

Proposed

## Date

2026-07-16

## Context

A 2026-07-16 triage of a third-party "anti-slop design law" prompt catalogue
(~150 rules describing AI-generated-UI tells) shortlisted ten candidate rules
and assessed them against the anti-pattern registry (44 rules across ten
families at the time of triage — no presentation coverage; a deliberately
stacked probe file returned zero findings). The full assessment and work item
are recorded in CIB-198. The triage split cleanly:

- **Nine of ten shortlisted rules are render-time or judgement concerns** —
  contrast ratios, clipped text, column alignment, dead controls, image
  seams, palette/typography cohesion. A single-file source scanner cannot see
  computed styles or rendered layout; these are out of model.
- **One rule is a genuine, statically detectable correctness trap:** content
  authored invisible (`opacity: 0`) whose visibility depends on an entrance
  animation firing — the motion/framer-motion idiom
  `initial={{ opacity: 0, ... }}`. A `prefers-reduced-motion` setting, a
  backgrounded tab, a hydration miss, or failed JavaScript leaves the section
  permanently blank, silently. The construction is single-line,
  deterministic, and RE2-expressible.

The scope guard (`docs/vision/anvil-scope-guard.md`) admits security
anti-patterns "only as syntactic-construction smells" (ADR-087,
`insecure-construction`) and is silent on presentation correctness. The
scanner is regex plus same-line post-filter for non-Rust rules, with AST
detection reserved and kept off the daemon hot path (ADR-071). AI agents
apply entrance animations by default, so this trap ships at scale precisely
in AI-assisted development — Anvil's charter. A decision is needed on whether
any UI-facing rule belongs in the scanner, and where the line sits, before
the family ships (CIB-198 is Ready).

## Decision

1. **Presentation-correctness construction smells are in scope** for the
   anti-pattern scanner, under the same bar ADR-087 set for the security
   class: single-file, single-line (or existing AST path) deterministic
   detection of a construction whose failure mode is a **correctness loss**
   (content unreachable or invisible), with the human confirming the contract
   through the standard suppression flow.
2. Create the **`fragile-presentation` family** (`patterns/fragile-presentation/`)
   with its own first-class category — `AntiPatternCategory::FragilePresentation`
   in Rust, `fragile-presentation` in the TS `KNOWN_CATEGORIES` — so rules do
   not fall back to `code-quality`. First rule **FRAG-001** per CIB-198:
   severity `warning`, confidence `medium`, enabled by default, JS/TS
   extensions only.
3. **Design-taste rules are rejected** for the catalogue. Font blacklists,
   "gradient = slop" palette rules, and layout-fashion skeletons from the
   same source are non-deterministic judgement, fashion-bound, and fail the
   determinism principle. They do not enter the scanner in any form.
4. **Render-time checks remain out of scope** for antipattern-scan. Contrast,
   clipping, alignment, dead controls, and image seams need a rendered DOM.
   If ever pursued, that is a new check category with its own ADR, justified
   against the existing accessibility ecosystem (axe-core, Lighthouse) first.

## Rationale

FRAG-001's failure mode is objective (a blank section), the construction is
lexically pinpointable, and it is a distinctly AI-generation-shaped defect —
the same shape as the existing families: code that looks finished and fails
silently in an environment the author never exercised. The bar in decision
(1) is what keeps the family from becoming a design linter: a candidate rule
must name a correctness failure, not an aesthetic judgement.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| New family under the ADR-087 construction-smell bar (chosen) | Real correctness catch; deterministic; reuses existing engine; precedent for future rules | One more category across Rust/TS surfaces; warning noise on animation-heavy codebases |
| Leave it to the ESLint ecosystem | No scope stretch | No existing lint catches this idiom; repo-external and unenforced in the workspaces Anvil governs |
| Fold FRAG-001 into `unsafe-rendering` | No new category | Wrong semantics — UR is the DOM-XSS security class (ADR-087); diluting it breaks category meaning |
| Build a render-time check category now | Covers all ten shortlisted rules | Disproportionate: new engine, new dependency surface, duplicates axe-core/Lighthouse; its own ADR-sized decision |
| Reject all ten | Zero risk | Loses a cheap, real, on-charter correctness catch |

## Consequences

- **Positive:** first presentation-correctness coverage; a recorded bar for
  future FRAG rules (an SVG `text-anchor="middle"` without
  `dominant-baseline` rule is a plausible FRAG-002 candidate); the triage is
  amortised into a durable decision instead of re-litigating per rule.
- **Negative:** one more category variant across `types.rs`,
  `registry_loader.rs`, and the TS schema; potential warning noise on
  codebases where every section legitimately animates in.
- **Risks:** false-positive rate on marketing-style codebases exceeding the
  N=5% acceptance bar (#3067 precedent).
- **Mitigations:** severity `warning` (never blocks; exit-0 posture holds);
  the standard suppression directive with a reason; false-positive rate
  measured at dogfood before any severity escalation. Known gaps (multi-line
  `initial` objects, `variants`, class-toggle reveals) are documented in the
  family definition and deferred to a JS/TS AST detection path (ADR-071
  precedent) rather than stretched regexes.

## References

- Related ADRs: ADR-087 (insecure-construction; the construction-smell
  posture and scope boundary), ADR-071 (AST-aware detection off the daemon
  hot path)
- APS: CIB-198 (`plans/modules/continuous-improvement-backlog.aps.md`)
- `docs/vision/anvil-scope-guard.md` (anti-pattern scope boundary)
