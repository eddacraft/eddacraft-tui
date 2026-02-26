# Anvil Production Strategy — Options Analysis

## Context

Anvil is a deterministic development automation platform (EddaCraft) that catches
architecture drift and AI anti-patterns at save-time. Currently in closed beta,
distributed as `@eddacraft/anvil-cli` on npm with invite tokens. The product
consists of: CLI, VS Code extension, MCP server, REST API (Hono/Vercel/Neon),
docs site, marketing website, and a web UI in development.

This document lays out options across three dimensions — **distribution**,
**access/auth**, and **business model** — then proposes a cohesive recommended
bundle.

---

## 1. Distribution & Installation

### Option A: npm-only (current)

```bash
npm install -g @eddacraft/anvil-cli
npx @eddacraft/anvil-cli tutorial
```

**Pros:** Zero friction for JS/TS ecosystem. Existing publish pipeline works.
Provenance signing already enabled. Users understand npm.

**Cons:** Requires Node 20+. Couples to npm infrastructure. Enterprise
air-gapped environments may not have npm access. Difficult to distribute
non-JS components (OPA binary, VS Code ext) as a single install.

**Best for:** Current beta, JS/TS-only audience.

### Option B: Standalone binary (pkg / Bun compile / SEA)

Package the CLI as a self-contained binary — no Node required.

| Approach | Maturity | Output |
|----------|----------|--------|
| Node SEA (Single Executable App) | Stable in Node 22+ | ~50-80MB binary |
| Bun compile | Stable | ~30-50MB binary |
| pkg (Vercel) | Deprecated but works | ~50MB binary |

**Pros:** Zero runtime dependency. Simpler install (download + run).
Easier for non-JS teams (Go, Python, Rust devs). Air-gapped friendly
(just copy the binary). Can be distributed via Homebrew, apt, etc.

**Cons:** Larger download. Cross-platform build matrix (linux-x64,
linux-arm64, darwin-x64, darwin-arm64, win-x64). Need to handle OPA
binary bundling. More complex release pipeline.

**Best for:** Multi-language expansion, enterprise, reducing adoption friction.

### Option C: Docker image

```bash
docker run --rm -v $(pwd):/project eddacraft/anvil check --all
```

**Pros:** Fully isolated. Great for CI. Air-gapped via registry mirror.
Consistent environment. No local install needed.

**Cons:** Terrible DX for watch mode (volume mounts, file events).
Overhead for simple local use. Docker dependency is a big ask for
individual developers.

**Best for:** CI/CD pipelines, enterprise environments with Docker registries.

### Option D: GitHub Action (complementary)

```yaml
- uses: eddacraft/anvil-action@v1
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
```

Already partially exists in docs. This isn't a primary distribution but
is a critical companion — many teams will first encounter Anvil via CI
before adopting locally.

**Best for:** Team adoption entry point. Low-friction trial.

### Recommendation: **A + B + D** (npm now, binary next, Action always)

Keep npm as primary for beta. Add standalone binary for GA to widen the
audience. GitHub Action as the team adoption wedge. Docker only if
enterprise demand materialises.

---

## 2. Access Control & Authentication

### Option A: API token (current)

`anvil login` exchanges a beta invite token for an API-verified session.
Tokens stored locally. API at `anvil-api` on Vercel/Neon manages tokens.

**Pros:** Simple. Already built. Works for beta scale.

**Cons:** No identity — tokens aren't tied to users in a recoverable way.
No team management. No SSO. No audit trail of who did what. Token
rotation is manual.

### Option B: License key (offline-capable)

Cryptographically signed license key containing: org ID, seat count,
tier, expiry. Validated locally without phoning home.

```
ANVIL-ENT-abcdef1234-20270101-50seats-sig
```

**Pros:** Works air-gapped. No uptime dependency. Familiar model
(JetBrains, Sublime). Can encode tier + seat count. Offline-first.

**Cons:** Revocation is hard (need periodic online check or short
expiry + renewal). Piracy risk (key sharing). No usage telemetry.
Harder to do seat management dynamically.

**Best for:** Enterprise/air-gapped. Can layer on top of account system.

### Option C: Account-based auth (OAuth / SSO)

Users create an EddaCraft account (email + password or OAuth). CLI
authenticates via device flow (like `gh auth login`). Teams are
first-class entities.

```bash
anvil auth login          # Opens browser, device flow
anvil auth status         # Shows org, tier, seats
anvil team add user@co.com
```

**Pros:** Real identity. Team/org management. SSO for enterprise
(SAML/OIDC). Audit trail. Seat management. Enables dashboard tied to
identity. Industry standard.

**Cons:** Requires building auth infrastructure (or using Auth0/Clerk/
WorkOS). Online dependency for initial auth (can cache locally). More
complex than tokens.

**Best for:** GA and beyond. Required for team/enterprise tiers.

### Option D: Git-based identity (lightweight)

