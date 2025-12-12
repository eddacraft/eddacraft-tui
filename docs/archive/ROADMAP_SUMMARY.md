# Anvil Roadmap Summary - Security + Architecture Focus

**Last Updated**: November 10, 2025 **Status**: Updated with security gates,
architecture validation, and visual preview features

---

## 🎯 New Strategic Positioning

### Primary Message

**"The only AI coding platform that validates security AND architecture"**

### Supporting Messages

- "AI writes code fast. Anvil makes it production-ready."
- "Catch vulnerabilities, enforce patterns, ship safely"
- "Stop manually reviewing AI-generated code"
- "Ship at AI speed, sleep at human peace"

---

## 📅 Updated MVP Roadmap (Weeks 9-18)

### ✅ Completed (Weeks 1-8)

- **Phase 1**: Foundations (repository, CI/CD, quality gates) ✅
- **Phase 2**: APS Core (schema, validation, hashing) ✅
- **Phase 2.5**: Adapters (SpecKit, BMAD, Generic) ✅
- **Phase 3**: CLI Integration (validate, gate, export) ✅
- **Phase 4**: Gate v1 (lint, test, coverage, secrets) ✅
- **Phase 4.5**: Comprehensive Testing (232 adapter tests) ✅

---

### 🔒 Week 9-10: Security Gates v1 - EARLY FLASHINESS

**Goal**: Establish "secure AI development" credibility with quick wins

#### Features

1. **Dependency Vulnerability Scanning** (2-3 days)
   - npm audit / pnpm audit integration
   - Snyk API integration (optional)
   - CVE detection with severity scoring
   - Actionable fix suggestions
   - **Demo Impact**: HIGH - "Anvil caught a critical CVE in lodash!"

2. **Enhanced Secret Scanning** (2 days)
   - Shannon entropy-based detection
   - Git history scanning (not just current state)
   - Combine with existing regex patterns
   - Reduce false positives with allowlists
   - **Demo Impact**: HIGH - "Catches secrets regex misses"

3. **Plan History & Comparison** (1-2 days)
   - Track plan evolution (created, validated, applied)
   - `anvil history` - list all plans
   - `anvil diff <plan-1> <plan-2>` - compare plans
   - **Demo Impact**: MEDIUM - governance story

#### Why This Week Matters

- **Quick wins**: Dependency scanning is 2-3 days for huge value
- **Visible impact**: Easy to demo "Anvil caught X vulnerabilities"
- **Differentiator**: Security-first AI development positioning

---

### 🎨 Week 11-12: Architecture Gates + Visual Preview - WOW FACTOR

**Goal**: Unique differentiation + stunning visuals for demos

#### Features

1. **Architecture & Best Practices Gate** (5 days)
   - Dependency analysis (circular deps, direction validation)
   - Layer boundary validation (clean architecture enforcement)
   - Anti-pattern detection (god classes, tight coupling)
   - Uses: dependency-cruiser, ts-morph AST analysis
   - **Demo Impact**: VERY HIGH - unique differentiator!

2. **Visual Diff Preview** (4 days)
   - Interactive HTML diff report
   - Side-by-side file comparisons with syntax highlighting
   - File tree visualization
   - Blast radius analysis (dependency impact)
   - `anvil dry-run --output html` auto-opens browser
   - **Demo Impact**: VERY HIGH - instant wow!

3. **Inline GitHub PR Comments** (2-3 days)
   - Comment directly on changed lines
   - Validation issues inline (like code review)
   - Update comments as code changes
   - **Demo Impact**: HIGH - better developer experience

#### Why This Week Matters

- **Unique**: Architecture validation doesn't exist elsewhere
- **Visual**: Beautiful HTML diff creates immediate impact
- **Memorable**: People remember what they see

---

### 🛡️ Week 13-14: SAST Integration - COMPREHENSIVE SECURITY

**Goal**: Complete enterprise-grade security story

#### Features

1. **SAST Scanning (Semgrep)** (5 days)
   - Semgrep integration with default rulesets
   - OWASP Top 10 coverage
   - CWE mapping and severity scoring
   - Custom rule support
   - Fix suggestions with documentation links
   - **Demo Impact**: VERY HIGH - enterprise requirement

2. **Infrastructure-as-Code Security** (2-3 days)
   - Hadolint for Dockerfiles
   - Checkov for Terraform/K8s/CloudFormation
   - Misconfiguration detection
   - **Demo Impact**: MEDIUM-HIGH

3. **License Compliance Scanning** (1-2 days)
   - Detect license conflicts (GPL in proprietary code)
   - SBOM generation
   - Configurable allowlist/blocklist
   - **Demo Impact**: MEDIUM - compliance story

#### Why This Week Matters

- **Enterprise sales**: SAST is table stakes for security
- **Compliance**: SOC2, ISO 27001 requirements
- **Complete story**: Dependency + Secrets + SAST = comprehensive

---

### ⚙️ Week 15-16: Policy Engine + Apply

**Goal**: Safe execution of validated plans

#### Features

1. **OPA/Rego Integration** (3-4 days)
   - OPA binary integration
   - Sample policies (coverage, security, architecture, change scope)
   - Policy CLI commands
   - **Demo Impact**: MEDIUM - governance

2. **Apply Implementation** (3-4 days)
   - Transactional application of changes
   - Snapshot creation before apply
   - Audit trail generation
   - Safety guards (gate pass required, approval flag)
   - **Demo Impact**: HIGH - core value prop

