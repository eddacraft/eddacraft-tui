# Investor FAQ: Anvil by EddaCraft

Top 10 questions with concise answers.

---

## 1. What does Anvil actually do?

Anvil is a governance engine that enforces policy on code at file save. When a developer saves a file, Anvil parses it, classifies each line's authorship (human, AI, mixed, or unknown), evaluates it against the team's policy-as-code rules (OPA/Rego), checks for architecture drift, and emits a governance event: pass, warn, or block. It runs in the terminal as a CLI and TUI, built in Rust.

## 2. Why is governance needed for AI-generated code specifically?

AI-generated code produces 1.7x more defects, 1.4x more critical issues, and fails security tests 45% of the time (Veracode, 100+ models). Fewer than half of developers review AI output before committing. Code duplication has quadrupled. Meanwhile, 46% of production code is now AI-generated. The volume is too high for manual review, and the quality gap is measurable.

## 3. Why file save and not PR time or CI?

By PR time, ungoverned code is already in the codebase. The cost of fixing defects rises 10-100x between writing and deployment. Anvil intercepts at the earliest possible point -- file save -- when the developer is still in context and the fix is cheapest. No other tool operates here.

## 4. Why deterministic and not AI-powered?

Using AI to review AI-generated code compounds probabilistic uncertainty. The best AI model produces secure code only 56% of the time (BaxBench). Anvil uses OPA/Rego policy-as-code: the same input always produces the same output. This is auditable, predictable, and does not introduce its own failure modes.

## 5. Who buys this?

The primary buyer is the engineering leader (VP Engineering, CTO) at organisations with 20+ developers using AI coding assistants. The initial adopter is the developer who installs via CLI. The purchase trigger is a compliance requirement: SOC 2 audit, EU AI Act deadline, enterprise customer asking about AI code governance. Secondary buyers: security and compliance teams.

## 6. What is the competitive landscape?

No existing tool is both deterministic and pre-commit. Static analysis tools (SonarQube, Semgrep) are deterministic but post-commit. AI code review tools (CodeRabbit, Sourcery) are probabilistic and post-commit. Supply chain tools (Snyk) cover dependencies, not code provenance. Anvil occupies a unique quadrant in the market.

## 7. What is the regulatory driver?

The EU AI Act high-risk requirements become enforceable in August 2026 -- five months from now. Non-compliance penalties reach 7% of global annual turnover. Gartner forecasts AI governance platform spend at USD 492M in 2026, growing past USD 1B by 2030. Regulations are converting governance from optional to mandatory.

## 8. How does Anvil make money?

Per-seat subscription pricing. Free open-source core for community adoption. Team tier for governance features. Enterprise tier for centralised policy management, SSO, audit dashboards. Policy packs (pre-built rule sets for SOC 2, HIPAA, EU AI Act) as add-on revenue. Land with developers, expand with compliance.

## 9. What is the technology moat?

Four layers. (1) Timing: pre-commit enforcement requires different architecture than post-commit -- cannot be bolted on. (2) Attribution: line-level authorship tracking requires deep workflow integration. (3) Policy: OPA/Rego is an open standard, preventing vendor lock-in (competitive advantage, not risk). (4) Architecture graph: persistent semantic graph tracking dependency, trust, and plan graphs incrementally -- no competitor has this.

## 10. What stage is the product?

Anvil has a working Rust kernel, Ratatui TUI, OPA/Rego policy engine, authorship attribution, and architecture analysis. The product is functional, not a prototype. [EVIDENCE NEEDED: specific metrics on product maturity, testing, coverage, performance benchmarks.]

## 11. Why £15–25M pre-money for a pre-revenue company?

The valuation reflects four factors: (1) category heat — AI governance is the hottest new category in developer tooling, with record pre-seed rounds establishing valuation precedent; (2) product maturity — Anvil is a production-grade Rust system with a working policy engine, semantic graph, and authorship attribution, while most competitors are pre-product; (3) regulatory forcing function — the EU AI Act creates mandatory spend with a known deadline (August 2026), meaning this is not speculative demand; (4) capital efficiency — £0 raised to date with a production product built, demonstrating exceptional capital-to-output ratio.
