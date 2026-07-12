# ADR-027: Semantic Pack Architecture

## Status

Accepted (2026-04-26)

> Accepted at minimum bar per Council D recommendation; refinements to land on first implementation.

## Date

2026-04-22

## Context

The [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
defines six semantic packs as Phase 1 + Phase 2 deliverables: `pack-pulumi`,
`pack-llm-provider`, `pack-drizzle`, `pack-nextjs`, `pack-hono`, `pack-tokio`.
Each is a domain-specific rule bundle layered on an anchor language (TS or
Rust).

The council review (§16.5 #4, council finding C-018) flagged that the spec
defines what packs *do* but says nothing about how they live: where packs
sit in the crate tree, whether they have access to the symbol graph or only
file content, whether they are compiled in or dynamically loaded. Pulumi
and Drizzle in particular catch defects that pure regex cannot reach — a
`.delete()` without `.where()` requires symbol-level method-chain awareness;
the Pulumi `acl: "public-read"` rule needs to know that the call is on an
S3 resource constructor. Pack-shaped work without the architecture decision
risks each pack inventing its own access pattern.

The decision needs to be made before `pack-pulumi` writes its first task.
PACKPUL is the first pack to ship and sets the pattern every other pack
follows — this ADR is referenced from PACKPUL's Ready Checklist.

## Decision

1. **Substrate access**: packs operate on the **kernel symbol graph**, not
   on raw file content. Packs receive parsed AST + extracted symbols/imports
   from the existing kernel (`crates/anvil-kernel/src/parser/`) plus the
   architecture analysis layer for cross-file context. Content-only access
   (regex over source text) is permitted as a fallback for narrow cases
   (e.g. comment-string patterns) but is not the default.

2. **Crate location**: each pack is its own crate under
   `crates/anvil-pack-{name}/` (e.g. `crates/anvil-pack-pulumi/`,
   `crates/anvil-pack-llm-provider/`). A central registry crate
   `crates/anvil-packs/` aggregates and exposes the activated set to the
   gate runner. This matches the existing per-domain crate split
   (`anvil-checks`, `anvil-architecture`, `anvil-policy`).

3. **Activation model**: packs are **compiled in** to the `anvil` binary
   and registered through the `crates/anvil-packs/` aggregate at build
   time. No dynamic plugin loading, no separate distribution, no out-of-band
   pack installation. Per-pack feature flags from the operational supplement
   ([OPSUP](../archive/modules/operational-supplement.aps.md)) control runtime
   activation.

4. **Pack activation detection**: each pack declares the file-shape and
   import-shape signals that activate it (e.g. `pack-pulumi` activates on
   any TS file importing `@pulumi/*`; `pack-tokio` activates on any Rust
   file importing `tokio` or `tokio::*`). The check registry from OPSUP
   short-circuits work when activation signals are absent — no cost in
   repos that do not use the pack.

5. **Substrate tier gating**: pack registration enforces the substrate
   minimum tier declared in the pack module (e.g. PACKPUL declares "TS T3";
   the registry refuses to activate the pack until the TS anchor module
   reports T3 capability). The tier-check is data-driven from the LANGTS
   T3 acceptance checklist artefact.

## Rationale

Symbol-graph access is the load-bearing constraint — the design's
strategically-most-valuable packs (LLM Provider, Pulumi, Hono) all need it.
Building packs against content-only would force each one to reimplement
parser-shaped reasoning, which is the wrong abstraction layer (council
C-018 explicit framing).

Per-pack crates match the existing crate decomposition pattern; one crate
per Cargo.toml `[lib]` keeps build-graph reasoning predictable, and any
pack can be excluded from a build by removing it from
`crates/anvil-packs/Cargo.toml` dependencies.

Compiled-in activation rejects the dynamic-plugin alternative on
correctness grounds: dynamic plugins introduce ABI compatibility surface,
versioning concerns, and a runtime trust boundary Anvil does not currently
have infrastructure for. Per-pack feature flags from OPSUP cover the
"toggle a pack off" use case without the dynamic-loading complexity.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Per-pack crate, symbol-graph access, compiled in** *(chosen)* | Matches existing crate layout; gives packs the access they need; no plugin runtime to maintain | Every pack added requires a binary recompile + release; pack rule changes ride the same release cadence as the kernel |
| Single `crates/anvil-packs/src/{name}.rs` module | Simpler crate count | Loses crate-level isolation; one pack's deps leak into others; harder to exclude individual packs |
| Dynamic plugin loading (`.so` / `.wasm`) | Packs ship out-of-band; rule iteration without binary release | ABI compatibility surface; versioning hell; runtime trust boundary; cross-platform packaging cost; council §16.5 #4 explicitly flags this is unspecified — adopting it implicitly is worse than explicitly rejecting it |
| Content-only (regex / file-text) access | Simplest API for pack authors | Pulumi/Drizzle/Hono/Tokio rules cannot be expressed; design's most strategic packs would not be viable; council C-018 |

## Consequences

- **Positive:**
  - Packs get the same parser access the existing checks have.
  - Crate boundary makes excluding a pack from a build trivial.
  - No new runtime trust surface (no plugin loading).
  - Per-pack feature flags from OPSUP give the runtime toggle without
    needing dynamic loading.
  - Tier gating is data-driven from the LANGTS T3 checklist — no hard-coded
    "wait for substrate" logic in pack code.

- **Negative:**
  - Every pack iteration requires a binary release. Pack rule changes
    cannot ship out-of-band.
  - Build matrix grows with pack count.
  - Pack authors must understand the kernel symbol-graph API surface.

- **Risks:**
  - Symbol-graph API is currently TS/JS-shaped (council C-003) — packs on
    Rust substrate (`pack-tokio`) cannot land cleanly until the kernel
    extractor refactor (LANGTS prereq work) lands.
  - Pack count grows; build time grows. Mitigation: the activation
    short-circuit from OPSUP keeps runtime cost flat; build-time cost is
    accepted up to the Phase 2 set of 6.

- **Mitigations:**
  - Pack architecture is enforced in `crates/anvil-packs/` registration —
    no pack can ship without going through the registry.
  - The tier-gate refuses to activate packs on substrates not yet at
    declared tier, so a Rust pack cannot accidentally ship before Rust
    reaches T2+.

## References

- Related ADRs: ADR-026 (Rust scanner authoritative — establishes the
  kernel-side architecture this ADR builds on), ADR-014 (TS vs Rust
  language allocation)
- Spec: [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
  §8.4, §16.5 #4, council finding C-018
- APS modules: [pack-pulumi](../modules/pack-pulumi.aps.md) (sets the pattern),
  [pack-llm-provider](../modules/pack-llm-provider.aps.md),
  [pack-drizzle](../modules/pack-drizzle.aps.md),
  [pack-nextjs](../modules/pack-nextjs.aps.md),
  [pack-hono](../modules/pack-hono.aps.md),
  [pack-tokio](../modules/pack-tokio.aps.md),
  [operational-supplement](../archive/modules/operational-supplement.aps.md)
  (per-pack feature flags + check registry + activation guards)