Use git author (email) as identity. Validate against a team allowlist
in `.anvilrc` or org config. No separate auth system.

**Pros:** Zero friction. Works immediately. No account creation.
Git email is already available everywhere.

**Cons:** Not secure — git email is trivially spoofable. No payment
tie-in. No real access control. Can't revoke. Can't bill.

**Best for:** Free tier / open-source use only.

### Recommendation: **A → C (+ B for enterprise)**

Keep tokens for now. Build account-based auth for GA using a service
like WorkOS (which gives you SSO/SAML for enterprise for free until
scale). Add license keys as an enterprise add-on for air-gapped
deployments. Use git identity only for the free tier (no auth needed).

---

## 3. Business Model

### Option A: Open Core

Open-source the core engine (`anvil-core`, `anvil-runtime`, anti-pattern
detection, architecture checks). Charge for:
- Dashboard / web UI
- Team features (shared config, org policies)
- Enterprise features (SSO, RBAC, audit export, remote cache)
- MCP server (pro feature for AI-assisted teams)

**Examples:** ESLint (fully open, funded by sponsors), SonarQube (community
edition free, developer/enterprise paid), Semgrep (open rules engine,
paid platform).

**Pros:** Maximises adoption. Community contributions. Trust through
transparency. Easier hiring (people know the tool). Strong competitive
moat if community forms.

**Cons:** Revenue delayed. Competitors can fork. Hard to draw the
line between free and paid. Open-source maintenance burden. Need to
find features worth paying for that don't feel like hostage-taking.

**Revenue risk:** High in early stage. Open-source dev tools are
notoriously hard to monetise (ESLint is chronically underfunded).

### Option B: Freemium SaaS

Free tier with usage limits. Paid tiers unlock more:

| Tier | Price | What you get |
|------|-------|-------------|
| **Free** | $0 | CLI + VS Code, 1 project, 3 anti-patterns, local only |
| **Pro** | $19/dev/mo | Unlimited projects, all patterns, MCP server, CI mode |
| **Team** | $39/dev/mo | Shared config, dashboard, org policies, PR comments |
| **Enterprise** | Custom | SSO, RBAC, air-gapped, audit export, SLA, support |

**Examples:** Snyk (free for individuals, paid for teams), Vercel (free
tier, usage-based pro), Linear (free for small teams).

**Pros:** Clear upgrade path. Revenue from day 1 of GA. Self-serve
for pro/team. Enterprise for larger deals. Aligns price with value
(more developers = more seats = more revenue).

**Cons:** Free tier must be genuinely useful or nobody converts.
Per-seat can feel punitive as teams grow. Need billing infrastructure.
Need to define and enforce limits.

### Option C: Usage-based

Charge per analysis run, per repo, or per violation detected/fixed.

| Metric | Price |
|--------|-------|
| Per gate run | $0.001 |
| Per repo/month | $10 |
| Per active developer/month | $15 |

**Pros:** Aligns cost with value delivered. Low barrier to start.
Scales naturally with usage.

**Cons:** Unpredictable costs scare buyers. Hard to budget. Watch
mode generates thousands of runs — do you count those? Metering
infrastructure needed. Enterprise procurement prefers predictable costs.

**Best for:** Platform/API play (charge per API call to the Anvil
service). Not ideal as primary model for a CLI tool.

### Option D: Proprietary per-seat (current trajectory)

Keep everything proprietary. Sell seats.

| Tier | Price | What you get |
|------|-------|-------------|
| **Individual** | $9/mo | Full CLI + VS Code, 1 user |
| **Team** | $29/dev/mo | + Dashboard, shared config, CI |
| **Enterprise** | Custom | + SSO, RBAC, audit, support |

**Examples:** JetBrains (fully proprietary, per-seat), GitLab (tiers),
Linear (per-seat).

**Pros:** Simple. All features are revenue-generating. No open-source
maintenance burden. Clear value proposition at each tier. Full control
over roadmap.

**Cons:** Higher adoption friction (no free path). Harder to build
community. Competitors with open-source alternatives may undercut.
Every user needs to pay or trial.

### Option E: Source-available + commercial license

Publish source code under a restrictive license (BSL, SSPL, or
Functional Source License). Free for individuals/small teams.
Commercial license required for organisations above a threshold.

**Examples:** MariaDB (BSL), HashiCorp (BSL), Sentry (FSL), Elastic
(SSPL).

**Pros:** Source visible (builds trust, allows inspection).
Community can report bugs and contribute. Converts to open-source
after N years (BSL). Protects against cloud providers reselling.
Simpler than open-core (one product, one license, two tracks).

**Cons:** "Fauxpen source" criticism. Some enterprises won't touch
BSL/SSPL. Community contributions are limited by CLA requirements.
Legal complexity.

### Recommendation: **B (Freemium) with elements of E (source-available)**

**Primary model: Freemium per-seat with a generous free tier.**

Rationale:
- Anvil needs adoption before it needs revenue
- A genuinely useful free tier (full CLI, all anti-patterns, local use)
  creates word-of-mouth
