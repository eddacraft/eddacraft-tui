# Anvil Licensing — One-Pager for Discussion

> **⚠️ SUPERSEDED 2026-04-07** — This brainstorm was written under the
> assumption that the Anvil CLI was a candidate for open-sourcing as
> part of an open-core strategy. Subsequent clarification established
> that the actual model is **free at base tier, source proprietary**,
> with a deliberate three-piece OSS surface (`eddacraft-tui`,
> `anvil-plan-spec`, `kindling`) that does *not* include the product
> code. The licensing question is therefore not open — see
> [ADR-018: Product / IP Architecture](../decisions/018-product-ip-architecture.md)
> for the resolved framing and
> [`docs/architecture/oss-surface.md`](../../docs/architecture/oss-surface.md)
> for the external-facing description. This document is kept as
> historical context for the conversation that produced ADR-018.

**Date:** 2026-04-07
**Status:** Superseded by ADR-018
**Decision needed by:** ~~before first `cargo publish` (DIST-008)~~ — DIST-008 deferred per ADR-018
**Audience:** Joshua / EddaCraft IP holder

---

## TL;DR

Publishing the Anvil CLI to crates.io forces us to pick a real license. The
workspace currently says `LicenseRef-Proprietary`, which crates.io technically
accepts but every corporate license scanner flags as "do not use" — killing
adoption inside our target market. **Recommendation: ship under `Apache-2.0`.**
The CLI source is not where Anvil's moat lives, and a permissive license
removes every adoption barrier in the demographic we care about.

---

## Why this is on the table now

DIST-008 (publish `eddacraft-anvil` to crates.io for Windows / `cargo install`
users) is one of the last unblocked items between us and a real release. We
have already settled the naming question (ADR-017), branched the
`eddacraft-tui` repo for publish prep, and renamed all 7 workspace crates. The
license field is the next remaining blocker — and unlike the others (repo URL,
path-dep versions), it is a **business decision**, not an engineering one.

Publishing to crates.io means the source tarball is downloadable forever by
anyone with `cargo`. There is no such thing as a "private but on crates.io"
crate. Any answer must therefore start from "the source is public — what
restrictions, if any, do we want to place on what people can do with it?"

---

## The three viable options

