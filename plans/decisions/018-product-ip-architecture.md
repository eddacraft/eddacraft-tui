# ADR-018: Product / IP architecture — free-tier-not-open-source with three-piece OSS surface

## Status

Accepted

> **Freshness note (2026-05-18):** ADR-047 proposes changing
> `eddacraft-tui`'s source-of-truth topology: the crate remains part of the
> Apache-2.0 OSS surface, but its canonical source moves back into Anvil and the
> public repository becomes a read-only mirror. This amends the contribution
> assumption for that repo without changing the three-piece OSS surface.
>
> **Amendment (2026-07-14, SKPKG):** proprietary distribution may include
> customer-readable operational assets such as agent-skill Markdown embedded
> in the binary and materialised by the CLI. "Binary-only" means product
> source is not published; it does not require every installed support asset
> to be opaque. Skills remain part of the closed product during beta, with an
> eventual OSS transition explicitly left open.

## Date

2026-04-07

## Context

While preparing DIST-008 (publish `anvil-cli` to crates.io), an
implicit assumption surfaced: that publishing the Anvil binary required
either open-sourcing the CLI under a permissive license (Apache-2.0,
open-core style) or picking a source-available license (BUSL, FSL).

That framing turned out to be wrong. eddacraft's actual product / IP
architecture is **free at base tier, source proprietary** — the
Postman / Cursor / Linear / Raycast / Warp model — with a deliberate,
narrow OSS surface limited to three foundational repos. The closed
monorepo (`anvil-001`) stays closed, and `cargo install` is structurally
incompatible with the model.

This ADR captures the architecture so it stops being implicit, and so
future distribution / licensing / contribution decisions have a single
authoritative reference.

## Decision

eddacraft operates a **closed-source product with a deliberate
three-piece open-source surface**.

### The closed product (private monorepo: `eddacraft/anvil-001`)

Everything that makes Anvil "Anvil" lives in the closed monorepo:

- The Anvil CLI (`crates/anvil-cli`, published as binary `anvil`)
- The Rust kernel, policy engine, architecture engine, checks
  (`crates/anvil-kernel`, `crates/anvil-policy`,
  `crates/anvil-architecture`, `crates/anvil-checks`,
  `crates/anvil-kernel-types`, `crates/anvil-tui`)
- Edda (canonical memory) and Ember (interpretation pipeline)
  in `packages/edda-stack`
- The web dashboard (`apps/website`)
- The Anvil API (`apps/anvil-api`)
- The MCP server (`packages/mcp-server`)
- OPA enhancements, agent governance, compliance reporting,
  policy lifecycle, policy federation
- The compliance policy packs (CPACKS — see "Future" below)
- The auth / activation / licence stack (BAUTH)
- The infrastructure-as-code (IAC)

License: `LicenseRef-Proprietary`. Source never leaves the private
repo. Distribution is binary-only via the public release repo
(`eddacraft/anvil`).

### The OSS surface — three repos, all `Apache-2.0`

| Repo | Layer | Purpose |
|---|---|---|
| `eddacraft/eddacraft-tui` | Presentation primitive | Reusable Ratatui widget library implementing the eddacraft Terminal Standard design system. Consumed by the closed CLI and by other eddacraft tools. |
| `eddacraft/anvil-plan-spec` | Format / protocol | The APS planning format spec, parser, and validator. Anyone can adopt APS for their own AI-assisted development workflow without using Anvil. |
| `eddacraft/kindling` | Memory primitive | Small, composable memory primitives for agentic workflows — observation, capture, basic stores. Foundation that the closed Edda and Ember components consume. |

These three are deliberately **protocol / primitive / infrastructure
layers**, not the product. They are open because:

- **They benefit from network effects.** An open APS spec is more
  valuable to eddacraft as an *adopted standard* than as a private
  format. Same logic as OpenTelemetry vs proprietary APMs.
- **They are a trust signal.** Publishing the format spec, the memory
  primitives, and the design system tells enterprise buyers "this is
  built on inspectable foundations."
- **They are a contribution surface.** Outside contributions to widgets,
  memory backends, or plan validators are *helpful* without giving away
  product code.
- **None of them are the product.** Consuming all three primitives does
  not get a competitor anywhere close to having Anvil.

