# ADR-049: Cross-language `^build` contract — defer to nxrust D-009

## Status

Accepted

## Date

2026-05-20

## Context

Anvil's `pnpm test` was 40m03s end-to-end before PR
[#1729](https://github.com/eddacraft/anvil-001/pull/1729). Direct measurement
showed real work was ~52s; the remaining ~39 minutes was concurrent
`cargo build -p X` invocations serialising on the workspace `target/` lock.

The cross-stack `^build` edge that triggered the cascade emerges from the
combination of three independent defaults:

1. `nx.json` declares `test.dependsOn: ["^build"]` as a workspace target
   default.
2. `@nx/js/typescript` plugin auto-creates project-graph edges from each JS
   `package.json`'s `dependencies`.
3. Any JS package whose deps include a napi-rs crate exposed as a JS workspace
   package (e.g. `@eddacraft/anvil-checks-native`) gains a graph edge to that
   crate's nx project.

The nxrust APS plan itself does **not** declare cross-language edges — Module
03 only wires `dependsOn` from intra-Rust cargo edges; Module 13 puts
cross-language edge construction explicitly out of scope. The contract for
cross-language `dependsOn` was undecided in writing until nxrust D-009 landed
(see References).

Since nxrust is a public plugin with downstream adopters, the contract belongs
at the plugin level, not as an Anvil-local convention.

## Decision

Anvil defers to **nxrust D-009** (the binding upstream decision) for all
cross-language `dependsOn` shape questions. Anvil does not maintain a parallel
convention.

In Anvil specifically:

1. The seam is enforced at the **script layer** in root `package.json`:
   `test` runs `pnpm test:js && pnpm test:rust`, where `test:js` filters to
   JS-tagged projects (`tag:npm:public,tag:npm:private`) and `test:rust` runs
   `cargo test --workspace`. PR #1729 made this change.
2. If Anvil later adopts an nxrust generator that creates a cross-language
   edge (e.g. `add-wasm-reference`, `add-napi`), the generator is expected to
   enforce the D-009 contract at the generator boundary; the script-layer
   split remains as defence-in-depth.
3. Anvil does not maintain a recipe doc for the JS↔Rust test seam. The
   canonical reference is
   [nxrust `docs/recipes/javascript-rust-test-seams.md`](https://github.com/eddacraft/nxrust/blob/main/docs/recipes/javascript-rust-test-seams.md).
4. Pre-emptive narrowing of `test.dependsOn` on individual JS projects that
   don't import Rust artefacts at TS-build time is **out of scope**. Per
   memory note about JS/TS retirement velocity, manual per-project
   annotation is paying down decay already decided. Wait for `nxrust doctor`
   to surface the cases if and when it materialises.

## Rationale

Three properties of nxrust D-009 made deferral the right shape:

- **Generator-boundary enforcement protects downstream adopters too.** Anvil's
  script-layer fix only protects Anvil. Putting the contract in nxrust
  protects every consumer of the plugin from the same 40-min footgun.
- **D-009's import-time vs runtime gate is more nuanced than a blanket
  default-no-`^build`.** WASM modules bundled into a webpack/Vite build
  legitimately need `^build`; NAPI `.node` files loaded at require-time do
  not. The plugin-level decision captures that distinction; an Anvil-side
  convention couldn't.
- **One authoritative source avoids drift.** A parallel Anvil ADR or recipe
  would fork from nxrust on every clarification.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Defer to nxrust D-009 (chosen) | Single source of truth; protects downstream; respects upstream framing | Anvil convention is implicit until the relevant generator is adopted |
| Maintain a parallel Anvil-side recipe + convention | Locally complete | Drifts from nxrust on every update; duplicates effort; doesn't help downstream adopters |
| Inline the contract into `nx.json` target defaults (override `test.dependsOn` per-project) | Enforces at the graph layer in Anvil | Touches every JS package; pays down decay already decided per JS/TS retirement velocity; doesn't help downstream adopters |
| Add a `nxrust doctor` check now and require it in Anvil CI | Mechanically enforces the contract | Premature — no consumers of `add-wasm-reference` exist yet; would block on tooling not built |

## Consequences

- **Positive:** The decision is recorded once, in the right repo, with the
  right blast radius. Anvil's `package.json` script split (PR #1729) is the
  current enforcement; future nxrust generators inherit the constraint
  automatically.
- **Positive:** Anvil's `plans/decisions/` stays as the index for what's
  decided here, not as a parallel copy of upstream framing.
- **Negative:** Anvil reviewers need to follow the link to nxrust D-009 to
  see the full contract.
- **Risks:** If nxrust D-009 is revised or softened, Anvil inherits the
  change implicitly. Mitigation: anchor this ADR to the binding decision ID
  (D-009) and the public PR URL; nxrust's change history is the audit
  surface.

## References

- Related ADRs: ADR-014 (language allocation TS vs Rust)
- Anvil PR [#1729](https://github.com/eddacraft/anvil-001/pull/1729) —
  empirical anchor: `pnpm test` 40m03s → 31–52s
- nxrust PR [#11](https://github.com/eddacraft/nxrust/pull/11) — D-009 +
  recipe doc + module 10 alignment
- nxrust `plans/index.aps.md` D-009 — binding cross-language `dependsOn`
  contract
- nxrust `docs/recipes/javascript-rust-test-seams.md` — canonical recipe
  for the seam
