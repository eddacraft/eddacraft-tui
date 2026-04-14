# ADR-017: crates.io naming — `eddacraft-anvil-*` namespace prefix

## Status

Accepted

## Date

2026-04-07

## Context

Distribution Pipeline (DIST) needs to publish the Anvil binary to crates.io
so users on platforms without our install script (notably Windows users
without Scoop / Chocolatey / WinGet) can run `cargo install`.

When DIST-008 was first scoped, the assumption was that we would publish
under the natural names `anvil-cli`, `anvil-kernel`, `anvil-tui`, etc.

A reality check on 2026-04-07 found:

- `anvil-cli` on crates.io is **already taken** by an unrelated template
  generator (last published 2025-10-16, categories: `template`, `generator`).
  We cannot use it.
- The bare `anvil-*` library names (`anvil-kernel`, `anvil-tui`,
  `anvil-policy`, `anvil-architecture`, `anvil-checks`,
  `anvil-kernel-types`) are all currently available — but they are generic
  enough that future collisions are likely, and "anvil" is a common-enough
  word that trademark friction is plausible if a different "Anvil" tool
  emerges in the Rust ecosystem.
- The brand-prefixed names (`eddacraft-anvil`, `eddacraft-anvil-kernel`,
  `eddacraft-anvil-tui`, `eddacraft-anvil-policy`,
  `eddacraft-anvil-architecture`, `eddacraft-anvil-checks`,
  `eddacraft-anvil-kernel-types`) are all available.

The decision needs to be made **now**, before any release on
`eddacraft/anvil` is cut, because the names will be baked into install
scripts, GitHub Action artifact URLs, documentation, and (eventually)
the crates.io registry itself. Renaming after publication is far more
expensive than renaming today.

## Decision

Publish all Anvil crates to crates.io under the `eddacraft-anvil-*`
namespace prefix:

| Internal directory | Local dep key | crates.io package name |
| ------------------ | ------------- | ---------------------- |
| `crates/anvil-cli` | `eddacraft-anvil` (binary) | `eddacraft-anvil` |
| `crates/anvil-kernel` | `anvil-kernel` | `eddacraft-anvil-kernel` |
| `crates/anvil-kernel-types` | `anvil-kernel-types` | `eddacraft-anvil-kernel-types` |
| `crates/anvil-tui` | `anvil-tui` | `eddacraft-anvil-tui` |
| `crates/anvil-policy` | `anvil-policy` | `eddacraft-anvil-policy` |
| `crates/anvil-architecture` | `anvil-architecture` | `eddacraft-anvil-architecture` |
| `crates/anvil-checks` | `anvil-checks` | `eddacraft-anvil-checks` |

The user-facing **binary remains `anvil`** — only the package name on
crates.io changes. Users still type `anvil gate`, `anvil watch`, etc.

The `eddacraft-tui` crate (in a separate repo) is already brand-prefixed
and available on crates.io — no rename needed.

`anvil-bench` and `anvil-spike` are internal-only and marked
`publish = false`; they keep their bare names because they will never
appear on crates.io.

### Path-dep aliasing strategy

To avoid touching every `use ...` statement in the source tree, internal
path-deps use Cargo's `package = "..."` aliasing field:

```toml
anvil-kernel = { path = "../anvil-kernel", package = "eddacraft-anvil-kernel" }
```

The local key (`anvil-kernel`) is what `use anvil_kernel::...` resolves
to, so source code is unchanged. Only `Cargo.toml` files needed editing.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Chosen: `eddacraft-anvil-*` everywhere** | Brand-protected, future-proof, mirrors `aws-sdk-*` / `tokio-*` / `bevy_*` convention, consistent with the binary name we already settled on | Longer to type when adding to a `Cargo.toml`; `cargo install eddacraft-anvil` is less catchy |
| Bare `anvil-*` names | Shorter, more idiomatic for `cargo install` | "anvil" is generic — `anvil-cli` was already taken once; future collisions likely; no brand protection; inconsistent with the binary name we already chose |
| Defensive squat — claim **both** namespaces | Maximum future flexibility | 6 extra placeholder crates to maintain forever; crates.io discourages pure squatting; cost for benefit isn't justified |
| Drop crates.io entirely (Strategy "ship via cargo-dist + Homebrew + WinGet only") | Zero ongoing publish toil | Loses the `cargo install` discovery channel; revisits the question every time a Rust user asks |

### Key trade-off accepted

We accept the longer crate names and the longer `cargo install
eddacraft-anvil` command in exchange for:

- **No future rename pressure.** The `eddacraft-*` namespace is ours.
- **Consistency.** Half-and-half (`eddacraft-anvil` binary + `anvil-kernel`
  library) would be the worst of both worlds: long name where it doesn't
  matter, generic name where collisions are most likely.
- **Branding.** Anyone browsing crates.io sees an `eddacraft-*` family,
  not a scattered `anvil-*` collection that visually competes with the
  unrelated `anvil-cli` template generator.

## Consequences

- **Positive:**
  - Naming is locked in and bulletproof against future collisions
  - Aligns the entire library surface with the brand
  - The `package = "..."` aliasing trick means **zero source-code churn**
    (no `use` statements changed, no breakage in dependent crates)
  - `cargo check --workspace` passes after the rename
  - DIST-008 can move forward without further naming decisions
- **Negative:**
  - `cargo install eddacraft-anvil` is wordier than the original plan
  - Library consumers writing `Cargo.toml` files type more characters
- **Risks:**
  - The path-dep `package = "..."` aliasing is a less-common Cargo
    feature; future contributors may be confused. **Mitigation:** the
    aliasing is documented in this ADR and the dep lines are
    self-explanatory.
  - The workspace `repository` field still points to `eddacraft/anvil-001`
    (private). Before the first crates.io publish, this should be updated
    to `https://github.com/eddacraft/anvil` (the public mirror), or each
    published crate's listing will have a broken repository link.
    **Mitigation:** tracked as a follow-up under DIST-008.
  - The workspace license is `LicenseRef-Proprietary`, which crates.io
    will reject — it requires an SPDX identifier (e.g. `Apache-2.0`,
    `MIT`, dual `MIT OR Apache-2.0`). This is a separate decision from
    naming and is **not** addressed by this ADR. Tracked as a follow-up
    under DIST-008.

## References

- APS modules: DIST-008
- Plan: `plans/modules/distribution-pipeline.aps.md`
- Related ADRs: ADR-011 (Rust core engine), ADR-012 (Rust CLI replacement)
- crates.io aliasing docs: <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml>
