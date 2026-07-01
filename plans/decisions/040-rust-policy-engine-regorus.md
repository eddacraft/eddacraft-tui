# ADR-040: Adopt regorus as the Anvil Policy Engine

## Status

Accepted

## Date

2026-05-10 (Proposed); 2026-05-13 (Accepted)

## Context

ADR-006 (Accepted, 2024) chose Go OPA as Anvil's policy engine alongside
Dependency-Cruiser for static analysis. That decision was made when Anvil was a
Node monorepo and "OPA as a Go library or sidecar" was the cheapest viable
integration — wrap OPA, ship Rego, get a battle-tested engine for free.

Three things have changed since:

1. **Anvil is now Rust-first.** ADR-026 made the Rust scanner authoritative,
   ADR-033 archives the TS scanner, ADR-014 reserves Rust for CPU-bound paths.
   Policy evaluation is CPU-bound and on the save-time hot path. Embedding a Go
   runtime inside the Rust CLI (FFI, sidecar daemon, or cgo) reintroduces the
   exact GC-runtime weight ADR-026 set out to eliminate.
2. **OPAE flagged this.** The `opa-enhancements` module
   (`plans/modules/opa-enhancements.aps.md`) carries an explicit `NOTE(post-rust)`
   that its "we wrap OPA, not replace it" Out-of-Scope clause is a pre-rust
   assumption to revisit when OPAE moves to Ready. OPAE is currently Draft and
   blocks RCLI2-005..-008, COMPLY, POLFED, and parts of CPACKS — i.e. the
   engine choice is on the critical path for the Policy Governance
   constellation.
3. **Customer signal: tiered authoring.** Customers want Out-of-the-Box
   policies, a YAML configuration tier, and Rego as the power-user escape
   hatch. All three tiers must compile to the same evaluation engine —
   maintaining three engines is not an option. Engine choice constrains the
   substrate the tier model is built on.

A pure-Rust Rego engine — `regorus` (Microsoft, MIT) — has matured to the
point where it is a credible substrate: Rego v1 compatible (OPA v1.2.0 test
suite), `no_std`-capable (1.9 MB minimal binary), ~10× faster than reference
OPA on real workloads (Microsoft ACI benchmarks), supports custom Rust
builtins for engine-internal data sources, and ships bindings to eight host
languages.

The decision needs to land before POLENG-001 (engine crate skeleton) so the
substrate question stops blocking dependent modules.

## Decision

### D-1 — Adopt `regorus` as the embedded policy engine

A new crate `crates/anvil-policy-engine` wraps `regorus` behind an
Anvil-shaped facade:

```rust
// Public facade — sketch only; types and bodies live in `crates/anvil-policy-engine`.
// `eval` takes `&mut self` because the underlying engine mutates state when
// the input document is set; the skeleton signature lives in the crate.
pub struct Engine { /* opaque */ }

impl Engine {
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> { todo!() }
    pub fn eval(&mut self, input: &PolicyInput, query: &str) -> Result<EvalResult, EngineError> { todo!() }
    pub fn register_builtin<B: Builtin>(&mut self, b: B) -> Result<(), EngineError> { todo!() }
    pub fn coverage(&self) -> Coverage { todo!() }
}
```

`regorus` is pinned at a committed minor version. The facade is the public
surface; downstream crates (anvil-policy, anvil-cli, anvil-tui, anvil-kernel)
depend on the facade, never on `regorus` directly. This preserves the option
to swap engines later without a fan-out refactor.

### D-2 — Engine surface contract

Three properties are non-negotiable for the facade:

1. **Input is a closed data document.** Policies receive an Anvil-defined
   `PolicyInput` — repo state, plan files, decision log, diff. The engine does
   not reach for files at eval time. (Schema is owned by POLENG-002.)
2. **Builtins are explicit, audited, deterministic.** No clock, no network,
   no filesystem access except via Anvil-controlled builtins (which themselves
   honour the determinism contract). Aligns with ADR-001/002/003.
3. **Coverage and trace are first-class.** Every eval returns the line-level
   coverage map and rule-firing trace. This is what the OPAE TUI debugger and
   "warnings over blocks" rule attribution build on. Not an opt-in.

### D-3 — Relationship to ADR-006

ADR-006 is **amended, not superseded**. The Dependency-Cruiser half is
unaffected. Only the OPA-engine half is updated:

> ADR-006 §"OPA for policy evaluation" — pre-rust pick. Superseded by ADR-040
> for post-rust embedding. Rego as the power-user authoring language stands;
> the runtime is regorus, not Go OPA.

The DECISION-LOG.md row for ADR-006 is updated to "Accepted (engine half
amended by ADR-040)" when this ADR moves to Accepted.

### D-4 — Migration scope

There is no live migration. The current `OpaExecutor` and friends live in the
archived TS tree (`archive/anvil-ts-*`) and stay archived. Policy work landing
in `crates/anvil-policy/` builds on regorus from POLENG-001 onwards.

CPACKS Rego content is portable as-is — Rego v1 is the source-of-truth syntax
on both engines. CPACKS's `NOTE(post-rust)` is resolved by this ADR.

### D-5 — Out of scope for this ADR

