---
name: security-analysis
description:
  Security vulnerability assessment, threat modeling, secure coding practices
---

# Security Analysis Skill

## Overview

Comprehensive security assessment methodology for identifying vulnerabilities
and ensuring secure code.

## When to Apply

- Security audits
- New feature review
- Dependency updates
- Compliance assessments
- Incident response

## Threat Modeling

### STRIDE Framework

| Threat                     | Description                     | Examples                           |
| -------------------------- | ------------------------------- | ---------------------------------- |
| **S**poofing               | Impersonating something/someone | Fake login, session hijacking      |
| **T**ampering              | Modifying data/code             | SQL injection, file manipulation   |
| **R**epudiation            | Denying actions                 | Missing audit logs                 |
| **I**nformation Disclosure | Exposing data                   | Data leaks, verbose errors         |
| **D**enial of Service      | Making unavailable              | Resource exhaustion, crashes       |
| **E**levation of Privilege | Gaining access                  | Admin bypass, privilege escalation |

### Analysis Process

```
1. Identify assets (data, systems, users)
2. Create data flow diagrams
3. Identify entry points
4. Apply STRIDE to each component
5. Prioritize threats
6. Define mitigations
```

## Vulnerability Categories

### OWASP Top 10 (2021)

#### A01: Broken Access Control

```
Checks:
- Authorization on every request
- Deny by default
- Rate limiting
- Directory listing disabled
- JWT validation
```

#### A02: Cryptographic Failures

```
Checks:
- TLS for data in transit
- Strong encryption for data at rest
- No deprecated algorithms (MD5, SHA1)
- Proper key management
- No hardcoded secrets
```

#### A03: Injection

```
Types:
- SQL injection
- NoSQL injection
- Command injection
- LDAP injection
- XPath injection

Prevention:
- Parameterized queries
- Input validation
- Output encoding
- Least privilege
```

#### A04: Insecure Design

```
Checks:
- Threat modeling done
- Security requirements defined
- Secure design patterns
- Defense in depth
```

#### A05: Security Misconfiguration

```
Checks:
- Unnecessary features disabled
- Default credentials changed
- Error handling doesn't leak info
- Security headers present
- Latest patches applied
```

#### A06: Vulnerable Components

```
Checks:
- Dependencies up to date
- No known CVEs
- Components from trusted sources
- Unused dependencies removed
```

#### A07: Authentication Failures

```
Checks:
- Strong password policy
- Multi-factor available
- Brute force protection
- Secure session management
- Credential storage (bcrypt, argon2)
```

#### A08: Software and Data Integrity Failures

```
Checks:
- CI/CD pipeline secured
- Dependencies verified
- Code signing
- Integrity checks
```

#### A09: Security Logging Failures

```
Checks:
- Security events logged
- Log integrity protected
- Alerts configured
- Logs don't contain sensitive data
```

#### A10: Server-Side Request Forgery

```
Checks:
- URL validation
- Allowlist destinations
- Response validation
- Network segmentation
```

## Code Patterns

### Secure Patterns

```typescript
// Input validation
function processInput(input: string): Result {
  const sanitized = validator.escape(input);
  const validated = schema.validate(sanitized);
  if (!validated.success) {
    throw new ValidationError(validated.error);
  }
  return process(validated.data);
}

// Parameterized queries
const result = await db.query('SELECT * FROM users WHERE id = $1', [userId]);

// Proper error handling
try {
  await riskyOperation();
} catch (error) {
  logger.error('Operation failed', { errorId: uuid() });
  throw new PublicError('Operation failed');
}
```

### Insecure Patterns (Avoid)

```typescript
// SQL injection vulnerability
const result = await db.query(
  `SELECT * FROM users WHERE id = ${userId}` // BAD!
);

// Command injection
exec(`ls ${userInput}`); // BAD!

// Hardcoded secrets
const apiKey = "sk-1234567890"; // BAD!

// Verbose errors
catch (error) {
  res.json({ error: error.stack }); // BAD!
}
```

## Security Report Template

```markdown
## Security Assessment Report

### Scope

What was assessed

### Risk Summary

| Severity | Count |
| -------- | ----- |
| Critical | X     |
| High     | X     |
| Medium   | X     |
| Low      | X     |

### Findings

#### [CRITICAL] Finding Title

- **Location**: file:line
- **Type**: CWE-XXX
- **Description**: What was found
- **Impact**: What could happen
- **Remediation**: How to fix
- **References**: CVE, OWASP

### Recommendations

Prioritized action items

### Positive Observations

Security measures done well
```
