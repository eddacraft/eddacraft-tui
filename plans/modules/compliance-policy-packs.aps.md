<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Compliance Policy Packs

| ID  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| CPACKS | —     | high     | Draft  |

**Last reviewed:** 2026-04-26

> **Policy-solution validation (2026-06-24):** CPACKS keeps Rego as the
> portable policy language, but the shipping Anvil runtime is
> `crates/anvil-policy-engine` over regorus (ADR-040/POLENG), not a production
> Go OPA dependency. Direct `opa test` commands in older work items are
> compatibility/syntax checks only; completion evidence for each pack must also
> include a Rust/regorus-backed `cargo test` path through `crates/anvil-policy`
> or `crates/anvil-policy-engine`. Keep CPACKS **Draft** until POLVAL defines
> the pack manifest/test contract and the lower work-item file paths are
> retargeted to Rust-owned pack locations.
>
> NOTE(post-rust): All task `Scope`/`Files` paths target the retired TS tree
> (`packages/anvil/runtime/src/gate/__fixtures__/policies/...`). The Rego
> policies themselves are language-agnostic and can be reused, but when
> CPACKS moves to Ready they must land under
> `crates/anvil-policy/policies/compliance/<pack>/` (or the equivalent
> bundled-resource location chosen by `crates/anvil-policy`), with
> `cargo test -p eddacraft-anvil-policy` integration tests replacing the
> `nx test runtime` validations. Dependencies POLVAL-001..005 reference the
> archived `policy-pack-validation` module — re-confirm those contracts
> against the current `crates/anvil-policy::bundle` / `loader` modules.
> AGOV-001/006/007 dependencies still target the retired TS tree (see
> `agent-governance-patterns.aps.md`).

## Purpose

Ship production-ready, installable compliance policy packs for the regulatory
and security frameworks most relevant to teams building software with AI tools.
Each pack contains OPA/Rego policies mapped to specific framework controls, a
pack manifest following POLVAL conventions, test fixtures, and documentation.
Teams adopt governance with `anvil policy install --pack <name>` and get
immediate gate-check coverage. The pack selection reflects Anvil's positioning
as an AI-aware governance layer: application security fundamentals (OWASP),
enterprise compliance (SOC 2, ISO 27001), data protection (GDPR), and
AI-specific regulation (NIST AI RMF, EU AI Act).

## In Scope

- **OWASP Top 10** policy pack — Application security baseline covering
  injection, broken auth, sensitive data exposure, XSS, insecure deserialisation,
  vulnerable components, and insufficient logging
- **SOC 2** policy pack — Trust Services Criteria for Security, Availability,
  and Confidentiality mapped to engineering controls
- **ISO 27001** policy pack — Annex A controls relevant to code and CI
  (access control, cryptography, operations security, secure development)
- **GDPR** policy pack — Technical measures for data protection by design
  (consent handling, data minimisation, encryption, right to erasure patterns,
  breach notification readiness)
- **NIST AI RMF** policy pack — AI Risk Management Framework controls for
  transparency, fairness, robustness, and accountability in AI-assisted
  development
- **EU AI Act** policy pack — Technical requirements for high-risk AI systems
  (human oversight, transparency, data governance, robustness, logging)
- Pack manifests with versioning, dependency declarations, and control mappings
- OPA/Rego policies per control with test cases and fixture data
- Control-mapping metadata linking each policy to its framework control ID
- Documentation per pack explaining control coverage and customisation
- Integration tests validating pack install, load, and evaluation

## Out of Scope

- ❌ HIPAA, PCI-DSS, NIST 800-53, CIS Controls (future packs once pattern is
  proven; stubs in AGOV-002)
- ❌ Compliance reporting and evidence generation (COMPLY module)
- ❌ Policy authoring UI or wizard
- ❌ Pack marketplace or remote registry
- ❌ Legal or regulatory advice — packs encode engineering best practices, not
  legal interpretations
- ❌ OPA engine implementation (OPAE module)
- ❌ Policy pack validation infrastructure (POLVAL module)

## Interfaces

**Depends on:**

