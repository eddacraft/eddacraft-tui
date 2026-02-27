# Anvil Production Strategy — Options Analysis

## Context

Anvil is a deterministic development automation platform from EddaCraft that catches
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
| **Free** | $0 | CLI + VS Code, 1 project, all anti-patterns & architecture checks, local only |
| **Pro** | $19/dev/mo | Unlimited projects, MCP server, CI mode |
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
   with minimal effort and offers a generous free tier. Clerk or Auth0 for
   consumer auth. Recommendation: WorkOS for enterprise SSO, simple
   email+password or GitHub OAuth for individual accounts.

---

## 5. IP Protection & Source Extraction Risk

### The Problem

npm packages are fully extractable. Anyone can run `npm pack @eddacraft/anvil-cli`
and get the complete published tarball. Before hardening, the esbuild config made this worse:

- **`sourcemap: true` (before hardening)** — shipped a `.js.map` that reconstructed original TypeScript
- **esbuild output is readable** — bundled but with meaningful variable names,
  clean control flow, and preserved string literals
- **All workspace packages bundled in** — `anvil-core`, `anvil-runtime`, the
  anti-pattern detection engine, architecture analysis, everything lands in one
  readable `dist/index.js`

The proprietary license is a legal deterrent but not a technical one. A competitor
or sufficiently motivated actor can extract, read, and reimplement the detection
logic with minimal effort.

### What's Actually Worth Protecting

| Component | Extractability | Value if copied |
|-----------|---------------|-----------------|
| Anti-pattern detectors | High (string patterns, AST rules) | Medium — patterns are the "what", the curation is the value |
| Architecture analysis engine | High (bundled in dist/) | High — core differentiator, hard to build from scratch |
| Gate/policy evaluation (OPA integration) | High | Low — OPA is open, integration is commodity |
| Tutorial/onboarding content | High | Low |
| Auth/token handling | High | Low |

The architecture analysis engine and the curated pattern set together are the
core IP. The composition of all detectors + their tuning + the watch-mode
integration is what makes Anvil hard to replicate even if individual patterns
are simple.

### Options Considered

| Approach | Effort | Protection | DX impact |
|----------|--------|-----------|-----------|
| **Minify + no sourcemaps** | 30 seconds | Low — stops casual reading, not determined actors | Worse debugging, harder-to-use stack traces |
| **JS obfuscation** | 1-2 days | Medium — control flow flattening, string encryption | 10-30% perf hit, useless stack traces |
| **Compiled binary** | 1-2 weeks | High — no JS on disk at all | Better (no Node requirement) |
| **Server-side core** | 2-4 weeks | Very high — IP never leaves servers | Worse for local/offline use |
| **Private npm registry** | 1 day | Medium — controls access, not extraction | Adds auth step to install |

### Decision: Compiled Binary (primary) + Immediate Hardening (now)

**Compiled binary is the right move.** It solves IP protection and distribution
in one shot:

1. No readable JS shipped — binary contains V8 bytecode or Bun's compiled form
2. Eliminates Node.js requirement — widens audience to Go/Python/Rust devs
3. Simpler install — `curl -fsSL ... | sh` or Homebrew, no `npm install`
4. Air-gapped friendly — just copy the binary
5. Consistent runtime — no "works on my Node version" issues

**Immediate hardening** (do now, regardless of binary timeline):
- `sourcemap: false` in esbuild config — stop shipping the map that
  reconstructs TypeScript source
- `minify: true` in esbuild config — collapse variable names, remove whitespace
- Verify `.npmignore` or `files` field excludes any stray source files

### Binary Compilation: Technical Assessment

**Recommended approach: Bun compile**

| Factor | Bun compile | Node SEA | pkg |
|--------|------------|----------|-----|
| Maturity | Stable | Stable (Node 22+) | Deprecated |
| Output size | ~30-50MB | ~50-80MB | ~50MB |
| ESM support | Native | Requires blob injection | Poor |
| Cross-compile | `--target=bun-linux-x64` etc. | Manual per-platform | Built-in |
| Native addons | Not embedded, loaded at runtime | Not embedded | Partial |
| Build speed | Fast | Moderate | Slow |

Bun is the best fit because: the CLI is already ESM (`"type": "module"`),
Bun's cross-compile flags are trivial, and it produces the smallest binaries.

**Key consideration: kindling packages with better-sqlite3**

