# Competitive Landscape: Anvil by EddaCraft

## Executive Summary

No existing tool occupies Anvil's position: deterministic, pre-commit governance with line-level authorship attribution. The market is segmented into four adjacent categories, each solving part of the problem. Anvil's differentiation is structural, not incremental.

---

## Category Map

### 1. Static Analysis / Code Quality

| Player | What They Do | Scale | Anvil Differentiator |
|--------|-------------|-------|---------------------|
| SonarQube / SonarSource | Post-commit code quality scanning across 35+ languages. Quality gates block deploys. | USD 4.7B valuation, 412M funding, 250-500M est. revenue | Anvil operates at file save, not post-commit. Architecture-aware, not just pattern matching. Authorship attribution (human/AI/mixed/unknown) is absent from SonarQube. |
| Codacy | Cloud-based code quality and coverage platform | Smaller scale, enterprise tier | Same timing gap as SonarQube. No governance layer for AI-generated code. |

**Category weakness**: These tools treat all code the same regardless of origin. They scan after commit, when the cost of fixing is higher. No authorship tracking. No architecture drift detection.

### 2. AI-Powered Code Review

| Player | What They Do | Scale | Anvil Differentiator |
|--------|-------------|-------|---------------------|
| CodeRabbit | AI-powered PR review with contextual feedback | Growing rapidly; self-described as best dedicated AI code review tool (2026) | Anvil is deterministic, not probabilistic. It does not use AI to review AI -- it enforces policy-as-code. Fires at file save, not at PR time. |
| Sourcery | AI code review and refactoring suggestions | Smaller scale | Same fundamental issue: AI reviewing AI introduces compounding uncertainty. |
| Qodo (formerly CodiumAI) | AI test generation and code review | Growing | Complementary rather than competitive; Qodo generates tests, Anvil governs architecture. |

**Category weakness**: AI reviewing AI-generated code creates a recursive trust problem. These tools are probabilistic -- they suggest, they do not enforce. They operate at PR time, not at generation time.

### 3. Software Supply Chain / Application Security

| Player | What They Do | Scale | Anvil Differentiator |
|--------|-------------|-------|---------------------|
| Snyk | SCA, SAST, container, IaC security scanning. 11 tools from single integration. | USD 8.5B valuation, USD 408M revenue (2025), 1.25B raised | Snyk secures dependencies and known vulnerabilities. Anvil governs architecture, authorship, and policy compliance. Different attack surface. |
| Semgrep | Pattern-based static analysis for security and policy violations. Syntax-tree level matching. | USD 500M-1B est. valuation, USD 204M raised | Semgrep is the closest technical analogue but operates post-commit, lacks authorship attribution, and does not track architecture drift over time. |
| Socket | Dependency risk analysis focused on supply chain attacks | Growing, well-funded | Socket watches packages; Anvil watches code provenance and architecture evolution. |

**Category weakness**: Supply chain tools focus on external dependencies. None address the governance gap for code generated within the repository by AI assistants. Authorship attribution is absent.

### 4. AI Governance (Emerging)

| Player | What They Do | Scale | Anvil Differentiator |
|--------|-------------|-------|---------------------|
| DryRun Security | AI SAST and code policy enforcement in agentic coding workflows | Early stage | Closest competitor by positioning. Anvil differentiates with deeper architecture analysis, TUI/CLI integration, file-save enforcement, and deterministic (non-AI) analysis. |
| Various enterprise GRC platforms | Broad governance, risk, compliance -- not code-specific | USD 45B+ market (eGRC) | These are not developer tools. They operate at the organisational level, not the codebase level. Anvil is the developer-facing governance layer. |

**Category weakness**: The AI governance space is nascent. Most tools focus on model governance (training data, bias, outputs), not code governance (what AI writes into your codebase). Anvil occupies the code governance gap specifically.

---

## Positioning Matrix

```
                    Pre-commit                    Post-commit
                    (file save)                   (PR / CI)
                    ─────────────────────────────────────────
Deterministic       │ ANVIL              │ SonarQube        │
(policy-based)      │ (governance +      │ Semgrep          │
                    │  attribution)      │ Snyk             │
                    ├────────────────────┼──────────────────┤
Probabilistic       │ [empty]            │ CodeRabbit       │
(AI-powered)        │                    │ Sourcery         │
                    │                    │ Qodo             │
                    ─────────────────────────────────────────
```

**Anvil is the only tool in the top-left quadrant**: deterministic governance at pre-commit (file save) time. Every other tool is either post-commit, probabilistic, or both.

---

## Key Competitive Differentiators

| Differentiator | Anvil | Static Analysis | AI Code Review | Supply Chain |
|---------------|-------|-----------------|----------------|--------------|
| **Fires at file save** | Yes | No (CI/PR) | No (PR) | No (CI) |
| **Deterministic analysis** | Yes | Yes | No (AI-based) | Yes |
| **Authorship attribution** | Yes (human/AI/mixed/unknown) | No | No | No |
| **Architecture drift detection** | Yes (incremental graph) | Limited | No | No |
| **Policy-as-code (OPA/Rego)** | Yes | Custom rules only | No | Limited |
| **Line-level provenance** | Yes | No | No | No |
| **AI anti-pattern detection** | Yes (purpose-built) | Generic patterns | AI-driven | No |

---

## Competitive Moat Assessment

1. **Timing moat**: Pre-commit enforcement is architecturally different from post-commit scanning. Retrofitting existing tools to operate at file save requires fundamental re-architecture.

2. **Attribution moat**: Line-level authorship tracking (human/AI/mixed/unknown) requires deep integration with the development workflow. Bolt-on attribution is unreliable.

3. **Policy moat**: OPA/Rego policy-as-code gives teams control over their own governance rules. Proprietary rule engines create vendor lock-in; Anvil uses open standards.

4. **Architecture awareness moat**: Maintaining a persistent semantic graph of the repository (dependency, trust, plan graphs) is a capability that no competitor has built. This enables trajectory-based analysis ("trending toward instability") rather than point-in-time scanning.

---

## Threat Assessment

| Threat | Likelihood | Impact | Mitigation |
|--------|-----------|--------|-----------|
| SonarQube adds AI authorship tracking | Medium (2-3 years) | High | Build attribution depth that takes years to replicate. Community + OSS moat. |
| Semgrep moves to pre-commit | Medium | Medium | Semgrep lacks architecture graph and attribution. Pre-commit is necessary but not sufficient. |
| GitHub native governance features | High (GitHub has the data) | Very High | Speed to market. Anvil is tool-agnostic (not GitHub-only). Policy-as-code flexibility. |
| New startup in same quadrant | Medium | Medium | First-mover advantage in the deterministic pre-commit governance space. |

---

## Sources

- CodeRabbit, "State of AI vs Human Code Generation Report" (2025)
- Gartner, "Global AI Regulations Fuel Billion-Dollar Market for AI Governance Platforms" (Feb 2026)
- Mordor Intelligence, AI Code Tools Market Report (2025)
- DryRun Security, "Top 10 AI SAST Tools for 2026" (2026)
- Stack Overflow 2025 Developer Survey
- Tracxn company profiles: Semgrep, Snyk, SonarSource (2025-2026)
