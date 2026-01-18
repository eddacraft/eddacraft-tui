# OPA Enhancement Vision: Making Policy-as-Code Delightful

**Status:** Vision Document **Priority:** High **Last Updated:** January 2026

## Executive Summary

This document outlines a comprehensive vision to make Anvil's OPA functionality
not just useful, but truly impressive—something that makes developers say "wow".
The key insight: **most developers don't want to learn Rego**. We need to meet
them where they are while providing an escape hatch to full power when needed.

### Core Principles

1. **Zero-Rego Required** — 90% of users should never write Rego
2. **Progressive Disclosure** — Simple things simple, complex things possible
3. **Instant Feedback** — Know immediately when policies pass/fail
4. **Self-Documenting** — Policies explain themselves when they fail
5. **Shareable** — Policies as a product, not just a feature

---

## Part 1: Custom Architecture Policies Made Easy

### The Problem

Users want to enforce architectural boundaries but:

- Learning Rego is a steep learning curve
- Architecture rules are often simple: "A cannot import B"
- Existing tools require configuration file gymnastics
- No visibility into what's actually being enforced

### The Solution: YAML-First Architecture Rules

#### Simple Rules (No Rego Required)

```yaml
# .anvil/architecture.yaml
schema_version: '1.0'
name: 'My Application'

# Start from a template (optional)
template: hexagonal

# Define your layers
layers:
  domain:
    paths:
      - 'src/domain/**'
      - 'src/entities/**'
    description: 'Core business logic - no external dependencies'

  application:
    paths: ['src/application/**', 'src/use-cases/**']
    description: 'Application services orchestrating domain logic'

  infrastructure:
    paths: ['src/infrastructure/**', 'src/adapters/**']
    description: 'External integrations (DB, APIs, messaging)'

  presentation:
    paths: ['src/api/**', 'src/web/**', 'src/cli/**']
    description: 'User-facing interfaces'

# Simple dependency rules (the magic!)
rules:
  # Domain is pure - no dependencies
  - layer: domain
    can_import: []
    cannot_import: [infrastructure, presentation]

  # Application depends only on domain
  - layer: application
    can_import: [domain]

  # Infrastructure can access domain and application
  - layer: infrastructure
    can_import: [domain, application]
    cannot_import: [presentation]

  # Presentation accesses application only
  - layer: presentation
    can_import: [application]
    cannot_import: [domain, infrastructure] # Must go through application
```

#### Module Boundaries

```yaml
# Define bounded contexts / modules
modules:
  ordering:
    paths: ['src/modules/ordering/**']
    public_api: ['src/modules/ordering/index.ts']
    description: 'Order management bounded context'

  inventory:
    paths: ['src/modules/inventory/**']
    public_api: ['src/modules/inventory/index.ts']

  shipping:
    paths: ['src/modules/shipping/**']
    public_api: ['src/modules/shipping/index.ts']

# Module interaction rules
module_rules:
  # Modules can only import each other's public APIs
  - enforce: public_api_only
    severity: error

  # Or be explicit about allowed interactions
  - from: ordering
    can_import: [inventory]
    via: public_api # Must use public API, not internals

  - from: shipping
    can_import: [ordering, inventory]
    via: public_api
```

#### File-Level Rules

```yaml
# Specific file patterns
file_rules:
  # Test files cannot import from src directly (use fixtures)
  - pattern: '**/*.test.ts'
    cannot_import: ['src/infrastructure/**']
    message: 'Tests should not depend on infrastructure directly'

  # Components cannot import server-only code
  - pattern: 'src/components/**'
    cannot_import: ['src/server/**', '**/api/**']
    message: 'Client components cannot import server code'

  # Utils should be pure
  - pattern: 'src/utils/**'
    cannot_import: ['src/**'] # Only external deps
    can_import: ['src/types/**'] # Except types
```

#### Import Restrictions

```yaml
# Package-level restrictions
import_rules:
  # Ban certain packages entirely
  - ban: ['lodash', 'moment']
    message: 'Use native methods or date-fns instead'
    severity: error

  # Restrict packages to certain layers
  - package: 'prisma'
    only_in: [infrastructure]
    message: 'Database access must be in infrastructure layer'

  # Enforce package boundaries
  - package: '@internal/*'
    not_in: [presentation]
    message: 'Internal packages not allowed in presentation layer'
```

### Interactive Setup Wizard