- POLVAL-001 — Policy metadata schema (required fields for each policy)
- POLVAL-002 — Pack manifest format (manifest.json structure)
- POLVAL-003 — Pack validator (structural validation)
- POLVAL-004 — Policy test runner (test enforcement)
- POLVAL-005 — CLI `anvil policy validate` with `--pack` mode
- OPAE-006 — Policy library infrastructure (discovery and loading)
- `crates/anvil-policy` — OPA executor and bundle/loader infrastructure
  (replaces retired `packages/anvil/policy`)
- AGOV-002 — `anvil policy install --pack <name>` CLI entry point
- AGOV-001/006/007 — AI-governance signal producers (trust scores, capability
  manifests, audit-chain state) required by AI-specific packs. An OPA input
  bridge task in AGOV or OPAE must serialise these signals into policy
  evaluation context before CPACKS-051/061/062/063 can implement AI checks.

**Exposes:**

- `library/compliance/owasp-top-10/` — OWASP Top 10 policy pack
- `library/compliance/soc2/` — SOC 2 policy pack
- `library/compliance/iso-27001/` — ISO 27001 policy pack
- `library/compliance/gdpr/` — GDPR policy pack
- `library/compliance/nist-ai-rmf/` — NIST AI RMF policy pack
- `library/compliance/eu-ai-act/` — EU AI Act policy pack
- Control-mapping metadata consumable by COMPLY for reporting
- `anvil policy install --pack owasp-top-10|soc2|iso-27001|gdpr|nist-ai-rmf|eu-ai-act`

## Acceptance Criteria

- [ ] Each pack installs via `anvil policy install --pack <name>` without errors
- [ ] Each pack passes `anvil policy validate` with zero issues
- [ ] Each pack includes ≥ 5 Rego policies covering the framework's key controls
- [ ] Each policy has at least one passing test case and one failing test case
- [ ] Pack manifests include control-mapping metadata (framework, control ID,
      control title)
- [ ] Policies evaluate correctly against provided fixture data
- [ ] Packs are independently versionable (semver in manifest)
- [ ] Documentation per pack lists covered controls and known gaps
- [ ] All six packs load and evaluate in < 1s combined
- [ ] AI-specific packs (NIST AI RMF, EU AI Act) integrate with Anvil's
      AI-tool governance features (trust scores, capability declarations)

## Risks & Mitigations

| Risk                                             | Mitigation                                                |
| ------------------------------------------------ | --------------------------------------------------------- |
| Control mappings become stale as standards update | Version packs independently; track standard revision in manifest |
| False sense of compliance from partial coverage   | Document known gaps prominently; label coverage as "engineering controls only" |
| Rego policies too generic to catch real violations | Ship with realistic test fixtures; iterate based on user feedback |
| Overlap between packs (e.g., encryption controls) | Share common policies via a `common/` directory; packs import rather than duplicate |
| Performance degrades with many policies loaded    | Benchmark at pack level; use incremental evaluation where possible |
| AI regulation (EU AI Act) is still evolving       | Version the pack against the specific regulation revision; document provisional mappings |
| NIST AI RMF is guidance, not enforceable law      | Frame as best-practice engineering controls, not legal compliance |

---

## Work Items

### Phase A: Foundation & Common Infrastructure

#### CPACKS-001: Common compliance policy utilities

- **Intent:** Create shared Rego helpers and test utilities used across all
  compliance packs to avoid duplication.
- **Expected Outcome:** A `library/compliance/common/` directory containing
  shared Rego helper functions (string matching, severity mapping, metadata
  extraction) and shared test fixture generators. All packs import from common
  rather than duplicating logic.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/common/`
- **Non-scope:** Framework-specific policies
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/common/helpers.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/common/severity.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/common/metadata.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/common/test-helpers.rego`
- **Dependencies:** OPAE-006 (policy library infrastructure)
- **Validation:** `nx test runtime --testNamePattern="compliance-common"`
- **Confidence:** high

#### CPACKS-002: Pack manifest and control-mapping schema

- **Intent:** Extend the POLVAL manifest format with compliance-specific fields
  for framework control mappings.
- **Expected Outcome:** Each pack manifest includes a `controlMappings` array
  linking policy IDs to framework control IDs, control titles, and control
  categories. Schema is validated by POLVAL-003 automatically.