#### Why This Week Matters

- **Safety**: Rollback capability is non-negotiable
- **Trust**: Audit trail enables compliance
- **Complete**: Validate → Apply workflow

---

### 🚀 Week 17-18: Rollback & GitHub Action

**Goal**: Complete end-to-end workflow

#### Features

1. **Rollback Implementation** (3-4 days)
   - Snapshot restoration
   - Change reversal with integrity verification
   - `anvil rollback <plan-id>` command

2. **GitHub Action** (3-4 days)
   - Full GitHub Action implementation
   - PR validation automation
   - Status checks and merge blocking

#### Why This Week Matters

- **Production ready**: Rollback completes safety story
- **Distribution**: GitHub Action enables adoption

---

## 🎪 Demo Progression Strategy

### Week 9-10 Demo: "Security Catches"

**Script**:

1. Show AI-generated code with dependency vulnerability
2. Run `anvil gate plan.md`
3. **Wow moment**: "Critical CVE in lodash detected!"
4. Show fix suggestion and CVE link
5. Show secret caught in git history

**Impact**: Establishes security credibility

---

### Week 11-12 Demo: "Visual + Architecture"

**Script**:

1. Show AI-generated code with architectural issues
2. Run `anvil dry-run --output html plan.md`
3. **Wow moment**: Beautiful HTML diff opens in browser
4. Show blast radius visualization
5. Show architecture gate catching circular dependency
6. Show inline PR comments

**Impact**: Creates memorable visual experience + unique differentiator

---

### Week 13-14 Demo: "Comprehensive Security"

**Script**:

1. Show AI-generated code with SQL injection
2. Run `anvil gate plan.md`
3. **Wow moment**: SAST catches OWASP Top 10 issues
4. Show CWE mapping and fix suggestions
5. Show IaC security catching exposed port
6. Show SBOM generation

**Impact**: Completes enterprise security story

---

## 💼 Value Propositions by Audience

### For Developers

- "Stop wasting time manually reviewing AI code"
- "Catch vulnerabilities before they reach production"
- "Beautiful visual previews show exactly what will change"
- "Architecture gates keep your codebase clean"

### For Engineering Managers

- "Standardize AI development across teams"
- "Enforce architectural patterns automatically"
- "Complete audit trail for compliance"
- "Reduce code review time by 50%+"

### For Security Teams

- "SAST scanning catches OWASP Top 10"
- "Dependency scanning prevents supply chain attacks"
- "Secret scanning with git history coverage"
- "IaC security for infrastructure code"

### For Compliance/Legal

- "Immutable audit trail for all changes"
- "License compliance scanning (SBOM)"
- "Policy enforcement with OPA/Rego"
- "Evidence bundles for SOC2/ISO compliance"

---

## 📊 Success Metrics

### Technical Metrics

- **Security**: % of vulnerabilities caught before commit
- **Architecture**: % of anti-patterns prevented
- **Coverage**: Test coverage maintained above threshold
- **Speed**: Gate execution time < 2 minutes

### Business Metrics

- **Adoption**: 15-20 pilot teams by Week 18
- **Engagement**: Plans validated per team per week
- **Quality**: Gate pass rate > 80%
- **Satisfaction**: NPS > 50

---

## 🎯 Competitive Differentiation

### What Competitors Do

- **GitHub Copilot**: AI coding, no validation
- **Cursor**: AI IDE, no governance
- **Snyk/SonarQube**: Static analysis, not AI-specific
- **Qodo**: AI testing, not governance

### What Anvil Does (Unique)

✅ **Security + Architecture validation** - No one else does both ✅ **Visual
diff preview** - Beautiful, shareable HTML reports ✅ **Architecture gates** -
Enforce patterns and layer boundaries ✅ **Format agnostic** - Works with
SpecKit, BMAD, any markdown ✅ **Complete audit trail** - Immutable evidence
bundles ✅ **Safe execution** - Apply with snapshots, rollback capability

---

## 📈 Growth Strategy

### Week 9-14: Build + Demo

- Implement security + architecture features
- Create compelling demos for each phase
- Record demo videos
- Publish blog posts on security/architecture validation

### Week 15-18: Pilot Expansion

- Onboard 10-15 pilot teams
- Gather feedback and testimonials
- Iterate based on real usage
- Prepare case studies

### Week 19+: Launch + Scale

- Public launch with security + architecture story
- GitHub Action published
- Developer relations campaign
- Enterprise sales outreach

---

## 🏆 Why This Roadmap Wins

1. **Early Flashiness**: Security catches in Week 9-10 create immediate
   credibility
2. **Unique Differentiation**: Architecture gates (Week 11-12) have no
   competition
3. **Visual Impact**: HTML diff preview creates memorable demos
4. **Complete Story**: Security + Architecture + Execution = production-ready
5. **Enterprise Ready**: SAST, compliance, audit trail = enterprise sales
6. **Fast MVP**: 8 additional weeks to comprehensive platform

---

**Next Steps**:

1. Review and approve roadmap
2. Begin Week 9 implementation (dependency scanning)
3. Prepare demo scripts for each phase
4. Set up pilot customer recruitment

---

**Document Version**: 1.0 **Authors**: Anvil Planning Team **Next Review**:
Weekly during implementation