```bash
$ anvil architecture init

╭─────────────────────────────────────────────────────────────────╮
│                    Architecture Setup Wizard                     │
╰─────────────────────────────────────────────────────────────────╯

? What architecture pattern does your project follow?

  ❯ Layered (Presentation → Business → Data)
    Hexagonal (Ports & Adapters)
    Clean Architecture (Entities → Use Cases → Adapters)
    Domain-Driven Design (Bounded Contexts)
    Monorepo (Multiple packages)
    Custom (Define from scratch)

? Analysing your codebase...

  Found 4 potential layers:
    ✓ src/api/         → Presentation (23 files)
    ✓ src/services/    → Application (45 files)
    ✓ src/domain/      → Domain (12 files)
    ✓ src/database/    → Infrastructure (8 files)

? Does this look correct? (Y/n)

? Checking existing imports...

  ⚠ Found 3 potential violations:
    • src/api/handler.ts imports src/database/client.ts
    • src/domain/order.ts imports src/services/email.ts
    • src/services/user.ts has circular dependency

? How should we handle existing violations?

  ❯ Add to baseline (fix later)
    Show me each one
    Fail on all (strict mode)

✓ Created .anvil/architecture.yaml
✓ Generated dependency rules
✓ Added 3 violations to baseline

Next steps:
  1. Review .anvil/architecture.yaml
  2. Run: anvil architecture check
  3. Fix violations or adjust rules
```

### Visual Architecture Map

```bash
$ anvil architecture visualise

╭─────────────────────────────────────────────────────────────────╮
│                      Architecture Map                            │
╰─────────────────────────────────────────────────────────────────╯

                    ┌─────────────────┐
                    │   Presentation  │
                    │   (23 files)    │
                    └────────┬────────┘
                             │ ✓
                    ┌────────▼────────┐
                    │   Application   │
                    │   (45 files)    │
                    └────────┬────────┘
                             │ ✓
        ┌────────────────────┼────────────────────┐
        │                    │                    │
┌───────▼───────┐   ┌────────▼────────┐   ┌──────▼──────┐
│    Domain     │   │ Infrastructure  │   │   Shared    │
│  (12 files)   │   │   (8 files)     │   │  (5 files)  │
└───────────────┘   └─────────────────┘   └─────────────┘

Legend:
  ✓ = Allowed dependency
  ✗ = Violation (3 found)
  ⚠ = Warning (1 found)

Violations:
  1. presentation → infrastructure (2 imports)
     └─ src/api/handler.ts:15 → src/database/client.ts
     └─ src/api/health.ts:8 → src/database/pool.ts

  2. domain → application (1 import)
     └─ src/domain/order.ts:23 → src/services/email.ts
```

---

## Part 2: Rich Policy Library

### Pre-Built Policies (Install with One Command)

```bash
# Browse available policies
$ anvil policy browse

╭─────────────────────────────────────────────────────────────────╮
│                      Policy Library                              │
╰─────────────────────────────────────────────────────────────────╯

Security (8 policies)
  ├─ security-review      Require review for sensitive files
  ├─ no-secrets           Block hardcoded secrets/credentials
  ├─ dependency-audit     Check for vulnerable dependencies
  ├─ auth-changes         Extra scrutiny for auth code
  ├─ crypto-review        Review cryptographic operations
  ├─ input-validation     Ensure user input is validated
  ├─ sql-injection        Detect SQL injection patterns
  └─ xss-prevention       Cross-site scripting checks

Quality (6 policies)
  ├─ coverage-minimum     Enforce test coverage thresholds
  ├─ complexity-limit     Cyclomatic complexity bounds
  ├─ file-length          Maximum file length
  ├─ function-length      Maximum function length
  ├─ no-any-types         Ban TypeScript 'any' usage
  └─ documentation        Require JSDoc for public APIs

Scope (4 policies)
  ├─ change-limit         Limit files per PR
  ├─ directory-focus      Limit directories per PR
  ├─ single-concern       One feature per PR
  └─ blast-radius         Risk assessment for changes

Compliance (5 policies)
  ├─ license-check        OSS license compatibility
  ├─ gdpr-pii             PII handling requirements
  ├─ audit-logging        Ensure audit trails
  ├─ data-retention       Data lifecycle policies
  └─ accessibility        a11y requirements

API (4 policies)
  ├─ breaking-changes     Detect API breaking changes
  ├─ versioning           API version requirements
  ├─ deprecation          Deprecation notice requirements
  └─ documentation        OpenAPI/Swagger requirements

# Install a policy
$ anvil policy install security-review

✓ Installed security-review to .anvil/policies/
✓ Created security-review_test.rego with example tests

Configuration options (add to .anvilrc):
  {
    "policies": {
      "security-review": {
        "sensitive_paths": ["src/auth/**", "src/security/**"],
        "require_approval_from": ["@security-team"]
      }
    }
  }

# Install multiple policies
$ anvil policy install security-review no-secrets coverage-minimum

# Install a category
$ anvil policy install --category security
```