- **Scope:** `packages/anvil/runtime/src/gate/policy/`
- **Non-scope:** Reporting or evidence generation
- **Files:**
  - `packages/anvil/runtime/src/gate/policy/compliance-manifest.ts`
  - `packages/anvil/runtime/src/gate/policy/compliance-manifest.test.ts`
- **Dependencies:** POLVAL-001, POLVAL-002
- **Validation:** `nx test runtime --testNamePattern="compliance-manifest"`
- **Confidence:** high

### Phase B: OWASP Top 10 Pack

#### CPACKS-010: OWASP Top 10 pack manifest and structure

- **Intent:** Create the OWASP Top 10 pack directory structure and manifest
  with mappings to the 2021 OWASP Top 10 categories.
- **Expected Outcome:** A complete manifest.json listing all policies, their
  mappings to OWASP categories (A01–A10), pack version, and dependencies.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/`
- **Non-scope:** Policy implementation
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/manifest.json`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/README.md`
- **Dependencies:** CPACKS-002
- **Validation:** `anvil policy validate --pack owasp-top-10`
- **Confidence:** high

#### CPACKS-011: OWASP injection and XSS policies (A03, A07)

- **Intent:** Detect injection vulnerabilities (SQL, command, LDAP) and
  cross-site scripting patterns in code diffs.
- **Expected Outcome:** Rego policies for: SQL injection patterns (string
  concatenation in queries), command injection (unsanitised shell execution),
  XSS patterns (unescaped user input in HTML output), and template injection.
  Each with passing and failing test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/`
- **Non-scope:** Runtime DAST scanning
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/injection.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/injection_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-010
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/ -v`
- **Confidence:** high

#### CPACKS-012: OWASP broken access control policies (A01)

- **Intent:** Detect missing authorisation checks, insecure direct object
  references, and privilege escalation patterns.
- **Expected Outcome:** Rego policies for: missing auth middleware on route
  handlers, direct object reference without ownership checks, CORS
  misconfiguration patterns, and directory traversal risks. Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/`
- **Non-scope:** Identity provider configuration
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/broken-access-control.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/broken-access-control_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-010
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/ -v`
- **Confidence:** high

#### CPACKS-013: OWASP cryptographic failures policies (A02)

- **Intent:** Detect weak cryptographic practices — deprecated algorithms,
  insufficient key lengths, hardcoded secrets, and plaintext sensitive data.
- **Expected Outcome:** Rego policies for: weak algorithm usage (MD5, SHA1,
  DES, RC4), hardcoded credentials and API keys, missing encryption on
  sensitive data stores, and insecure random number generation. Each with
  test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/`
- **Non-scope:** Key management infrastructure
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/cryptographic-failures.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/cryptographic-failures_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-010
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/ -v`
- **Confidence:** high

#### CPACKS-014: OWASP security misconfiguration and logging policies (A05, A09)

- **Intent:** Detect security misconfigurations and insufficient logging in
  code and configuration files.
- **Expected Outcome:** Rego policies for: debug mode enabled in production
  configs, default credentials, missing security headers, verbose error messages
  exposing internals, missing audit logging on sensitive operations, and
  insufficient log detail. Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/`
- **Non-scope:** Infrastructure-level configuration scanning
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/security-misconfiguration.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/security-misconfiguration_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-010
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/ -v`
- **Confidence:** medium

#### CPACKS-015: OWASP vulnerable components policies (A06)

- **Intent:** Detect usage of known-vulnerable dependencies and outdated
  component patterns in code.
- **Expected Outcome:** Rego policies for: dependency age thresholds,
  known-vulnerable package patterns, unpinned dependency versions, and use of
  deprecated APIs. Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/`
- **Non-scope:** Vulnerability database integration (uses static patterns)
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/vulnerable-components.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/vulnerable-components_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-010
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/owasp-top-10/ -v`
- **Confidence:** medium

### Phase C: SOC 2 Pack

#### CPACKS-020: SOC 2 pack manifest and structure

- **Intent:** Create the SOC 2 pack directory structure and manifest with
  control mappings to Trust Services Criteria (TSC).
- **Expected Outcome:** A complete manifest.json listing all policies, their
  mappings to SOC 2 TSC categories (CC1–CC9, A1, C1), pack version, and
  dependencies.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/`