### Why this is not "open core"

Open core typically splits a single product into "free OSS edition" and
"paid enterprise edition" of the *same* codebase. We are not doing that.

What we have is:

- A **closed product** with a free tier gated by license server / activation
- A separate **open primitives layer** that the closed product consumes
  alongside any other tool that wants to consume it

The open repos are not "Anvil community edition." They are foundations
that exist independently of Anvil and would have value even if Anvil
did not exist. This is closer to the **PostgreSQL pattern** (open
protocol, closed managed services), the **OpenTelemetry pattern**
(open instrumentation, closed observability backends), and the
**Language Server Protocol pattern** (open spec, closed editors) than
to traditional open core.

### Distribution implications

Because the product is closed-source:

- **`cargo install` is not viable.** Publishing to crates.io requires
  publishing source. DIST-008 is **deferred / out of scope** —
  see `plans/modules/distribution-pipeline.aps.md`.
- **Windows install path is WinGet (+ optional scoop)**, not crates.io.
  Both are manifest-based — they point at the GitHub Release binary
  and require zero source disclosure.
- **macOS install path is Homebrew tap** (DIST-009) — already in scope.
- **Linux install path is `install.sh`** via `install.eddacraft.ai`
  (DIST-003 / DIST-005 / DIST-006) — already in scope.
- **Universal install path is GitHub Releases via cargo-dist** (DIST-007).
- The crate-rename to `eddacraft-anvil-*` was analysed, approved, and applied
  to all publishable crates (ADR-017). Crates.io publication itself is deferred
  alongside DIST-008. Current status and any future crates.io-related work
  should be tracked via `plans/modules/distribution-pipeline.aps.md`. No
  crates.io publish is currently planned.

### Customer-readable operational assets

The closed product may embed and install bounded, human-readable operational
assets needed to connect customers' tools to Anvil, including Agent Skills,
configuration templates, shell completions, and help text. These assets:

- ship only through a signed Anvil release during beta;
- may be inspected after installation so operators can audit instructions
  given to their agents;
- do not expose the Anvil engines, catalogue, emission tooling, or private
  monorepo source; and
- do not create a fourth OSS repository or an open-source commitment.

If the skills catalogue becomes OSS later, that is a distribution/source
transition for the assets, not an implicit opening of the Anvil product.

### Activation / licensing / telemetry

Free-tier-not-open-source requires runtime license validation. The
existing BAUTH module (complete: 20/20) provides device code + email
OTP activation with JWT sessions, which is the foundation. Tier
gating, telemetry, and feature flags consume this same auth context.
This is already architecturally supported and does not need new work
to satisfy the IP model.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|---|---|---|
| **Chosen: free-tier closed source + 3 OSS primitives** | Maximum control of product code; clean separation of "what is the moat" vs "what is the foundation"; binary distribution is well-tooled (cargo-dist, Homebrew, WinGet); BAUTH already supports activation; no per-feature license drama | No `cargo install`; can't accept community PRs to product code; some developer audiences distrust closed source by default |
| Open core (Apache CLI, closed enterprise repos) | Enables `cargo install`; community contributions to CLI; easier developer adoption | Would require splitting the monorepo into "open" and "closed" halves, which we don't have; gives away the CLI source which contains the policy engine, architecture engine, checks; the "edition split" is messy and ongoing maintenance |
| Source-available (FSL / BUSL on the whole monorepo) | Source visible (trust signal); blocks competing managed services; no monorepo split | Source is still public, which contradicts the "closed product" intent; doesn't enable `cargo install` cleanly anyway because internal dep graph is too tangled; legal text adds procurement friction |
| Fully proprietary, no OSS surface at all | Maximum control; simplest to reason about | No network-effect upside from format / protocol / primitive adoption; no community trust signal; loses the strategic value of an open APS / Kindling / TUI design system |

### Key trade-off accepted

We accept the loss of the `cargo install` install path and the inability
to take community PRs against product code in exchange for:

- **Total control of the product source** (architecture, kernel, policy
  engine, dashboard, compliance packs, Edda, Ember, OPA enhancements)
- **A clean public/private boundary** that does not require ongoing
  monorepo splitting
