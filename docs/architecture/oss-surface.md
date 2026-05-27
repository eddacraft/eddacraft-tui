# eddacraft OSS Surface

| Type  | Authority | Owner  | Status | Freshness                                        |
| ----- | --------- | ------ | ------ | ------------------------------------------------ |
| Guide | Derived   | DOCGOV | Live   | Metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream         | Downstream                                    |
| ---------------- | --------------------------------------------- |
| ADR-018, ADR-047 | OSS mirror policy, public repository guidance |

eddacraft operates a closed-source product (the Anvil platform) with a
deliberate, narrow open-source surface limited to three foundational
repositories. This document describes those repositories, why they are open, and
how they relate to the closed product.

For the underlying decision, see
[ADR-018: Product / IP Architecture](../../plans/decisions/018-product-ip-architecture.md).
ADR-047 proposes moving `eddacraft-tui`'s canonical source back into Anvil while
keeping the public repo as a read-only mirror.

## TL;DR

| Repo                                                                        | Layer                  | License      | Status                  |
| --------------------------------------------------------------------------- | ---------------------- | ------------ | ----------------------- |
| [`eddacraft/eddacraft-tui`](https://github.com/eddacraft/eddacraft-tui)     | Presentation primitive | `Apache-2.0` | Public; mirror proposed |
| [`eddacraft/anvil-plan-spec`](https://github.com/eddacraft/anvil-plan-spec) | Format / protocol      | `Apache-2.0` | Public, in use          |
| [`eddacraft/kindling`](https://github.com/eddacraft/kindling)               | Memory primitive       | `Apache-2.0` | Public, in use          |

The Anvil product itself — CLI, kernel, policy engine, dashboard, compliance
packs, Edda, Ember, agent governance, OPA enhancements, auth, infrastructure,
the works — lives in the closed monorepo and is shipped as binary releases via
[`eddacraft/anvil`](https://github.com/eddacraft/anvil).

## The three open-source repos

### `eddacraft-tui` — Presentation primitive

A reusable Ratatui widget library implementing the **eddacraft Terminal
Standard** design system: colour palette, theme trait, keybinding conventions,
and a catalogue of TUI widgets (tables, badges, charts, panels, status bars,
editors).

**Why open:** Anyone building eddacraft-styled or eddacraft-compatible terminal
applications benefits from sharing the same visual identity and component
library. The widgets themselves have no Anvil-specific business logic — they are
pure presentation primitives.

**Source topology:** Today the public repo is the standalone source. ADR-047
proposes making Anvil's `crates/eddacraft-tui/` the canonical source and the
public repo a read-only mirror while preserving crates.io distribution.

**Consumed by:**

- The closed Anvil CLI (currently via crates.io; ADR-047 proposes an
  in-workspace path crate)
- Other eddacraft tools, current and future
- Any third-party Rust TUI application that wants the eddacraft look

**Contribution surface:** new widgets, theme refinements, accessibility
improvements, docs, examples. If ADR-047 is accepted, source changes land in
Anvil first and are mirrored publicly rather than being merged directly into the
public repo.

### `anvil-plan-spec` — Format / protocol

The **APS (Anvil Plan Spec)** format spec, parser, and validator. APS is a
lightweight markdown-based format for describing planning, task authorisation,
and progress tracking in AI-assisted development workflows. It is the format the
closed Anvil planning subsystem reads and writes.

**Why open:** A planning format is more valuable to eddacraft as an _adopted
standard_ than as a private file format. By open-sourcing the spec, the parser,
and the validator, we let other tools, agents, and workflows produce
APS-compatible plans that Anvil can consume — and we let teams adopt APS for
their own internal planning without requiring Anvil itself.

**Consumed by:**

- The closed Anvil CLI (via the `aps-loader` package)
- Third-party planning tools, agents, and AI workflows that adopt APS
- Anyone wanting a structured-but-human-readable plan format

**Contribution surface:** spec clarifications, validator improvements, language
bindings, format extensions, docs and examples.

### `kindling` — Memory primitive

**Kindling** provides small, composable memory primitives for agentic workflows:
observation, capture, basic stores, and the foundational event types that
downstream memory systems consume. It is the foundation that the closed Edda
(canonical memory) and Ember (interpretation pipeline) components build on.

**Why open:** Memory primitives benefit enormously from ecosystem reach. Open
primitives encourage integrations, third-party stores, alternative
implementations, and community-validated event schemas. The closed components
(Edda, Ember) layer interpretation, evolution, and provenance on top of these
primitives — none of which is given away by opening the foundation.

**Consumed by:**

- The closed Anvil CLI (Edda + Ember packages)
- Third-party agents and tools wanting structured observation capture
- Other memory systems that want to interoperate

**Contribution surface:** new store backends, additional event types, language
bindings, integration adapters, docs.

## Why these three (and not the rest)

These three repos are deliberately **protocol / primitive / infrastructure
layers** — exactly the things that benefit from being open:

- **Network effects.** An open APS spec is more valuable to eddacraft as an
  adopted standard than as a private format. Same logic as OpenTelemetry vs
  proprietary APMs, or LSP vs proprietary editor protocols.
- **Trust signal.** Publishing the format spec, the memory primitives, and the
  design system tells enterprise buyers "this is built on inspectable
  foundations."
- **Contribution surface.** Outside contributions to widgets, memory backends,
  or plan validators are _helpful_ without giving away product code.
- **None of them are the product.** Consuming all three primitives does not get
  a competitor anywhere close to having Anvil.

Anvil's actual value is the _combination_ of:

- The Rust kernel (semantic graph analysis, file watching, parsing)
- The policy engine and architecture engine
- Edda (canonical memory) and Ember (interpretation pipeline)
- The compliance policy pack catalogue (CPACKS)
- OPA enhancements, agent governance, policy lifecycle, federation
- The web dashboard with persistent views and historical analytics
- The hosted SaaS, auth, activation, and tier gating
- The integrations: GitHub Action, MCP server, IDE bridges
- The brand, the docs, the support, the enterprise contracts

None of those are in the three OSS repos. They live in the closed monorepo and
ship as binary releases only.

## How this is _not_ open core

Open core typically splits a single product into "free OSS edition" and "paid
enterprise edition" of the _same_ codebase. eddacraft does not do that.

What we have is:

- A **closed product** (Anvil) with a free tier gated by license server
- A separate **open primitives layer** that the closed product consumes
  alongside any other tool that wants to consume it

The open repos are not "Anvil community edition." They are foundations that
exist independently of Anvil and would have value even if Anvil did not exist.
This is closer to:

- The **PostgreSQL pattern** — open database protocol, closed managed services
  (Supabase, Neon, RDS, Crunchy)
- The **OpenTelemetry pattern** — open instrumentation, closed observability
  backends (Datadog, Honeycomb, New Relic)
- The **Language Server Protocol pattern** — open spec, closed editors (VS Code,
  Cursor, JetBrains)

…than to traditional open core (GitLab, Sentry-pre-FSL, Mattermost).

## Distribution implications

Because the product is closed-source, the install path is **binary-only**:

- **Linux / macOS:** `curl -fsSL https://install.eddacraft.ai | sh`
- **macOS (Homebrew):** `brew install eddacraft/tap/anvil`
- **Windows (WinGet):** `winget install eddacraft.anvil`
- **Windows (scoop):** `scoop bucket add eddacraft … && scoop install anvil`
- **Universal (GitHub Releases):** download the `.tar.xz` / `.zip` from
  <https://github.com/eddacraft/anvil/releases>

There is **no `cargo install anvil-cli`** path. The internal crates use the
`eddacraft-anvil-*` package-name prefix in their `Cargo.toml` files (for
namespace protection and future readiness), but no source is published to
crates.io. This is a deliberate consequence of the closed-source IP model — see
[ADR-018](../../plans/decisions/018-product-ip-architecture.md) for the full
reasoning.

## Future possibilities

The three-piece OSS surface is the _current_ shape, not necessarily the final
one. Areas where additional opening might happen later:

- **Compliance policy packs (CPACKS)** — the OWASP / SOC2 / ISO 27001 / GDPR /
  NIST AI RMF / EU AI Act policy text. Policy definitions are inherently
  community-curated knowledge and could become a fourth OSS repo once the engine
  that runs them is stable. The _engine_ would stay closed; only the policy
  _text_ would open.
- **The Anvil GitHub Action** wrapper.
- **The .anvil file format spec** (separate from its closed implementation).
- **Tree-sitter language adapters** for languages we add via the `lang-*`
  modules.

These are not commitments, just a record that the door is open. Any additional
opening will be captured in its own ADR.

## Contributing

- **`eddacraft-tui`** — see the repo's `CONTRIBUTING.md` (when added). Widget
  contributions, theme refinements, and accessibility improvements all welcome.
- **`anvil-plan-spec`** — spec clarifications and validator edge cases are the
  highest-value contributions.
- **`kindling`** — new store backends, additional event types, and integration
  adapters are the highest-value contributions.

For contributions to the closed Anvil product itself, eddacraft does not
currently accept outside PRs. Bug reports, feature requests, and feedback are
welcome via the GitHub issue tracker on
[`eddacraft/anvil`](https://github.com/eddacraft/anvil/issues).