- **Non-scope:** Policy implementation
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/manifest.json`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/README.md`
- **Dependencies:** CPACKS-002
- **Validation:** `anvil policy validate --pack soc2`
- **Confidence:** high

#### CPACKS-021: SOC 2 logical access policies (CC6)

- **Intent:** Enforce logical and physical access controls from CC6 — detecting
  authentication gaps, authorisation bypass patterns, and session management
  issues.
- **Expected Outcome:** Rego policies for: missing authentication checks
  (CC6.1), authorisation bypass patterns (CC6.3), session timeout configuration
  (CC6.1), and least-privilege violations (CC6.3). Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/`
- **Non-scope:** Physical access controls
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/logical-access.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/logical-access_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-020
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/ -v`
- **Confidence:** high

#### CPACKS-022: SOC 2 change management policies (CC8)

- **Intent:** Enforce change management controls from CC8 — validating PR
  review requirements, test coverage gates, and deployment approval workflows.
- **Expected Outcome:** Rego policies for: minimum reviewer requirements
  (CC8.1), test coverage thresholds (CC8.1), and CI pipeline integrity checks
  (CC8.1). Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/`
- **Non-scope:** Deployment orchestration
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/change-management.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/change-management_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-020
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/ -v`
- **Confidence:** high

#### CPACKS-023: SOC 2 monitoring and risk policies (CC7, CC9)

- **Intent:** Enforce system monitoring and risk mitigation controls — detecting
  missing logging, absent health checks, unhandled errors, and missing circuit
  breakers.
- **Expected Outcome:** Rego policies for: audit logging presence (CC7.2),
  health-check endpoint detection (CC7.1), error monitoring configuration
  (CC7.3), unhandled exception patterns (CC9.1), and missing error boundaries
  (CC9.1). Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/`
- **Non-scope:** Monitoring infrastructure deployment; runtime resilience testing
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/monitoring-and-risk.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/monitoring-and-risk_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-020
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/soc2/ -v`
- **Confidence:** medium

### Phase D: ISO 27001 Pack

#### CPACKS-030: ISO 27001 pack manifest and structure

- **Intent:** Create the ISO 27001 pack directory structure and manifest with
  control mappings to Annex A categories.
- **Expected Outcome:** A complete manifest.json listing all policies in the
  pack, their control mappings to ISO 27001 Annex A controls (A.5–A.18), pack
  version, and dependencies.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/`
- **Non-scope:** Policy implementation
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/manifest.json`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/README.md`
- **Dependencies:** CPACKS-002
- **Validation:** `anvil policy validate --pack iso-27001`
- **Confidence:** high

#### CPACKS-031: ISO 27001 access control and cryptography policies (A.9, A.10)

- **Intent:** Enforce access control and cryptographic requirements from
  Annex A.9 and A.10.
- **Expected Outcome:** Rego policies for: hardcoded credential detection
  (A.9.2.3), permissive file modes (A.9.4.1), missing auth middleware (A.9.4.2),
  weak cipher detection (A.10.1.1), minimum key length enforcement (A.10.1.1),
  and plaintext secret patterns (A.10.1.2). Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/`
- **Non-scope:** Runtime authentication; key management systems
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/access-and-crypto.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/access-and-crypto_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-030
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/ -v`
- **Confidence:** high

#### CPACKS-032: ISO 27001 operations security policies (A.12)

- **Intent:** Enforce operations security from Annex A.12 — detecting missing
  logging, unvalidated inputs, and debug code left in production paths.
- **Expected Outcome:** Rego policies for: missing audit logging patterns
  (A.12.4.1), debug/console statement detection (A.12.1.4), and input
  validation gaps (A.12.2.1). Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/`
- **Non-scope:** Log aggregation infrastructure
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/operations-security.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/operations-security_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-030
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/ -v`
- **Confidence:** medium

#### CPACKS-033: ISO 27001 secure development policies (A.14)

- **Intent:** Enforce secure development requirements from Annex A.14 —
  detecting missing security tests, insecure dependencies, and unsafe coding
  patterns.
- **Expected Outcome:** Rego policies for: dependency vulnerability thresholds
  (A.14.2.1), security test coverage requirements (A.14.2.8), and unsafe
  function usage patterns (A.14.2.5). Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/`