- **A focused OSS surface** where contributions are welcomed and
  meaningful without endangering the product
- **A coherent licensing story** that enterprise procurement can
  understand in 30 seconds

The Windows-user gap that DIST-008 was meant to fill is filled by
WinGet, which is the standard Windows install path for closed-source
developer tools and requires no source disclosure.

## Consequences

- **Positive:**
  - DIST-008 stops being a blocker; it becomes a non-goal
  - No license-text-decision required for the monorepo
  - The crate-rename analysis is preserved as defensive namespace
    insurance — execution is deferred alongside DIST-008 but the
    analysis protects future optionality
  - The three OSS repos can be developed, versioned, and released on
    their own cadence without coupling to the product release cycle
  - WinGet + scoop covers Windows; Homebrew covers macOS; install.sh
    covers Linux — a complete, closed-source-friendly install matrix
  - Activation + tier gating already supported by BAUTH
- **Negative:**
  - No `cargo install anvil-cli` — Rust users have to use the
    install script, Homebrew, or WinGet like everyone else
  - Cannot accept outside PRs to product code (only to the three
    OSS repos)
  - Some developer audiences default to distrusting closed source —
    the OSS primitive surface is the trust counterbalance
- **Risks:**
  - If a competitor publishes a parallel "Anvil-compatible" tool under
    the open APS spec, they could fragment adoption. **Mitigation:**
    we control the spec, the reference implementation, and the
    integrations — fragmentation is unlikely if we move fast on the
    closed product
  - If the OSS primitives stagnate, the trust-signal value erodes.
    **Mitigation:** treat the three OSS repos as first-class shipping
    artifacts with their own release cadence
  - If license-server / activation has downtime, free-tier users may
    be blocked from running the binary. **Mitigation:** BAUTH already
    designed with offline-grace tokens — verify the spec covers this

## Future

### Possible fourth OSS surface — policy definitions (CPACKS)

The compliance policy packs (`crates/anvil-policy/library/*` and
`core/src/gate/__fixtures__/library/*`) — OWASP Top 10, SOC2, ISO 27001,
GDPR, NIST AI RMF, EU AI Act, etc. — are a candidate for future
open-sourcing as a community contribution surface.

**Why this might make sense later:**

- Policy definitions are inherently community-curated knowledge
  (compliance frameworks change, interpretations vary by industry)
- Open policy contributions scale better than closed authoring
- It mirrors how `eslint-plugin-*` and `clippy` lint catalogues grew
- The *engine* that runs the policies stays closed — only the policy
  text is open
- Anvil's value remains the engine, the dashboard, the evidence
  workspace, the federation, the audit trail — not the rule text

**Why it is not happening now:**

- The current pack catalogue is small enough that eddacraft can curate
  it directly
- Opening before the engine is stable could create policy contributions
  that the engine cannot evaluate correctly
- The model can be revisited once CPACKS module is shipped (currently
  Draft, 0/28)

If/when this happens, it becomes a fourth ADR and a fourth OSS repo
(`eddacraft/anvil-policy-packs` or similar).

### Possible additional small OSS pieces

Other candidates that could be opened in the future without affecting
the core product:

- The Anvil GitHub Action (`.github/actions/anvil-check/`) — already
  effectively public via `eddacraft/anvil`
- The .anvil file format spec (separate from the implementation)
- Tree-sitter language adapters for languages we add (lang-* modules)

These are not commitments — just a record that the door is open.

## References

- Crate namespace analysis: `eddacraft-anvil-*` prefix was analysed and
  approved; execution deferred alongside DIST-008
- APS modules: DIST (deferral of DIST-008), CPACKS (future open candidate),
  BAUTH (activation foundation, complete)
- OSS repos:
  - <https://github.com/eddacraft/eddacraft-tui>
  - <https://github.com/eddacraft/anvil-plan-spec>
  - <https://github.com/eddacraft/kindling>
- Brainstorm (superseded by this ADR):
  `plans/brainstorms/2026-04-07-anvil-licensing-decision.md`
- External references:
  - Postman / Cursor / Linear / Raycast / Warp / Arc — closed-source
    binary distribution patterns
  - PostgreSQL, OpenTelemetry, LSP — open-protocol-closed-product
    patterns