### Policy Configuration (No Rego!)

```yaml
# .anvil/policies.yaml - Configure policies without Rego

policies:
  coverage-minimum:
    enabled: true
    config:
      threshold: 80
      exclude_patterns:
        - '**/*.generated.ts'
        - '**/migrations/**'
      per_directory:
        'src/critical/**': 95
        'src/utils/**': 70

  change-limit:
    enabled: true
    config:
      max_files: 25
      max_directories: 5
      exclude:
        - '*.lock'
        - '*.snap'
      warn_at: 15 # Warn before blocking

  security-review:
    enabled: true
    config:
      sensitive_paths:
        - 'src/auth/**'
        - 'src/security/**'
        - '**/credentials*'
        - '**/*.env*'
      require_tags: ['security-reviewed']
      exempt_authors: ['security-bot']

  license-check:
    enabled: true
    config:
      allowed_licenses:
        - MIT
        - Apache-2.0
        - BSD-3-Clause
        - ISC
      banned_licenses:
        - GPL-3.0
        - AGPL-3.0
      warn_on_unknown: true

  no-secrets:
    enabled: true
    config:
      patterns:
        - 'AKIA[0-9A-Z]{16}' # AWS keys
        - 'sk_live_[a-zA-Z0-9]+' # Stripe keys
        - 'ghp_[a-zA-Z0-9]{36}' # GitHub tokens
      exclude_files:
        - '**/*.test.ts'
        - '**/fixtures/**'
```

---

## Part 3: Natural Language Policy Generation

### Describe Policies in Plain English

```bash
$ anvil policy create

? Describe your policy in plain English:
> Require at least two reviewers for any changes to the payments module,
> and one of them must be from the payments team

╭─────────────────────────────────────────────────────────────────╮
│                    Generated Policy                              │
╰─────────────────────────────────────────────────────────────────╯

Name: payments-review-requirement
Description: Enforces review requirements for payments module changes

Conditions:
  ✓ Changes touch: src/payments/** OR src/billing/**
  ✓ Minimum reviewers: 2
  ✓ Required team: @payments-team (at least 1)

YAML Configuration:
┌─────────────────────────────────────────────────────────────────┐
│ payments-review:                                                │
│   enabled: true                                                 │
│   triggers:                                                     │
│     paths: ['src/payments/**', 'src/billing/**']                │
│   requirements:                                                 │
│     min_reviewers: 2                                            │
│     required_teams:                                             │
│       - team: '@payments-team'                                  │
│         min: 1                                                  │
│   severity: error                                               │
│   message: 'Payment changes require 2 reviewers including       │
│            someone from @payments-team'                         │
└─────────────────────────────────────────────────────────────────┘

? Save this policy? (Y/n)
? Run a test to see what historical PRs would have been affected? (y/N)
```

### More Natural Language Examples

```bash
# Security policies
> "Block any PR that adds a new npm dependency without a security review"
> "Require sign-off from legal for any changes to terms of service"
> "Flag any file that might contain PII (names, emails, addresses)"

# Quality policies
> "Warn if test coverage drops below 80% for any file being changed"
> "Block PRs that add more than 500 lines without breaking into commits"
> "Require documentation updates when public API signatures change"

# Architecture policies
> "The frontend folder should never import from backend"
> "Database models can only be accessed through repository classes"
> "Shared utilities cannot have any project-specific imports"

# Workflow policies
> "Changes to CI/CD configuration require DevOps team approval"
> "Database migrations must be reviewed by a DBA"
> "Feature flags must be cleaned up within 30 days of full rollout"
```

---

## Part 4: Policy Debugging & Explanation

### When Policies Fail, Explain Why

