# Security Policy

Anvil takes the security of its tooling and the code it protects seriously. This
policy explains how to report a vulnerability, which versions receive fixes, and
how quickly we respond.

## Reporting a vulnerability

**Please do not open a public issue for a security vulnerability.** Public
disclosure before a fix is available puts users at risk.

Report privately through one of:

- **GitHub private vulnerability reporting** (preferred) — use the **Report a
  vulnerability** button under this repository's **Security** tab. This opens a
  private advisory visible only to the maintainers and you.
- If you cannot use GitHub private reporting, contact the maintainers through
  the organisation's published security contact and we will open a private
  advisory on your behalf.

Please include, where possible: the affected version or commit, a description of
the issue and its impact, and minimal steps (or a proof of concept) to reproduce
it. Do not include live credentials or third-party data in the report.

We acknowledge new reports within **2 business days** and keep you updated as
triage and remediation proceed (see the SLA below). We welcome coordinated
disclosure and will credit reporters who wish to be named once a fix has
shipped.

## Supported versions

Anvil is pre-1.0 and ships as dated beta releases. Security fixes target the
**latest released version on `main`**; there is no back-porting to older betas.
Run a current release to stay covered.

| Version         | Supported          |
| --------------- | ------------------ |
| Latest beta     | :white_check_mark: |
| Older beta tags | :x:                |

## Response SLA by severity

Severity follows the impact of the issue (CVSS-style judgement, not a strict
score). Times are targets, measured from acknowledgement.

| Severity | Acknowledge     | Fix target                                           |
| -------- | --------------- | ---------------------------------------------------- |
| Critical | 2 business days | Patch released as soon as practical; days, not weeks |
| High     | 2 business days | Patched in the next release; expedited if exploited  |
| Medium   | 5 business days | Scheduled into an upcoming release                   |
| Low      | 5 business days | Tracked and addressed as capacity allows             |

## What happens next

The internal triage-to-patch process — intake, severity assessment, fix,
release, and advisory publication — is documented in the
[vulnerability-response runbook](docs/runbooks/vulnerability-response.md). Once
a fix has shipped, we publish a GitHub Security Advisory for the disclosed
issue.

## Scope

This policy covers the Anvil codebase in this repository and its released
artefacts. Dependency vulnerabilities are continuously scanned (see the
[dependency-audit posture](docs/guides/dependency-audit-posture.md)); report a
vulnerability in a third-party dependency to that project as well, and to us if
it affects Anvil's shipped behaviour.