- **Non-scope:** Vulnerability scanning tools
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/secure-development.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/secure-development_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-030
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/iso-27001/ -v`
- **Confidence:** medium

### Phase E: GDPR Pack

#### CPACKS-040: GDPR pack manifest and structure

- **Intent:** Create the GDPR pack directory structure and manifest with control
  mappings to GDPR articles and technical measures.
- **Expected Outcome:** A complete manifest.json listing all policies, their
  mappings to GDPR articles (Art. 5, 25, 30, 32, 33, 35), pack version, and
  dependencies.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/`
- **Non-scope:** Policy implementation; organisational measures
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/manifest.json`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/README.md`
- **Dependencies:** CPACKS-002
- **Validation:** `anvil policy validate --pack gdpr`
- **Confidence:** high

#### CPACKS-041: GDPR data protection by design policies (Art. 25, Art. 5)

- **Intent:** Enforce data protection by design and by default — detecting
  excessive data collection, missing consent patterns, and data retention
  without limits.
- **Expected Outcome:** Rego policies for: data minimisation patterns (detecting
  over-collection of user fields), missing consent-check patterns before data
  processing, hardcoded retention periods without expiry, and PII logging
  detection. Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/`
- **Non-scope:** Cookie consent UI; privacy policy content
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/data-protection-by-design.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/data-protection-by-design_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-040
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/ -v`
- **Confidence:** medium

#### CPACKS-042: GDPR security of processing policies (Art. 32)

- **Intent:** Enforce technical security measures required by Art. 32 —
  encryption, pseudonymisation, and integrity controls on personal data.
- **Expected Outcome:** Rego policies for: encryption at rest and in transit on
  personal data paths, pseudonymisation patterns (detecting raw PII in
  analytics/logging), and access control on personal data endpoints. Each with
  test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/`
- **Non-scope:** Organisational security measures
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/security-of-processing.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/security-of-processing_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-040
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/ -v`
- **Confidence:** medium

#### CPACKS-043: GDPR data subject rights and breach readiness policies (Art. 17, Art. 33)

- **Intent:** Detect missing implementations of data subject rights (deletion,
  export) and breach notification readiness.
- **Expected Outcome:** Rego policies for: missing deletion endpoint or
  soft-delete mechanism for user data, missing data export/portability patterns,
  and breach notification configuration (logging, alerting). Each with test
  cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/`
- **Non-scope:** Breach response procedures; DPO appointment
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/data-rights-and-breach.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/data-rights-and-breach_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-040
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/gdpr/ -v`
- **Confidence:** low

### Phase F: NIST AI RMF Pack

#### CPACKS-050: NIST AI RMF pack manifest and structure

- **Intent:** Create the NIST AI Risk Management Framework pack directory
  structure and manifest with mappings to RMF functions and categories.
- **Expected Outcome:** A complete manifest.json listing all policies, their
  mappings to NIST AI RMF functions (Govern, Map, Measure, Manage) and
  subcategories, pack version, and dependencies.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/`
- **Non-scope:** Policy implementation; organisational governance functions
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/manifest.json`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/README.md`
- **Dependencies:** CPACKS-002
- **Validation:** `anvil policy validate --pack nist-ai-rmf`
- **Confidence:** high

#### CPACKS-051: NIST AI RMF transparency and documentation policies (Map, Govern)

- **Intent:** Enforce AI transparency requirements — ensuring AI-generated code
  is attributed, model usage is documented, and decision rationale is recorded.
- **Expected Outcome:** Rego policies for: AI attribution markers in commit
  metadata (MAP 1.1), model version documentation in configuration (MAP 1.5),
  missing rationale on AI-suggested architectural changes (GOV 1.3), and
  provenance trail completeness (GOV 1.7). Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/`