```bash
$ anvil gate check

╭─────────────────────────────────────────────────────────────────╮
│                     Gate Check Results                           │
╰─────────────────────────────────────────────────────────────────╯

✗ FAILED: security-review (2 violations)

┌─────────────────────────────────────────────────────────────────┐
│ Violation 1 of 2                                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Rule: security-review.sensitive-file-changed                   │
│  File: src/auth/oauth-handler.ts                                │
│                                                                 │
│  WHY THIS FAILED:                                               │
│  ─────────────────                                              │
│  This file matches the sensitive path pattern 'src/auth/**'     │
│  which requires the 'security-reviewed' tag on the PR.          │
│                                                                 │
│  CURRENT STATE:                                                 │
│  ─────────────────                                              │
│  • File path: src/auth/oauth-handler.ts                         │
│  • Matches pattern: src/auth/**  ✓                              │
│  • PR tags: ['feature', 'authentication']                       │
│  • Required tag: 'security-reviewed'  ✗ MISSING                 │
│                                                                 │
│  HOW TO FIX:                                                    │
│  ─────────────────                                              │
│  Option 1: Add the 'security-reviewed' label to your PR         │
│            after getting security team sign-off                 │
│                                                                 │
│  Option 2: If this is a false positive, add to baseline:        │
│            anvil baseline add security-review src/auth/oauth-*  │
│                                                                 │
│  Option 3: Request an exception:                                │
│            anvil exception request security-review              │
│            --reason "Low risk change, only logging added"       │
│                                                                 │
│  DOCUMENTATION:                                                 │
│  https://docs.anvil.dev/policies/security-review                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Interactive Debugger

```bash
$ anvil policy debug security-review

╭─────────────────────────────────────────────────────────────────╮
│               Policy Debugger: security-review                   │
╰─────────────────────────────────────────────────────────────────╯

Evaluating against current changes...

Step 1/4: Loading policy inputs
  ✓ Plan: 5 files changed
  ✓ Context: branch=feature/oauth, author=alice
  ✓ Config: sensitive_paths=['src/auth/**', 'src/security/**']

Step 2/4: Finding sensitive files
  Checking: src/auth/oauth-handler.ts
    Pattern 'src/auth/**' → MATCH
  Checking: src/utils/helpers.ts
    Pattern 'src/auth/**' → no match
    Pattern 'src/security/**' → no match
  Result: 1 sensitive file found

Step 3/4: Checking requirements
  Required tag: 'security-reviewed'
  PR tags: ['feature', 'authentication']
  Result: MISSING required tag

Step 4/4: Generating violation
  → Violation created for src/auth/oauth-handler.ts

╭─────────────────────────────────────────────────────────────────╮
│ RESULT: 1 violation                                             │
│                                                                 │
│ The policy failed because:                                      │
│ • src/auth/oauth-handler.ts is in a sensitive path              │
│ • The PR is missing the 'security-reviewed' tag                 │
╰─────────────────────────────────────────────────────────────────╯

? Would you like to:
  ❯ See the raw OPA evaluation trace
    Modify the policy configuration
    Test with different inputs
    Exit debugger
```

---

## Part 5: Policy Impact Analysis

### Before Enforcing, Understand the Impact

```bash
$ anvil policy impact coverage-minimum --threshold 80

╭─────────────────────────────────────────────────────────────────╮
│            Policy Impact Analysis: coverage-minimum              │
╰─────────────────────────────────────────────────────────────────╯

Analysing last 100 merged PRs...

IMPACT SUMMARY:
  Would have blocked: 23 PRs (23%)
  Would have warned:  15 PRs (15%)
  Would have passed:  62 PRs (62%)

BLOCKED PRS BY AUTHOR:
  @alice    ████████░░░░  8 PRs
  @bob      █████░░░░░░░  5 PRs
  @charlie  ████░░░░░░░░  4 PRs
  @david    ███░░░░░░░░░  3 PRs
  others    ███░░░░░░░░░  3 PRs

BLOCKED PRS BY DIRECTORY:
  src/legacy/     ████████████  12 PRs (known tech debt)
  src/api/        █████░░░░░░░   5 PRs
  src/utils/      ████░░░░░░░░   4 PRs
  src/services/   ██░░░░░░░░░░   2 PRs

RECOMMENDATIONS:
  ⚠ src/legacy/ has consistently low coverage
    Consider: Exclude from policy or schedule cleanup sprint

  ⚠ 23% block rate may cause friction
    Consider: Start with threshold=70, increase gradually

  ✓ src/services/ is already compliant
    This policy will maintain quality here

