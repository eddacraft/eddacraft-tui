# Anvil

**The governance layer for AI-assisted development.**

Ship at AI speed. Sleep at human peace.

---

## The Problem

AI coding tools generate code faster than ever. But speed without safety is
chaos:

- No validation before code hits production
- No audit trail of what AI generated vs what humans wrote
- No way to roll back when things go wrong
- No governance for enterprise compliance

**Anvil makes AI-generated code changes safe for production.**

---

## Core Capabilities

### Works With Your Existing Workflow

- **Format agnostic** — works with SpecKit, BMAD, markdown, or any planning
  format
- **No migration required** — adapters bring validation to your current tools
- **Automatic detection** — Anvil figures out your format, you just run it

### Comprehensive Quality Gates

- **Lint, test, coverage** — all in one pipeline
- **Secret scanning** — regex + entropy detection, even in git history
- **Dependency vulnerabilities** — CVE detection with fix suggestions
- **SAST** — Semgrep integration for OWASP Top 10 coverage

### Deterministic & Tamper-Proof

- **Hash-stable** — same input produces same output, always
- **Cryptographic verification** — detect any modifications to validated plans
- **Schema-validated** — Zod + JSON Schema with strong TypeScript typing

### Complete Audit Trail

- **Immutable evidence** — append-only bundles for every operation
- **Full provenance** — who, what, when, why for every change
- **Compliance-ready** — documentation that satisfies auditors

### Safe Execution

- **Automatic snapshots** — state captured before any change
- **First-class rollback** — undo with confidence
- **Dry-run mode** — preview changes before applying

---

## What Makes Anvil Different

### AI Tool Intercept

_No competitor does this._

Validate AI suggestions **before** you accept them, not after. Native
integration with Claude Code, Cursor, and GitHub Copilot via MCP.

### Architecture Gates

_Unique to Anvil._

- Circular dependency detection
- Layer boundary validation (clean architecture, hexagonal, onion)
- Anti-pattern detection (god classes, tight coupling)
- Custom architecture rules

Catch structural problems that linters miss.

### Visual Blast Radius

_See the impact before you commit._

Interactive HTML reports showing:

- Dependency graph of affected files
- Impact score calculation
- Risk highlighting
- Side-by-side diffs with syntax highlighting

### Actionable Feedback

_Don't just fail — fix._

- Coverage gap analysis with generated test stubs
- Dependency upgrade suggestions with commands
- Security fix recommendations
- One-command fixes where possible

---

## Developer Experience

### Git Integration

```bash
anvil hooks install  # Add pre-commit and pre-push hooks
```

Validation runs automatically. No extra steps.

### Dev Mode

```bash
anvil gate --skip=coverage  # Skip gates during iteration
anvil gate --profile=dev    # Use relaxed dev profile
```

Flexible during development. Strict in CI.

### Watch Mode

```bash
anvil watch  # Continuous validation as you code
```

Real-time feedback without switching context.

### IDE Integration

- **VS Code extension** — inline validation, one-click gates, problem panel
- **JetBrains plugin** — same experience in IntelliJ/WebStorm

### CI/CD Native

- **GitHub Action** — gate in your pipeline, block bad merges
- **PR comments** — inline feedback on validation issues
- **Status checks** — pass/fail visible in PR

---

## CLI

```bash
anvil validate <plan>              # Validate any format
anvil gate <plan>                  # Run quality gates
anvil export <plan> --to <format>  # Convert between formats
anvil apply <plan>                 # Execute with snapshots
anvil rollback <plan-id>           # Undo applied changes
anvil preview --html               # Visual diff report
```

Works with Cursor, GitHub Copilot, Claude Code, and any AI coding tool.

---

## Policy as Code

Define your standards in Rego:

```rego
# Enforce minimum coverage
deny[msg] {
    input.coverage < 80
    msg := sprintf("Coverage %v%% is below minimum 80%%", [input.coverage])
}

# Block changes to critical paths
deny[msg] {
    some change in input.changes
    startswith(change.path, "src/auth/")
    not input.approved_by_security
    msg := "Changes to auth require security approval"
}
```

Policies are versioned, tested, and enforced automatically.

---

## Who Is Anvil For?

### Developers

Using AI coding tools and need production-safe workflows. Ship faster without
sacrificing quality.

### Platform Teams

Standardising AI development practices across the organisation. One tool, one
pipeline, consistent governance.

### Enterprises

Requiring compliance, audit trails, and governance. Immutable evidence bundles
satisfy auditors. Policy-as-code enforces standards.

---

## Design Principles

1. **Developer-first** — meet developers where they are (IDE, terminal, CI)
2. **Trust by design** — provenance, validation, rollback-ready
3. **Interoperability** — work with existing formats, don't force migration
4. **Speed** — sub-second validation or developers disable it
5. **Safety** — rollback is non-negotiable

---

## Current Status

| Component          | Status      |
| ------------------ | ----------- |
| APS Core           | ✅ Complete |
| Format Adapters    | ✅ Complete |
| CLI                | ✅ Complete |
| Gate v1            | ✅ Complete |
| Git Hooks          | Coming soon |
| GitHub Action      | Coming soon |
| VS Code Extension  | Coming soon |
| Architecture Gates | Coming soon |

---

## Get Started

```bash
# Install
npm install -g @anvil/cli

# Initialise in your project
anvil init

# Validate a plan
anvil validate docs/plan.md

# Run quality gates
anvil gate docs/plan.md
```

---

_Ship at AI speed. Sleep at human peace._
