# Anvil by EddaCraft -- Executive Summary

## 1. SITUATION OVERVIEW

AI coding assistants have reached mainstream adoption: 84% of developers use them, 90% of Fortune 100 companies have deployed GitHub Copilot, and 46% of production code is now AI-generated. Enterprise engineering organisations treat AI coding tools as baseline productivity infrastructure. The generation layer is solved. The governance layer does not exist.

## 2. KEY FINDINGS

**AI-generated code is measurably lower quality.** CodeRabbit's analysis of thousands of pull requests found 1.7x more defects, 1.4x more critical issues, and 1.57x more security findings in AI-generated code compared to human-written code. GitClear's study of 211 million changed lines documented a 4x increase in code duplication and a collapse in refactoring from 25% to under 10% of changed lines.

**Security vulnerabilities are compounding at scale.** Veracode tested 100+ large language models and found 45% of AI-generated code fails security tests. The best available model (Claude Opus 4.5 Thinking) produces secure code only 56% of the time without specific security prompting. Aikido Security's 2026 report attributes 1 in 5 breaches to AI-generated code.

**The governance gap is structural, not incidental.** Every existing tool -- static analysers, AI code reviewers, supply chain scanners -- operates after code is committed. No tool governs code at the point of generation. Meanwhile, fewer than half of developers review AI output before committing.

**Regulatory deadlines are creating mandatory spend.** The EU AI Act high-risk requirements become enforceable in August 2026 (5 months). Penalties reach 7% of global annual turnover. Gartner forecasts AI governance platform spend at USD 492 million in 2026, growing past USD 1 billion by 2030.

**The market window is open.** Anvil is the only tool that is both deterministic (policy-as-code, not AI reviewing AI) and pre-commit (file save, not PR time). This unique position requires fundamental re-architecture to replicate, not a feature addition.

## 3. BUSINESS IMPACT

The total addressable market across AI code tools, application security, and AI governance is USD 21.5 billion (2025). The serviceable market for AI code governance specifically is USD 1.5--2.0 billion (2026) and accelerating. Gartner's survey of 360 organisations found those with governance platforms are 3.4x more effective, while effective governance reduces regulatory expenses by 20%.

## 4. RECOMMENDATIONS

**(Critical)** Invest in Anvil's go-to-market ahead of the August 2026 EU AI Act deadline. Owner: founding team. Timeline: Q2 2026. Expected result: first enterprise customers secured before regulatory enforcement creates broad demand.

**(High)** Expand policy pack library for major compliance frameworks (SOC 2, HIPAA, EU AI Act). Owner: product team. Timeline: Q3 2026. Expected result: compliance-driven expansion revenue from existing developer-led adoptions.

**(High)** Build open-source community around the core governance engine. Owner: developer relations. Timeline: ongoing from Q2 2026. Expected result: bottom-up developer adoption that seeds enterprise accounts.

## 5. NEXT STEPS

1. Finalise funding round to accelerate engineering and go-to-market (30-day target)
2. Secure 3--5 design partners from regulated industries (financial services, healthcare, enterprise SaaS)
3. Publish authorship attribution benchmarks to establish technical credibility in the market
