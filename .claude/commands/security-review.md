---
description: Comprehensive security review of the codebase
---

# Security Review Workflow

In-depth security audit covering authentication, authorization, data protection,
and vulnerabilities:

1. **Threat Discovery** - Identify attack surfaces and security risks
2. **Vulnerability Scanning** - Check for common security issues (OWASP Top 10)
3. **Dependency Audit** - Scan for known CVEs in dependencies
4. **PII & Data Protection** - Verify sensitive data handling
5. **Security Report** - Detailed findings with severity levels and remediation
   steps

## Agent Sequence

- **security-auditor**: Performs comprehensive security review using
  `.claude/docs-templates/Security-Audit.md`

## Usage

```
/security-review
```

Run this command to perform a thorough security audit of your codebase.

## What Gets Checked

### Authentication & Authorization

- Authentication mechanisms on all endpoints
- Role-based access control (RBAC)
- Session management and token handling
- Password policies and storage

### Input Validation & Injection

- SQL/NoSQL injection vulnerabilities
- Cross-Site Scripting (XSS) risks
- Command injection risks
- Path traversal vulnerabilities

### Secrets Management

- Hardcoded credentials or API keys
- Environment variable usage
- Secret rotation practices
- Credential exposure in logs

### Data Protection

- PII identification and handling
- Encryption at rest and in transit
- Secure data transmission (HTTPS)
- Data retention and deletion

### Dependencies & Supply Chain

- Known CVEs in npm/pip/go packages
- Outdated dependencies
- Dependency license compliance
- Supply chain attack vectors

### Security Headers & Configuration

- CORS configuration
- CSP (Content Security Policy)
- HSTS, X-Frame-Options, etc.
- Security misconfigurations

### Logging & Monitoring

- Security event logging
- PII in logs and telemetry
- Audit trail completeness
- Error message information disclosure

## Output Artifacts

- **Security Audit Report** with risk levels:
  - 🔴 CRITICAL - Block deployment
  - 🟡 HIGH - Fix this sprint
  - 🟢 MEDIUM - Fix soon
  - 🔵 LOW - Consider fixing

- **OWASP Top 10 Checklist** - Coverage against common vulnerabilities
- **PII Inventory** - All identified personally identifiable information
- **Remediation Plan** - Prioritized action items with code examples
- **Compliance Notes** - GDPR, SOC 2, or other relevant compliance issues

## When to Run

- Before major releases
- After adding authentication/authorization features
- When integrating third-party services
- After dependency updates
- During security incidents or audits
- Quarterly security reviews

## Follow-up Actions

Based on findings, the security-auditor will create handoffs for:

- **coder**: To fix critical and high-priority vulnerabilities
- **reviewer**: To verify security fixes
- **docs-writer**: To document security practices
- **architect**: For architectural security improvements