- **Non-scope:** Model card generation; AI training data governance
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/transparency.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/transparency_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-050, AGOV-001 (trust scoring)
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/ -v`
- **Confidence:** medium

#### CPACKS-052: NIST AI RMF robustness and reliability policies (Measure, Manage)

- **Intent:** Enforce AI robustness requirements — ensuring AI-generated code
  has adequate test coverage, error handling, and validation safeguards.
- **Expected Outcome:** Rego policies for: minimum test coverage on
  AI-contributed files (MEASURE 2.6), error handling completeness on AI-written
  code (MANAGE 2.2), input validation on AI-generated endpoints (MEASURE 2.9),
  and rollback capability on AI-driven changes (MANAGE 4.1). Each with test
  cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/`
- **Non-scope:** Model performance benchmarking
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/robustness.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/robustness_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-050
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/ -v`
- **Confidence:** medium

#### CPACKS-053: NIST AI RMF fairness and bias policies (Measure)

- **Intent:** Detect potential bias and fairness issues in AI-assisted code —
  biased data handling, discriminatory logic patterns, and missing fairness
  checks.
- **Expected Outcome:** Rego policies for: demographic attribute usage without
  fairness documentation (MEASURE 2.10), hardcoded demographic-based logic
  (MEASURE 2.11), missing bias testing fixtures (MEASURE 2.6), and
  feature-selection patterns that proxy for protected attributes. Each with
  test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/`
- **Non-scope:** Statistical bias measurement; model fairness metrics
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/fairness.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/fairness_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-050
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/nist-ai-rmf/ -v`
- **Confidence:** low

### Phase G: EU AI Act Pack

#### CPACKS-060: EU AI Act pack manifest and structure

- **Intent:** Create the EU AI Act pack directory structure and manifest with
  mappings to EU AI Act articles and requirements for high-risk AI systems.
- **Expected Outcome:** A complete manifest.json listing all policies, their
  mappings to EU AI Act articles (Art. 9–15, Art. 52), pack version, and the
  regulation revision tracked.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/`
- **Non-scope:** Policy implementation; classification of AI system risk levels
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/manifest.json`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/README.md`
- **Dependencies:** CPACKS-002
- **Validation:** `anvil policy validate --pack eu-ai-act`
- **Confidence:** high

#### CPACKS-061: EU AI Act human oversight policies (Art. 14)

- **Intent:** Enforce human oversight requirements — ensuring AI-assisted
  changes include human review checkpoints and override mechanisms.
- **Expected Outcome:** Rego policies for: mandatory human review on
  AI-generated PRs exceeding thresholds (Art. 14.1), human approval gates on
  AI-driven configuration changes (Art. 14.4), and override mechanisms
  documented in capability manifests (Art. 14.3). Integrates with AGOV trust
  scoring to determine oversight level. Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/`
- **Non-scope:** Approval workflow orchestration
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/human-oversight.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/human-oversight_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-060, AGOV-001 (trust scoring)
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/ -v`
- **Confidence:** medium

#### CPACKS-062: EU AI Act transparency and logging policies (Art. 12, Art. 13, Art. 52)

- **Intent:** Enforce transparency and automatic logging requirements —
  ensuring AI tool usage is logged, outputs are labelled, and users are informed
  of AI involvement.
- **Expected Outcome:** Rego policies for: AI-generated content labelling in
  commit metadata and PR descriptions (Art. 52), automatic logging of AI tool
  invocations and parameters (Art. 12), traceability from output back to AI
  tool version and prompt (Art. 13), and log retention configuration. Each
  with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/`
- **Non-scope:** User-facing AI disclosure UI
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/transparency-logging.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/transparency-logging_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-060, AGOV-006 (hash-chained audit)
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/ -v`
- **Confidence:** medium

#### CPACKS-063: EU AI Act data governance and robustness policies (Art. 10, Art. 15)

- **Intent:** Enforce data governance and technical robustness for AI-assisted
  development — ensuring training data quality, output validation, and
  resilience to adversarial inputs.
- **Expected Outcome:** Rego policies for: AI output validation checks (Art.
  15.3), adversarial input handling on AI-generated endpoints (Art. 15.4),
  data quality assertions in test fixtures (Art. 10.2), and configuration
  guardrails on AI tool capabilities (Art. 10.5). Each with test cases.
