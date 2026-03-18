# Anvil -- Investor One-Pager

**AI Governance for Developers** | EddaCraft | anvil.eddacraft.ai

---

## The Situation

AI coding assistants are writing 46% of production code. GitHub Copilot has 4.7 million paid subscribers and 90% Fortune 100 adoption. The generation problem is solved -- AI writes code faster than humans. But no governance layer exists between AI generation and production deployment.

## The Problem

AI-generated code produces 1.7x more defects, fails security tests 45% of the time, and fewer than half of developers review it before committing. Code duplication has increased 4x. Refactoring has collapsed. Every existing tool -- static analysers, AI reviewers, supply chain scanners -- operates after commit. Nobody governs code at the point of generation.

## The Solution: Anvil

Anvil is a deterministic policy engine that enforces governance at file save -- the moment code is generated.

- **Pre-commit enforcement**: operates at file save, not PR time or CI
- **Deterministic analysis**: policy-as-code (OPA/Rego), not AI reviewing AI
- **Authorship attribution**: every line classified as human, AI, mixed, or unknown
- **Architecture drift detection**: persistent semantic graph tracks structural trajectory
- **Built in Rust**: ships as a single binary, runs in the terminal

Anvil is the only tool that is both deterministic and pre-commit. Every competitor is either post-commit, probabilistic, or both.

## The Market

| Segment | Size | Source |
|---------|------|--------|
| TAM (AI code tools + AppSec + governance) | USD 21.5B (2025) | Mordor Intelligence, Gartner |
| AI governance platforms | USD 492M (2026), >USD 1B (2030) | Gartner (Feb 2026) |
| SAM (governance for AI-assisted dev) | USD 1.5-2.0B (2026) | Derived |

**Regulatory driver**: EU AI Act high-risk requirements enforceable August 2026. Penalties: up to 7% of global annual turnover.

## Traction

- Functional product: Rust kernel, Ratatui TUI, OPA/Rego engine, authorship attribution
- [EVIDENCE NEEDED: waitlist, design partners, community metrics, revenue]

## Business Model

Developer-led adoption (CLI install, bottom-up). Per-seat subscription. Policy packs for compliance frameworks (SOC 2, HIPAA, EU AI Act) as expansion revenue. Enterprise tier with centralised management, SSO, audit dashboards.

## The Ask

[EVIDENCE NEEDED: funding amount, use of funds, milestones]

---

*Anvil by EddaCraft -- the adult in the room for AI-assisted development.*
