# Dependency Audit Posture

| Type  | Authority     | Owner | Status | Freshness                                         |
| ----- | ------------- | ----- | ------ | ------------------------------------------------- |
| Guide | Authoritative | SEC   | Live   | First filed 2026-06-16 against `main` for SEC-001 |

| Upstream                                                                                                                                                                       | Downstream                                                                               |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| [`.github/workflows/security.yml`](../../.github/workflows/security.yml), [`deny.toml`](../../attribution/deny.toml), [`.github/dependabot.yml`](../../.github/dependabot.yml) | [`supply-chain-policy.md`](./supply-chain-policy.md), [`SECURITY.md`](../../SECURITY.md) |

## Overview

Anvil audits its dependencies for known vulnerabilities, disallowed licences,
and leaked secrets through automation that already ships on `main`. This guide
makes that as-built surface legible — what runs, where it gates, and how to
respond — so the posture is documented rather than tribal. It does **not**
change any threshold; amending the rules is a separate, deliberate act (see
[`supply-chain-policy.md`](./supply-chain-policy.md)).

The historic plan ("`pnpm audit` + `cargo audit` in CI") is satisfied, but not
by those literal tools: `pnpm audit` is not used because the npm v1 audit
endpoint returns HTTP 410, and `cargo audit`'s RUSTSEC advisory feed is consumed
via `cargo-deny` instead.

## The audit surface

Two workflows carry the posture. Both are path-gated per PR (they run when a
dependency-relevant path changes) and also run on a weekly schedule.

- **Vulnerability scan (JS + lockfiles) — `Dependency Audit`.** The
  `dependency-audit` job in
  [`.github/workflows/security.yml`](../../.github/workflows/security.yml) runs
  a Trivy filesystem scan over the lockfiles at `HIGH,CRITICAL` severity with
  `exit-code: 1`, so a high/critical advisory fails the gate.
- **Rust advisories + bans — `cargo-deny`.** The `deny` job in
  [`.github/workflows/rust.yml`](../../.github/workflows/rust.yml) runs
  `cargo-deny`, whose `[advisories]` section in
  [`deny.toml`](../../attribution/deny.toml) consumes the RUSTSEC advisory
  database. This is the "cargo audit" intent. The job is gated on
  `detect-rust-changes` (manifests, lockfile, toolchain, or the workflow
  itself).
- **Secret scan — `Secret Scan`.** The `secret-scan` job runs TruffleHog with
  `--only-verified`, so only credentials it can actively verify trip the gate
  (textbook/example keys do not).
- **Licence compliance — `License Compliance`.** The `license-check` job runs
  [`scripts/license-check.sh`](../../scripts/license-check.sh) over the JS
  dependency tree; the Rust allowlist is enforced by `cargo-deny`'s `[licenses]`
  section.
- **SAST — `SAST (Semgrep)`.** The `semgrep` job runs Semgrep and uploads SARIF
  to GitHub Security. It is static analysis of source, complementary to the
  dependency audit.

## Automated updates

[`.github/dependabot.yml`](../../.github/dependabot.yml) opens weekly update PRs
for the **npm**, **github-actions**, and **cargo** ecosystems. The `cargo` entry
closes the one real gap this guide's change fills — Rust dependency _updates_
were previously not automated, even though Rust advisory _gating_ already
shipped via `cargo-deny`. Advisory gating (cargo-deny / Trivy) and update
automation (Dependabot) are complementary: gating blocks a known-bad dependency
at CI; Dependabot proposes the bump that resolves it.

## Responding to a finding

- **A Trivy or cargo-deny advisory fails CI.** Prefer bumping the affected
  dependency over suppressing. If a fix is not yet available, time-box an
  `[advisories]` ignore entry in `deny.toml` with a tracking issue URL and an
  expected-fix date (see the in-file guidance), never a permanent bypass.
- **A licence is rejected.** The Rust allowlist is generated from
  `licences.toml`; edit that source (not `deny.toml`) per the
  [supply-chain policy](./supply-chain-policy.md).
- **TruffleHog flags a verified secret.** Treat it as a live exposure: rotate
  the credential ([secret-rotation runbook](../runbooks/secret-rotation.md)) and
  follow the
  [vulnerability-response runbook](../runbooks/vulnerability-response.md).

## Out of scope

SBOM / build attestation (owned by the supply-chain-attestation track), changes
to the Trivy or cargo-deny thresholds, and secret-detection _rule content_ (the
Rust scanner) are out of scope here.