Explicit non-decisions, to keep this ADR reviewable:

- **Tier model (OOB / YAML / Rego).** A separate ADR will pin
  configurative-vs-additive YAML scope. POLENG-Y blocks on it.
- **OOB rule catalogue v1.** Owned by POLENG-O work items.
- **Bundle distribution and signing.** Lives in POLFED; the former OPAE bundle
  references are historical and unaffected by engine choice.
- **Watch-mode performance budget.** Validated against the substrate by
  POLENG-008 bench harness, not pre-committed here.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **regorus (chosen)** | Pure Rust, no GC runtime; `no_std`, 1.9 MB minimal; ~10× faster than Go OPA on real workloads; Rego v1 compatible (CPACKS content portable); custom Rust builtins fit Anvil's data model; MIT licence; bindings to 8 host languages future-proof for browser/wasm | Single-vendor (Microsoft), 306 stars; some OPA builtins absent (JWT, graph, CIDR, crypto); no formal SLA |
| Go OPA via cgo / FFI | Battle-tested; full builtin coverage; reference behaviour | Drags Go runtime into the Rust binary (binary size, startup cost, GC pause on hot path); cgo undermines `cargo`-native build story; conflicts with ADR-026 spirit |
| Go OPA as sidecar daemon | Clean process boundary; no FFI; OPA upgrades independent | Adds an IPC hop on the save-time hot path (ADR-031 latency rubric tightens; daemon already owns scope per ADR-036, second daemon is operational tax); install-time complexity; conflicts with planless-first (ADR-001) |
| Cedar (AWS, Rust) | Pure Rust; formally verified evaluator; simpler/safer language by design; Apache-2.0 | Different policy language — CPACKS Rego content not portable; smaller community in policy-as-code space; weaker fit for "model arbitrary repo state and query it" |
| Build our own Rego | Total control; tailored to Anvil semantics | Multi-engineer-year scope (parser, type system, builtins, partial eval, comprehensions, negation-as-failure); reinvents a solved problem; every CPACKS pack becomes a parity-test target |

The decision optimises for: zero-GC embedding, ADR-026 alignment, customer
Rego skill transferability, and reversibility (the facade isolates the
choice).

The accepted trade-offs are: dependency on a single-vendor Rust crate, and
the missing OPA builtins (mitigated below).

## Consequences

- **Positive:**
  - Anvil ships a single-binary policy stack consistent with ADR-026
  - 10× headroom on the save-time latency budget (ADR-031) means watch-mode
    and intercept-loop (ADR-015) gain breathing room
  - Custom Anvil builtins become first-class — repo state, plan documents,
    decision log queryable from Rego without bespoke pre-processing
  - Wasm target unlocks future browser playground and embedded dashboard
    evaluation (informs Web Dashboard module)
  - CPACKS Rego content carries forward; no rewrite

- **Negative:**
  - Customers writing custom policies still need Rego literacy; no reduction
    in learning curve at the power-user tier (the tier-model ADR addresses
    this for the 90% case)
  - Some OPA builtins absent (JWT, graph, CIDR, crypto); first-party shims
    needed if a CPACKS pack requires them

- **Risks:**
  - **Single-vendor dependency.** Microsoft could deprioritise regorus.
  - **Performance claim regression.** 10× was Microsoft's ACI benchmark, not
    Anvil's policy mix.
  - **Missing builtin discovered late.** A CPACKS pack hits a gap after
    distribution.
  - **Sandboxing claims weak in regorus README.** Confidential-computing
    framing is not the same as a hard capability boundary.

- **Mitigations:**
  - Pin regorus version + own the facade. Engine swap remains a contained
    refactor (downstream crates depend on `anvil-policy-engine`, never on
    `regorus` directly).
  - POLENG-008 bench harness runs against real Anvil checks before POLENG
    moves to Ready. If parity (not 10×, just parity) doesn't hold, this ADR
    is revisited.
  - First-party builtin shim pattern (`Engine::register_builtin`) gives a
    drop-in escape hatch for missing OPA builtins, implemented in Rust where
    we control the determinism contract.
  - Determinism is enforced at the facade (no clock, no net, no fs except
    via builtins) regardless of upstream sandboxing posture. Belt and braces.

## References

- **Philosophy:** [ADR-001](001-planless-first.md),
  [ADR-002](002-warnings-over-blocks.md), [ADR-003](003-new-edges-only.md)
- **Amended:** [ADR-006](006-hybrid-dc-opa.md) (engine half only;
  Dependency-Cruiser stands)
- **Stack alignment:** [ADR-014](014-language-allocation-tree-ts-vs-rust.md),
  [ADR-026](026-rust-scanner-authoritative.md),
  [ADR-033](033-park-ide-mcp-retire-ts-scanner.md)
- **Latency contract:** [ADR-031](031-validation-latency-rubric.md)
- **APS modules:** POLENG (proposed), OPAE (re-platforms onto POLENG),
  CPACKS, POLVAL, POLFED
- **External:**
  - regorus — <https://github.com/microsoft/regorus>
  - Open Policy Agent — <https://www.openpolicyagent.org/>
  - Cedar — <https://www.cedarpolicy.com/>