- **Scope:** `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/`
- **Non-scope:** Training data curation; model robustness testing
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/data-governance-robustness.rego`
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/data-governance-robustness_test.rego`
- **Dependencies:** CPACKS-001, CPACKS-060, AGOV-007 (capability declarations)
- **Validation:** `opa test packages/anvil/runtime/src/gate/__fixtures__/policies/compliance/eu-ai-act/ -v`
- **Confidence:** low

### Phase H: Integration & Documentation

#### CPACKS-070: Pack installation integration tests

- **Intent:** Validate the full install → load → evaluate pipeline for all
  six packs working together.
- **Expected Outcome:** Integration tests that install each pack, run
  `anvil policy validate`, evaluate policies against fixture data, and verify
  control-mapping metadata is accessible. Tests cover pack isolation (installing
  one does not affect others) and combined loading.
- **Scope:** `packages/anvil/runtime/src/gate/policy/`, `apps/anvil-cli/src/commands/`
- **Non-scope:** Performance benchmarking
- **Files:**
  - `packages/anvil/runtime/src/gate/policy/compliance-packs.integration.test.ts`
- **Dependencies:** All Phase B–G manifest and policy implementation tasks
    (CPACKS-010–063 inclusive)
- **Validation:** `nx test runtime --testNamePattern="compliance-packs.integration"`
- **Confidence:** high

#### CPACKS-071: Compliance pack documentation

- **Intent:** Document each pack's control coverage, customisation options,
  and known gaps for users and auditors.
- **Expected Outcome:** A guide per pack explaining: which controls are covered,
  which are out of scope and why, how to customise severity and thresholds, and
  how to extend with additional policies. Plus an overview guide comparing all
  six packs.
- **Scope:** `docs/guides/`
- **Non-scope:** Legal compliance guidance
- **Files:**
  - `docs/guides/compliance-packs.md`
  - `docs/guides/compliance-packs-owasp.md`
  - `docs/guides/compliance-packs-soc2.md`
  - `docs/guides/compliance-packs-iso-27001.md`
  - `docs/guides/compliance-packs-gdpr.md`
  - `docs/guides/compliance-packs-nist-ai-rmf.md`
  - `docs/guides/compliance-packs-eu-ai-act.md`
- **Dependencies:** CPACKS-070
- **Validation:** Manual review — all covered controls listed, gaps documented
- **Confidence:** high

---

## Decisions

D-CPACKS-001: Pack selection — security + enterprise + AI regulation

- **Rationale:** The six packs cover three tiers of value: (1) OWASP is the
  universal application security baseline every team needs, (2) SOC 2, ISO
  27001, and GDPR are the enterprise compliance frameworks most commonly
  required, (3) NIST AI RMF and EU AI Act are the emerging AI-specific
  frameworks that differentiate Anvil. HIPAA, PCI-DSS, NIST 800-53, and CIS
  Controls follow once the pattern is proven.
- **Alternatives:** Ship all frameworks simultaneously; ship only traditional
  compliance (no AI-specific)
- **Trade-offs:** Six packs is ambitious but achievable in phases. The AI packs
  are lower confidence but higher strategic value.

D-CPACKS-002: Shared common directory over policy duplication

- **Rationale:** Many controls overlap across frameworks (encryption, access
  control, logging). A `common/` directory with shared helpers reduces
  maintenance burden and ensures consistency.
- **Alternatives:** Fully self-contained packs with no shared code
- **Trade-offs:** Introduces a dependency between packs; but the alternative
  is worse (divergent implementations of the same check).

D-CPACKS-003: Policies as Rego, not native checks

- **Rationale:** Rego is the policy language retained by ADR-040, and
  `crates/anvil-policy-engine` evaluates it through regorus. Compliance
  policies should use that same facade for consistency, composability, and
  portability. Native gate checks (in `crates/anvil-checks`) are for structural
  concerns; Rego is for policy logic.
- **Alternatives:** Implement as native Rust gate checks in `crates/anvil-checks`
- **Trade-offs:** Requires Rego literacy and pack validation; Go OPA remains
  useful as a reference test runner, but the product runtime is the regorus
  facade.

D-CPACKS-004: AI packs reference AGOV trust scoring and capability model

