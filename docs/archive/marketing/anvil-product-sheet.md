# Anvil by eddacraft — Product Information Sheet

## What Is Anvil?

**Anvil is a deterministic development automation platform that catches
architecture drift, AI anti-patterns, and policy violations at file save —
before they ever reach code review.**

It validates every code change against your team's architecture rules, security
policies, and quality standards in milliseconds, giving developers instant
feedback while maintaining a cryptographically signed audit trail of every
decision.

Anvil is not a linter. It is a **structural governance layer** for your
codebase.

---

## Who Is Anvil For?

### Engineering Teams Using AI Coding Tools

AI assistants generate code fast — but they don't understand your architecture.
Anvil acts as a deterministic guardrail that validates AI-generated changes
against your team's boundaries, conventions, and security policies before they
land.

### Platform & DevOps Engineers

Enforce consistent architecture and dependency rules across teams without
blocking velocity. Define policies once, enforce everywhere — CLI, IDE, CI, and
PR review.

### Engineering Leaders & CTOs

Get verifiable proof that code changes — whether human or AI-authored — meet
your organisation's standards. Anvil's audit trail provides cryptographically
signed evidence for compliance, SOC 2, and regulatory requirements.

### Solo Developers & Small Teams

Anvil works out of the box with zero configuration. It analyses your existing
codebase and dependency structure as the baseline, so you get value from the
first scan without writing a single rule.

---

## Core Capabilities

### Quality Gates

Deterministic validation checkpoints that run automatically on every change:

| Gate              | What It Catches                                              |
| ----------------- | ------------------------------------------------------------ |
| Architecture      | Layer violations, circular dependencies, boundary breaches   |
| Anti-Pattern      | Explicit `any`, empty catch blocks, `ts-ignore`, debug code  |
| Secret Detection  | API keys, passwords, credentials via pattern + entropy scan  |
| OPA/Rego Policies | Custom organisational rules (file size, naming, conventions) |
| ESLint            | Lint errors surfaced as gate results                         |
| Coverage          | Configurable line and branch thresholds                      |
| Dependency        | Licence compliance, vulnerability checks                     |

Gates are composable, configurable, and cacheable. Unchanged files skip
re-validation automatically.

### Real-Time Watch Mode

```bash
anvil watch --source
```

Validates on every file save in milliseconds. Runs in a terminal pane or via the
VS Code extension for in-editor diagnostics. No waiting for CI — issues surface
the moment you introduce them.

### Architecture Enforcement

Define module boundaries and dependency rules declaratively:

```json
{
  "architecture": {
    "boundaries": [
      {
        "name": "api-layer",
        "pattern": "src/api/**",
        "deny": ["src/repositories/**"]
      }
    ]
  }
}
```

Anvil enforces these boundaries deterministically. The same input always
produces the same output — no flaky results, no race conditions.

### Drift Detection

Capture architectural snapshots over time and track how your codebase evolves.
Anvil detects when boundaries erode, coupling increases, or trust surfaces
expand — warning you before entropy wins.

### Custom Policy Engine (OPA/Rego)

Write custom rules in OPA/Rego for your organisation's specific standards:

```rego
package anvil.policy

deny[msg] {
  input.file.path == "src/index.ts"
  count(input.file.lines) > 500
  msg := "src/index.ts exceeds 500 lines"
}
```

Ship policies as reusable policy packs with versioning, testing, and
inheritance.

### AI Authorship Tracking

Trace which changes were AI-generated via Git Notes. Know exactly what was
written by a human and what was written by an AI assistant — critical for
compliance and code review workflows.

### Cryptographic Audit Trail

Every validation run produces a signed evidence record:

- **What** was validated (files, hashes)
- **When** it was validated (timestamps)
- **Which rules** were active (configuration hash)
- **What the result** was (pass/fail/warn)
- **Cryptographic signature** for tamper detection

Evidence can be exported for compliance audits, attached to Git commits, or
pushed to remote storage (S3).

### Plan Validation

Validate planning documents — APS (Anvil Plan Spec), SpecKit, BMAD — before
execution. Ensure planned changes align with architecture rules before a single
line of code is written.

---

## How It Works

### 1. Install & Initialise

```bash
curl -fsSL https://install.eddacraft.ai | sh
anvil init
```

Anvil detects your project type, creates configuration, and baselines existing
code. You get value in under 5 minutes.

### 2. Scan & Fix

```bash
anvil check --all
```

See what Anvil catches. Existing issues are baselined so you are never
overwhelmed — only new violations surface.

### 3. Watch & Enforce

```bash
anvil watch --source
```

Continuous validation as you code. Every save is checked in milliseconds.

### 4. Integrate & Scale

Add Anvil to CI with a single step. A reusable GitHub Action is provided:

```yaml
- uses: eddacraft/anvil-001/.github/actions/anvil-check@main
```

---

## Surfaces & Integrations

| Surface            | Description                                                    |
| ------------------ | -------------------------------------------------------------- |
| **CLI**            | Full-featured terminal interface (Rust + clap + Ratatui TUI)   |
| **VS Code**        | Real-time in-editor diagnostics and warnings                   |
| **MCP Server**     | Model Context Protocol integration for AI tools                |
| **GitHub Actions** | CI/CD gate checks and PR annotations                           |
| **REST API**       | Programmatic access for dashboards and automation              |
| **Web Dashboard**  | Visual overview of gates, drift, and warnings (in development) |

---

## The Edda Stack — Observational Memory

Anvil includes a unique observational memory system called the **Edda Stack**,
designed for teams working with AI coding agents:

| Layer        | Purpose                                                           |
| ------------ | ----------------------------------------------------------------- |
| **Kindling** | Raw observations — what happened during a coding session          |
| **Ember**    | Pattern proposals — recurring behaviours detected across sessions |
| **Edda**     | Confirmed memories — validated patterns that inform governance    |

The Edda Stack lets Anvil learn from your codebase's evolution over time,
detecting emerging anti-patterns and architectural trends before they become
structural problems.

---

## Design Principles

1. **Planless-first** — Delivers value without requiring plans, specs, or
   upfront configuration. Your codebase is the baseline.

2. **Deterministic** — Same input, same output. Always. No flaky results.

3. **Composable** — Every check, policy, and surface is independently usable.

4. **Safety by default** — Warnings over blocks. Existing violations are
   baselined on first run. Developers are never overwhelmed.

---

## Technology

- **Rust** native binary — single static binary, zero runtime dependencies,
  10-40x faster than the previous Node.js implementation
- **Tree-sitter** for incremental AST parsing (<1ms per file)
- **Ratatui** for a native terminal UI with real-time watch dashboard
- **OPA/Rego** policy engine for custom rule authoring
- **Zod** schemas for type-safe contracts (TypeScript domain packages)
- Designed for **monorepos** (NX, Turborepo, pnpm workspaces)

---

## Pricing & Availability

Anvil is currently in **closed beta**. Request access at
**[eddacraft.ai](https://eddacraft.ai/#waitlist)**.

- Open-source core (Apache-2.0)
- TypeScript and JavaScript projects supported (Rust language support planned)
- Ships as a single binary — no runtime dependencies

---

## Why Anvil?

> Most tools are reactive. They tell you what's broken after the fact.
>
> Anvil is anticipatory. It catches structural problems the moment they're
> introduced — and tracks the trajectory of your architecture over time.
>
> We're not building a linter. We're building a **constitutional layer for
> software**.

---

**eddacraft** | [eddacraft.ai](https://eddacraft.ai) |
[GitHub](https://github.com/eddacraft/anvil-001) |
[Documentation](https://eddacraft.ai/beta)
