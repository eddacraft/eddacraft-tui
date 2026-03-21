<!--
APS Module: Security
====================================
Ongoing security posture management. Replaces archived security modules.
See: plans/aps-rules.md
-->

# Security

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| SEC    | —     | Draft |

## Purpose

Maintain ongoing security posture beyond the initial 0.1.x hardening.
Covers dependency auditing, secret rotation, vulnerability response,
supply chain security, and security hardening as the project grows.

**Replaces:** `security-ci-pipeline` (archived), `security-review-backlog`
(archived). Both were 0.1.x scope; this module covers forward-looking
security concerns.

## In Scope

- **Dependency auditing:** Automated `pnpm audit`, `cargo audit`, GitHub
  Dependabot configuration, lockfile policies
- **Secret management:** Rotation strategy, Pulumi ESC integration,
  environment variable governance
- **Vulnerability response:** Incident process, patch SLA, disclosure policy
- **Supply chain security:** Lockfile policies, private registry evaluation,
  transitive dependency risk assessment
- **Security hardening:** HTTP security headers, CSP, rate limiting (feeds
  into API governance)
- **SBOM generation:** Software bill of materials for release artifacts

## Out of Scope

- Penetration testing (external)
- SOC 2 compliance preparation (covered by compliance-reporting)
- Secret detection in code (covered by secret.check.ts gate)

## Interfaces

**Depends on:**

- CI pipeline — automated security scanning
- dependency.check.ts — gate check for vulnerabilities
- Pulumi — secret management via ESC
- API governance — rate limiting, security headers

**Exposes:**

- Security policy and incident response process
- Dependency audit automation
- Secret rotation schedule

## Estimated Scope

- **Effort:** 1-2 weeks

## Tasks

- SEC-001: Dependency audit automation (pnpm + cargo audit in CI)
- SEC-002: Secret rotation strategy and documentation
- SEC-003: Vulnerability response process and SLA
- SEC-004: Supply chain security policy (lockfile, registry)
- SEC-005: HTTP security headers configuration
- SEC-006: SBOM generation for release artifacts