| Option | License | Source visibility | Adoption friction | What it blocks | What it costs us |
|---|---|---|---|---|---|
| **A. Open source, permissive** | `Apache-2.0` | Public | **Zero** — passes every corporate procurement scan | Nothing | The theoretical right to keep CLI source closed |
| **B. Source-available, modern** | `FSL-1.1-Apache-2.0` (Sentry's Functional Source License) | Public | **Low** — well-known to AI/dev-tool buyers, auto-converts to Apache-2.0 after 2 years | Competitors reselling Anvil as a managed service | Some procurement teams flag any "non-OSI" license; smaller pool of contributors; minor friction |
| **C. Don't publish to crates.io** | Workspace stays `LicenseRef-Proprietary` | **Private** — no source on registry | n/a — `cargo install` simply doesn't work | n/a | Drop DIST-008 entirely; Windows users install via cargo-dist `.msi`, WinGet, or scoop instead |

Custom EULA via `license-file` is **not** on this list. It's strictly worse
than A, B, or C on every axis except theoretical control, and the theoretical
control buys us nothing — see "What is actually our moat" below.

---

## What is actually our moat

The Rust source of the CLI binary itself is **not** where Anvil's defensible
value lives. The moat is:

- **The policy library** — compliance packs (CPACKS), OWASP, SOC2, ISO-27001,
  GDPR, NIST AI RMF, EU AI Act — the library, the manifest format, the
  curation
- **The hosted dashboard** — DASHCORE / DASHARCH / DASHOPS / DASHAI views,
  the web UI, the auth layer, the multi-tenant data plane
- **Edda + Ember** — the canonical memory + interpretation pipeline, the
  evolution service, the hash-chain audit trail
- **OPA enhancements + agent governance** — agent trust scoring, capability
  manifests, policy lifecycle, policy federation
- **Compliance evidence workspace + trust center automation**
- **The brand, the docs, the support, the integrations, the SaaS**

If a competitor cloned `eddacraft-anvil` from crates.io tomorrow, they would
get a CLI binary that knows how to enforce policies — and they would still
need to build *all* of the above to have a product. The CLI is the on-ramp,
not the destination.

This is the same calculus that led `ripgrep`, `bat`, `helix`, `zellij`,
`uv`, `ruff`, and every successful Rust dev tool to ship under MIT or Apache.
The CLI being open accelerates adoption; the value capture happens elsewhere.

---

## Recommendation: Apache-2.0

**Pick A.** Ship the entire workspace under `Apache-2.0` (or `MIT OR
Apache-2.0` if we want to maximise compatibility — Apache alone is fine).

**Why Apache over MIT specifically:** Apache-2.0 includes an explicit patent
grant, which protects us against patent trolls and signals to enterprise
procurement that we have thought about IP. MIT is shorter but lacks this.
Most modern Rust dev tools use Apache or dual MIT-OR-Apache for exactly this
reason.

**Why not FSL:** FSL is a great license, but the only thing it blocks —
"someone reselling Anvil as a managed service" — is a problem we don't have
yet, won't have until we are large enough to be worth copying, and can switch
to later if it ever becomes real (BUSL/FSL relicensing of an existing
permissive crate is straightforward; the inverse is not). Picking FSL today
trades real adoption friction for hypothetical future protection.

**Why not C (don't publish):** dropping DIST-008 is genuinely viable and
worth keeping in our back pocket — but it leaves a gap in the install story
and signals "we're afraid of our own source code" to the developer audience
we are trying to win. The cost of publishing under Apache is zero; the cost
of *not* publishing is a smaller funnel.

---

## What is being decided

> **Should the Anvil workspace be relicensed from `LicenseRef-Proprietary`
> to `Apache-2.0` so we can publish to crates.io as part of DIST-008?**

Sub-questions that fall out of "yes":

- Apache-2.0 alone, or `MIT OR Apache-2.0` dual? (recommend: Apache alone for
  simplicity, switch to dual only if a contributor specifically asks)
- Does the relicense apply to the whole workspace, or only the crates we
  actually publish? (recommend: whole workspace, for consistency and to avoid
  per-crate license drift)
- Do we want a `NOTICE` file alongside `LICENSE`? (Apache-2.0 supports but
  doesn't require — recommend: yes, add EddaCraft attribution)
- Same license for `eddacraft-tui` (separate repo)? It is currently already
  `Apache-2.0` ✅ — no change needed.

Sub-questions that fall out of "no, pick FSL instead":

- Confirm the 2-year Apache conversion clause is acceptable
- Confirm we are willing to maintain the FSL text and answer "what is FSL"
  questions from procurement

Sub-questions that fall out of "no, drop DIST-008":

- Update `plans/modules/distribution-pipeline.aps.md` to remove DIST-008
- Confirm Windows install story is cargo-dist `.msi` + Homebrew + (eventually)
  WinGet, with no `cargo install` path
- Revert the crate renames? Or keep them anyway for namespace protection?
  (recommend: keep — the renames cost nothing and protect us if we change
  our minds later)

---

## What happens after the decision

**If A (Apache-2.0):**

1. Add `LICENSE` file at workspace root (canonical Apache-2.0 text)
2. Update workspace `Cargo.toml`: `license = "Apache-2.0"`
3. Update workspace `repository = "https://github.com/EddaCraft/anvil"`
4. Add `version.workspace = true` to all internal path-deps
5. Publish `eddacraft-tui` from its branch, then publish the 7 workspace
   crates in dependency order
6. Verify `cargo install eddacraft-anvil` works on a clean machine
7. Mark DIST-008 complete

**If B (FSL):**

1–7 as above, with `license = "FSL-1.1-Apache-2.0"` and the FSL `LICENSE.md`
text from <https://fsl.software>. Add a short FAQ in the README explaining
what FSL is and the auto-conversion clause.

**If C (drop DIST-008):**

1. Update `plans/modules/distribution-pipeline.aps.md` to remove DIST-008,
   noting the reason
2. Verify cargo-dist `.msi` Windows installer is being generated and tested
3. Add WinGet manifest as a follow-up item (replacing crates.io as the
   Windows discovery channel)
4. Leave all the other DIST work intact — repo, install.sh, GitHub Pages,
   DNS, release workflow, Homebrew tap

---

## Open questions for the discussion

1. **Has anyone said this needs to be proprietary, or is the current
   `LicenseRef-Proprietary` just a default?** If it is just a default, the
   answer is almost certainly Apache and this conversation is short.
2. **Are there any third-party code contributions in the workspace already
   that we would need to relicense?** A quick `git log --format='%aN' | sort
   -u` will tell us. If all commits are EddaCraft-authored, relicense is
   trivial.
3. **What does the dashboard / web app license? What does the hosted SaaS
   ToS say about the CLI?** These should be coherent with whatever we pick
   here.
4. **Is there appetite for an "Anvil community edition vs Anvil enterprise"
   split?** That is the open-core path and is compatible with Option A —
   the enterprise features (compliance packs, dashboard, federation) live
   in private repos with their own license.

---

*This document is for discussion only. Once a decision is made, capture it
in `plans/decisions/018-anvil-licensing.md` (next ADR number) and update
DIST-008 in the plan accordingly.*