? Would you like to:
  ❯ Simulate with different threshold
    See specific PRs that would be blocked
    Add exclusions for legacy directories
    Save analysis report
```

---

## Part 6: Real-Time Policy Watch Mode

### Instant Feedback as You Code

```bash
$ anvil policy watch

╭─────────────────────────────────────────────────────────────────╮
│                    Policy Watch Mode                             │
│                    Watching for changes...                       │
╰─────────────────────────────────────────────────────────────────╯

[14:32:15] File changed: src/api/handler.ts
           ✓ All policies pass

[14:32:45] File changed: src/auth/login.ts
           ⚠ security-review: Sensitive file changed
             └─ Will require 'security-reviewed' tag on PR

[14:33:12] File changed: src/database/client.ts
           ✗ architecture: Layer violation
             └─ src/api/handler.ts imports infrastructure layer
             └─ Fix: Import through application layer instead

[14:33:30] File changed: src/services/user.ts
           ✓ Architecture violation fixed!
           ✓ All policies pass

────────────────────────────────────────────────────────────────────
Status: 0 errors, 1 warning | Press 'q' to quit, 'r' to re-run all
```

---

## Part 7: Policy Exceptions & Waivers

### Formal Process for Policy Exceptions

```bash
$ anvil exception request security-review

╭─────────────────────────────────────────────────────────────────╮
│                   Request Policy Exception                       │
╰─────────────────────────────────────────────────────────────────╯

Policy: security-review
Violation: src/auth/oauth-handler.ts requires security review

? Reason for exception:
> Adding debug logging only - no security-relevant changes

? How long should this exception last?
  ❯ This PR only
    24 hours
    1 week
    Permanent (requires manager approval)

? Who should approve this exception?
  Auto-detected: @security-team (from policy config)

╭─────────────────────────────────────────────────────────────────╮
│                    Exception Request Created                     │
╰─────────────────────────────────────────────────────────────────╯

Request ID: EXC-2026-0142
Status: Pending approval
Approvers: @security-team

What happens next:
  1. Approvers will be notified via Slack/email
  2. They can approve at: https://anvil.dev/exceptions/EXC-2026-0142
  3. Once approved, this PR can proceed
  4. Exception logged for audit trail

You can check status with: anvil exception status EXC-2026-0142
```

### Audit Trail

```bash
$ anvil exception history

╭─────────────────────────────────────────────────────────────────╮
│                    Exception Audit Log                           │
╰─────────────────────────────────────────────────────────────────╯

Last 30 days:

ID              Policy           Requestor  Approver    Status    Expires
─────────────────────────────────────────────────────────────────────────
EXC-2026-0142   security-review  @alice     @security   Pending   -
EXC-2026-0138   coverage-min     @bob       @tech-lead  Approved  2026-01-20
EXC-2026-0135   change-limit     @charlie   @manager    Approved  Expired
EXC-2026-0130   security-review  @david     -           Rejected  -
EXC-2026-0125   architecture     @eve       @architect  Approved  Permanent

Summary:
  Total requests: 12
  Approved: 8 (67%)
  Rejected: 2 (17%)
  Pending: 2 (17%)

Export for compliance: anvil exception export --format csv
```

---

## Part 8: Remote Policy Bundles

### Share Policies Across Organisation

```bash
$ anvil policy bundle list

╭─────────────────────────────────────────────────────────────────╮
│                    Policy Bundles                                │
╰─────────────────────────────────────────────────────────────────╯

Subscribed bundles:
  ┌────────────────────────────────────────────────────────────┐
  │ org-standards (from policies.acme.com)                     │
  │ Last sync: 2 hours ago | 12 policies | Signature: ✓        │
  │ Policies: security-*, compliance-*, review-*               │
  └────────────────────────────────────────────────────────────┘

  ┌────────────────────────────────────────────────────────────┐
  │ team-frontend (from policies.acme.com)                     │
  │ Last sync: 2 hours ago | 5 policies | Signature: ✓         │
  │ Policies: a11y-*, perf-*, bundle-size                      │
  └────────────────────────────────────────────────────────────┘

Local policies (in .anvil/policies/):
  • custom-review.rego
  • project-specific.rego

$ anvil policy bundle add https://policies.acme.com/security-bundle

✓ Fetched bundle metadata
✓ Verified signature (signed by: security@acme.com)
✓ Downloaded 8 policies
✓ Added to .anvilrc