The kindling packages (`@eddacraft/kindling-core`, `kindling-store-sqlite`,
`kindling-provider-local`) are kept external in the current esbuild config
because `kindling-store-sqlite` depends on `better-sqlite3`, a native C++
addon. Two options:

1. **Bundle kindling into the binary too** — Bun compile can embed JS but
   native `.node` addons must be distributed alongside. Use
   `--external better-sqlite3` and ship the platform-specific `.node` file
   next to the binary. This is what Turso's `libsql` does.

2. **Replace better-sqlite3 with a pure-JS or Bun-native alternative** —
   Bun has built-in `bun:sqlite` which is native to the runtime and embeds
   cleanly. Migrating `kindling-store-sqlite` from `better-sqlite3` to
   `bun:sqlite` eliminates the native addon problem entirely, at the cost of
   coupling the storage layer to Bun-specific APIs (and thus to Bun as the
   compiler/runtime). This is the cleaner path if Bun is the chosen compiler
   and you are comfortable with that lock-in.

**Runtime file access patterns** (must survive compilation):

- `template-loader.ts` reads template YAML files from disk via
  `readFile(filePath)` — these are user-project files, not bundled assets.
  Works fine in a binary since it reads from the user's filesystem, not from
  inside the binary.
- `file-io.ts` reads/writes project config files — same, filesystem-based.
- No embedded asset files (no `.ejs`, `.hbs`, `.yaml` templates baked into
  the package) — all templates are generated programmatically.
- Dynamic `import()` calls in `tutorial.ts` for lazy-loading TUI components —
  these need to be resolved at compile time by Bun (it handles this).

**Build matrix:**

| Target | Priority | Notes |
|--------|----------|-------|
| `bun-darwin-arm64` | P0 | macOS Apple Silicon (majority of devs) |
| `bun-linux-x64` | P0 | CI runners, Linux devs, Docker |
| `bun-darwin-x64` | P1 | macOS Intel |
| `bun-linux-arm64` | P1 | ARM CI, Graviton |
| `bun-windows-x64` | P2 | Windows devs |

**Distribution channels:**

| Channel | Mechanism | Priority |
|---------|-----------|----------|
| GitHub Releases | Binary attached to release tag | P0 |
| Install script | `curl -fsSL https://anvil.dev/install.sh \| sh` | P0 |
| Homebrew | `brew install eddacraft/tap/anvil` | P1 |
| npm (shimmed) | Thin wrapper that downloads the binary | P2 |

The npm shim approach (used by esbuild, Turbo, Biome) keeps `npx anvil` working
while shipping a binary. The npm package contains only a ~20-line postinstall
script that fetches the platform-appropriate binary.

### npm Shim Pattern (for backwards compatibility)

Keep publishing `@eddacraft/anvil-cli` to npm but change what it contains:

```
@eddacraft/anvil-cli/
  package.json        # bin entry + postinstall
  install.js          # Downloads platform binary from GitHub Releases
  bin/anvil           # Shell script that execs the downloaded binary
```

Platform-specific optional dependencies (the esbuild/Biome pattern):

```
@eddacraft/anvil-cli-darwin-arm64
@eddacraft/anvil-cli-darwin-x64
@eddacraft/anvil-cli-linux-x64
@eddacraft/anvil-cli-linux-arm64
@eddacraft/anvil-cli-win32-x64
```

Each contains just the binary for that platform. npm's `optionalDependencies`
ensures only the right one is installed. This is the gold standard — it's how
esbuild, Biome, SWC, and Turbo all distribute.

---

## 6. Implementation Priorities

This section is for reference — no code changes in this plan.

**Phase 0 (Immediate — this week):**
- esbuild hardening: `sourcemap: false`, `minify: true`
- Verify no source files leak into npm package

**Phase 1 (Beta → GA):**
- Compiled binary via Bun compile (5 platform targets)
- npm shim package with platform-specific optional deps
- Install script + GitHub Releases distribution
- Account system (replace invite tokens with real accounts)
- Free tier (remove token requirement for basic CLI use)
- Pro tier billing (Stripe integration)
- GitHub Action v1

**Phase 2 (GA → Growth):**
- Homebrew tap
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

## 7. Verification

This is a strategy document — no code changes to verify. The next
step is to decide on the key questions in section 4, then create
implementation plans for the chosen approach.
