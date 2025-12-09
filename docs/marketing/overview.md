# Anvil - Key Features

## Core Value Proposition

**Make AI-generated code changes safe for production** - Ship at AI speed, sleep
at human peace.

---

## Key Features

### Format Interoperability

- Works with your existing planning formats (SpecKit, BMAD, custom)
- No forced migration - adapters bring validation to your current workflow
- Automatic format detection and round-trip conversion

### Comprehensive Quality Gates

- Lint, test, coverage, and secret scanning in one pipeline
- Policy-as-code enforcement (OPA/Rego)
- Security validation and dependency vulnerability checks

### Deterministic & Tamper-Proof

- Hash-stable plan specification (APS) - same input, same output, always
- Cryptographic verification detects any modifications
- Schema-validated with strong typing

### Complete Audit Trail

- Immutable evidence bundles for every operation
- Full provenance tracking (who, what, when, why)
- Compliance-ready documentation

### Safe Execution with Rollback

- Automatic snapshots before any change
- First-class rollback capability
- Dry-run mode to preview changes

### Simple CLI Integration

- `anvil validate` - validate plans in any format
- `anvil gate` - run quality gates
- `anvil export` - convert between formats
- Works with Cursor, GitHub Copilot, Claude Code, and any AI coding tool

---

## Target Users

- **Developers** using AI coding tools who need production-safe workflows
- **Platform teams** standardising AI development practices
- **Enterprises** requiring governance, compliance, and audit trails