Policies added:
  • no-secrets
  • dependency-audit
  • auth-changes
  • crypto-review
  • input-validation
  • sql-injection
  • xss-prevention
  • security-baseline
```

---

## Part 9: PR Integration

### Automatic PR Comments

```markdown
## Anvil Policy Check Results

### Summary

| Check        | Status    | Score   |
| ------------ | --------- | ------- |
| Architecture | ✓ Pass    | 100/100 |
| Security     | ✓ Pass    | 100/100 |
| Coverage     | ⚠ Warning | 85/100  |
| Scope        | ✓ Pass    | 100/100 |

### Coverage Warning

> Test coverage for `src/services/payment.ts` dropped from 92% to 78%.
>
> **Affected lines:** 45-67 (new payment validation logic)
>
> This is a warning, not a blocker. Consider adding tests for:
>
> - `validatePaymentMethod()` (lines 45-52)
> - `processRefund()` (lines 58-67)

### Architecture Diagram
```

src/api/handler.ts └─→ src/services/payment.ts ✓ └─→ src/domain/order.ts ✓

```

<details>
<summary>Full policy evaluation details</summary>

- security-review: PASS (no sensitive files changed)
- change-limit: PASS (5 files < 25 max)
- coverage-minimum: WARN (78% < 80% threshold)
- architecture: PASS (no layer violations)

</details>

---
*Powered by [Anvil](https://anvil.dev) | [Configure policies](.anvil/policies.yaml)*
```

---

## Part 10: Metrics & Gamification

### Policy Compliance Dashboard

```bash
$ anvil metrics

╭─────────────────────────────────────────────────────────────────╮
│                  Policy Compliance Dashboard                     │
│                       Last 30 Days                               │
╰─────────────────────────────────────────────────────────────────╯

OVERALL COMPLIANCE TREND
100% │                                    ●●●●●
 90% │               ●●●●●●●●●●●●●●●●●●●●●
 80% │●●●●●●●●●●●●●●●
 70% │
     └────────────────────────────────────────→ Time

POLICY HEALTH
  security-review    ████████████████████  100% (0 violations)
  architecture       █████████████████░░░   85% (3 violations)
  coverage-minimum   ████████████████░░░░   80% (5 violations)
  change-limit       ██████████████░░░░░░   70% (8 violations)

TOP CONTRIBUTORS (Compliance Score)
  🥇 @alice     98.5%  ████████████████████
  🥈 @bob       95.2%  ███████████████████░
  🥉 @charlie   92.1%  ██████████████████░░
     @david     88.4%  █████████████████░░░
     @eve       85.0%  █████████████████░░░

MOST COMMON VIOLATIONS
  1. coverage-minimum (45%)
     └─ Mostly in src/legacy/ - consider exclusion
  2. change-limit (30%)
     └─ Large refactoring PRs - consider splitting
  3. architecture (25%)
     └─ api → infrastructure imports

RECOMMENDATIONS
  • Schedule tech debt sprint for src/legacy/ coverage
  • Create architecture diagram for new team members
  • Consider raising coverage threshold to 85%
```

---

## Implementation Phases

### Phase 1: Foundation (Immediate)

- [ ] YAML-based architecture rules
- [ ] Enhanced policy library (20+ policies)
- [ ] Policy configuration without Rego
- [ ] Improved violation explanations

### Phase 2: Developer Experience

- [ ] Interactive setup wizard
- [ ] Policy debugger
- [ ] Watch mode
- [ ] Impact analysis

### Phase 3: Enterprise Features

- [ ] Natural language policy generation
- [ ] Remote bundles with signatures
- [ ] Exception/waiver system
- [ ] Audit trail

### Phase 4: Integration & Visibility

- [ ] PR comments (GitHub/GitLab)
- [ ] Metrics dashboard
- [ ] IDE integration (LSP)
- [ ] Slack/Teams notifications

---

## Success Metrics

1. **Adoption**: 80% of users never write Rego
2. **Time to Value**: < 5 minutes from install to first policy check
3. **Satisfaction**: Policy failures feel helpful, not frustrating
4. **Compliance**: Teams achieve 90%+ policy compliance within 30 days

---

## Related Documents

- [OPA Policy Engine](./opa-policy-engine.md) — Current implementation
- [Architecture Integration](../../plans/modules/opa-architecture-integration.aps.md)
  — Technical plan
- [Hybrid DC+OPA Decision](../../plans/decisions/006-hybrid-dc-opa.md) —
  Architecture decision