- **Rationale:** The NIST AI RMF and EU AI Act packs are more powerful when
  they integrate with Anvil's AI governance features (trust scores from
  AGOV-001, capability declarations from AGOV-007). This creates a coherent
  governance story rather than isolated rule sets.
- **Alternatives:** Standalone AI packs with no AGOV integration
- **Trade-offs:** Creates a dependency on AGOV; but the packs still function
  (with reduced capability) without it.

## Notes

### Control Coverage Summary

| Pack         | Framework Sections         | Estimated Policies | Key Areas                                    |
| ------------ | -------------------------- | ------------------ | -------------------------------------------- |
| OWASP Top 10 | A01–A10 (2021 edition)     | 15–18              | Injection, access, crypto, config, logging   |
| SOC 2        | CC6, CC7, CC8, CC9         | 10–12              | Access, monitoring, change mgmt, risk        |
| ISO 27001    | A.9, A.10, A.12, A.14      | 12–15              | Access, crypto, ops, secure dev              |
| GDPR         | Art. 5, 17, 25, 32, 33     | 8–10               | Data protection, encryption, rights, breach  |
| NIST AI RMF  | Govern, Map, Measure, Manage | 10–12             | Transparency, robustness, fairness, oversight |
| EU AI Act    | Art. 9–15, 52              | 8–10               | Human oversight, logging, transparency, data |

### Pack Priority Order

```text
Phase B: OWASP Top 10   — Highest value, immediate applicability
Phase C: SOC 2           — Most requested by SaaS companies
Phase D: ISO 27001       — Enterprise standard, high overlap with OWASP
Phase E: GDPR            — Data protection, common EU/UK requirement
Phase F: NIST AI RMF     — AI governance differentiator
Phase G: EU AI Act       — Emerging regulation, strategic positioning
```

### Future Packs (not in this module)

```text
HIPAA         — Healthcare (§164.312 technical safeguards)
PCI-DSS       — Payment card industry
NIST 800-53   — US federal government
CIS Controls  — Centre for Internet Security benchmarks
FedRAMP       — US federal cloud
```

### Pack Directory Layout

```text
crates/anvil-policy/policies/compliance/
├── common/
│   ├── helpers.rego
│   ├── severity.rego
│   ├── metadata.rego
│   └── test-helpers.rego
├── owasp-top-10/
│   ├── manifest.json
│   ├── README.md
│   ├── injection.rego
│   ├── injection_test.rego
│   ├── broken-access-control.rego
│   ├── broken-access-control_test.rego
│   ├── cryptographic-failures.rego
│   ├── cryptographic-failures_test.rego
│   ├── security-misconfiguration.rego
│   ├── security-misconfiguration_test.rego
│   ├── vulnerable-components.rego
│   └── vulnerable-components_test.rego
├── soc2/
│   ├── manifest.json
│   ├── README.md
│   ├── logical-access.rego
│   ├── logical-access_test.rego
│   ├── change-management.rego
│   ├── change-management_test.rego
│   ├── monitoring-and-risk.rego
│   └── monitoring-and-risk_test.rego
├── iso-27001/
│   ├── manifest.json
│   ├── README.md
│   ├── access-and-crypto.rego
│   ├── access-and-crypto_test.rego
│   ├── operations-security.rego
│   ├── operations-security_test.rego
│   ├── secure-development.rego
│   └── secure-development_test.rego
├── gdpr/
│   ├── manifest.json
│   ├── README.md
│   ├── data-protection-by-design.rego
│   ├── data-protection-by-design_test.rego
│   ├── security-of-processing.rego
│   ├── security-of-processing_test.rego
│   ├── data-rights-and-breach.rego
│   └── data-rights-and-breach_test.rego
├── nist-ai-rmf/
│   ├── manifest.json
│   ├── README.md
│   ├── transparency.rego
│   ├── transparency_test.rego
│   ├── robustness.rego
│   ├── robustness_test.rego
│   ├── fairness.rego
│   └── fairness_test.rego
└── eu-ai-act/
    ├── manifest.json
    ├── README.md
    ├── human-oversight.rego
    ├── human-oversight_test.rego
    ├── transparency-logging.rego
    ├── transparency-logging_test.rego
    ├── data-governance-robustness.rego
    └── data-governance-robustness_test.rego
```