- Team/enterprise features (dashboard, shared config, SSO) are natural
  paid tiers that don't feel extractive
- Source-available (FSL or BSL) for the core engine builds trust without
  giving away the platform

---

## 4. Cohesive Recommended Strategy

### The Bundle

| Dimension | Now (Beta) | GA (v1.0) | Scale (v2.0+) |
|-----------|-----------|-----------|---------------|
| **Distribution** | npm only | npm + standalone binary + GH Action | + Docker, Homebrew |
| **Auth** | Invite tokens | Account-based (device flow) | + SSO/SAML, license keys |
| **Model** | Free beta | Freemium (Free / Pro / Team) | + Enterprise tier |
| **Source** | Proprietary | Source-available (FSL) for core | Core → open after 2yr |

### Free Tier (the adoption engine)

- Full CLI with all anti-patterns and architecture checks
- VS Code extension
- Local watch mode
- Single-project, single-user
- Community support (GitHub issues)
- No account required (git identity only)
- No time limit

**Why generous:** The free tier IS the marketing. Every developer
running Anvil locally is a potential team conversion. Save-time feedback
is the "aha moment" — don't gate it.

### Pro Tier ($19/dev/month)

- Everything in Free
- Unlimited projects
- MCP server (AI tool integration)
- CI mode (`--ci` flag, exit codes, JSON output)
- `anvil fix` auto-remediation
- Email support
- Account required

**Why this line:** CI mode and MCP server are where Anvil starts
touching team infrastructure. The developer who needs CI integration
is already getting enough value to justify the cost.

### Team Tier ($39/dev/month)

- Everything in Pro
- Dashboard (web UI)
- Shared org configuration (`extends` from org package)
- Team-wide suppression governance
- PR comments via GitHub Action
- Drift trend tracking
- Remote/shared cache
- Priority support

**Why this line:** These are features that only matter when >1 person
is using Anvil. The buyer is a team lead or engineering manager.

### Enterprise Tier (custom pricing)

- Everything in Team
- SSO (SAML/OIDC via WorkOS)
- RBAC (viewer/developer/architect/admin)
- Audit export (SARIF, compliance reports)
- Air-gapped deployment (license keys + binary)
- Custom policy authoring support
- SLA + dedicated support
- On-prem option

**Why this line:** Compliance, security, and control features that
only matter to organisations with procurement processes.

### Competitive Positioning

| Competitor | Model | Anvil's differentiation |
|-----------|-------|------------------------|
| ESLint | Free/OSS (sponsorship) | Anvil does architecture, not just lint rules |
| Semgrep | Open core | Anvil is save-time, not scan-time. AI-specific patterns |
| SonarQube | Open core | Anvil is local-first, not server-first |
| Snyk | Freemium | Anvil is architecture, not vulnerability |
| Biome | Free/OSS | Anvil is analysis, not formatting/linting |

**Anvil's unique angle:** Save-time architecture enforcement for
AI-assisted development. Nobody else occupies this exact position.

### Key Decisions to Make

1. **Source-available or fully proprietary?** FSL builds trust and
   enables inspection, but adds legal complexity. Proprietary is
   simpler but creates more adoption friction.

2. **Where to draw the free/paid line?** The recommendation above
   gates CI mode behind Pro. An alternative: gate nothing in the CLI
   and only charge for the platform (dashboard, shared config, SSO).
   This is more generous but delays revenue further.

3. **Per-seat or per-repo pricing?** Per-seat is simpler and standard
   for dev tools. Per-repo favours small teams with many repos.
   Per-seat is recommended — it's what buyers expect.

4. **Annual vs monthly billing?** Offer both. Annual at ~15-20%
   discount to improve cash flow predictability. Enterprise is always
   annual.

5. **Build auth in-house or use a service?** WorkOS gives you SSO/SAML
   with minimal effort and is free until 1M MAU. Clerk or Auth0 for
   consumer auth. Recommendation: WorkOS for enterprise SSO, simple
   email+password or GitHub OAuth for individual accounts.

---

## 5. Implementation Priorities

This section is for reference — no code changes in this plan.

**Phase 1 (Beta → GA):**
- Account system (replace invite tokens with real accounts)
- Free tier (remove token requirement for basic CLI use)
- Pro tier billing (Stripe integration)
- Standalone binary distribution
- GitHub Action v1

**Phase 2 (GA → Growth):**
- Team tier features (dashboard, shared config)
- Team billing and seat management
- Source-available license for core packages
- Usage telemetry (opt-in)

**Phase 3 (Growth → Enterprise):**
- SSO/SAML (WorkOS)
- RBAC
- Air-gapped deployment (license keys + binary)
- Audit/compliance export
- Enterprise sales motion

---

## 6. Verification

This is a strategy document — no code changes to verify. The next
step is to decide on the key questions in section 4, then create
implementation plans for the chosen approach.
