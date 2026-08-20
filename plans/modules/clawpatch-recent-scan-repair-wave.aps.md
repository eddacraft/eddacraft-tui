<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Clawpatch Recent Scan Repair Wave

| ID       | Owner | Priority | Status      | Progress |
| -------- | ----- | -------- | ----------- | -------- |
| CLAWSCAN | —     | P2       | In Progress | 0/6      |

**Last reviewed:** 2026-08-20 — validated against Clawpatch run
`20260819T164749-cb786b` and current `origin/main`.

## Purpose

Close the actionable product, contract-gate, and test-harness findings from the
most recent Clawpatch scan without duplicating existing coverage or editing the
shared continuous-improvement backlog.

## In scope

- Bound the public early-access verification request.
- Remove unusable placeholder PGP reporting guidance.
- Bind positioning claims to their intended composed components.
- Make three focused CLI integration-test contracts fault-detecting.

## Out of scope

- Publishing or generating a PGP key.
- Changing Vercel Analytics or the disclosed privacy posture.
- Duplicating SARIF schema, canonical ordering, or MCP refresh coverage.
- Editing the shared CIB module.
- Product or protocol redesign.

## Work Items

### CLAWSCAN-001: Bound early-access verification

- **Status:** In Progress
- **Intent:** A stalled verification upstream cannot hold a website request open indefinitely.
- **Expected Outcome:** the verification fetch has an eight-second timeout and timeout or network failure returns the existing 503 contract.
- **Files:** `apps/website/app/api/early-access/install/route.ts`,
  `apps/website/app/api/early-access/install/route.test.ts`, `vitest.config.ts`
- **Validation:** `pnpm exec vitest run apps/website/app/api/early-access/install/route.test.ts`
- **Finding ID:** `fnd_sig-feat-library-d956a2897f-0f50_c3327cf9e0`
- **Risk:** standard

### CLAWSCAN-002: Keep security reporting guidance usable

- **Status:** In Progress
- **Intent:** Public security guidance does not advertise unverifiable encryption material.
- **Expected Outcome:** the security page keeps the working reporting channel and does not publish placeholder PGP instructions or a missing key URL.
- **Files:** `apps/website/app/security/page.tsx`,
  `apps/website/scripts/check-public-trust.mjs`,
  `apps/website/package.json`
- **Validation:** `pnpm --dir apps/website test:public-trust`
- **Finding ID:** `fnd_sig-feat-library-d956a2897f-6ccd_5f040c6ece`
- **Risk:** standard

### CLAWSCAN-003: Bind positioning claims to composed sections

- **Status:** In Progress
- **Intent:** Positioning checks prove required claims occur in the intended rendered sections.
- **Expected Outcome:** each required claim is checked in its owning composed component, while retired claims remain absent from the scanned website surface.
- **Files:** `apps/website/scripts/check-positioning.mjs`
- **Validation:** `pnpm --dir apps/website test:positioning`
- **Finding ID:** `fnd_sig-feat-library-4b97edc98b-8c51_2bd1c39f10`
- **Risk:** standard

### CLAWSCAN-004: Bound MCP shutdown-response reads

- **Status:** In Progress
- **Intent:** A broken MCP server cannot hang the shutdown integration test.
- **Expected Outcome:** initialise and shutdown responses use the suite's bounded stdout receiver before the bounded child-exit wait.
- **Files:** `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_serve_stdio mcp_serve_stdio_shutdown_flushes_response_before_exit_notification -- --exact`
- **Finding ID:** `fnd_sig-feat-test-suite-dd97196f08-6_a49e0f4212`
- **Risk:** low

### CLAWSCAN-005: Fail config-mode tests when Git is absent

- **Status:** In Progress
- **Intent:** A missing Git executable is a failed suite precondition rather than a passing skip.
- **Expected Outcome:** old supported test hosts may skip config-mode integration paths, but missing or unparsable Git fails loudly and is regression-tested.
- **Files:** `crates/anvil-cli/tests/hooks_config_mode.rs`
- **Validation:** `cargo test -p eddacraft-anvil --test hooks_config_mode --no-fail-fast`
- **Finding ID:** `fnd_sig-feat-test-suite-e32dc70b0c-5_d227f2a3ef`
- **Risk:** low

### CLAWSCAN-006: Verify explicit MCP pin persistence

- **Status:** In Progress
- **Intent:** The CLI round trip proves the requested pin version is durably stored.
- **Expected Outcome:** the integration test checks the exact persisted version before unpinning.
- **Files:** `crates/anvil-cli/tests/mcp_heal.rs`
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_heal mcp_pin_and_unpin_round_trip -- --exact`
- **Finding ID:** `fnd_sig-feat-test-suite-ef3d495453-e_80e92fb3d6`
- **Risk:** low

## Validated non-actionable findings

The following findings are dispositioned in the Clawpatch store rather than
implemented here:

- `fnd_sig-feat-library-d956a2897f-0b89_b3ef3a82f5` — privacy allegation does
  not contradict the disclosed cookie-free analytics posture.
- `fnd_sig-feat-test-suite-db47ea07e9-a_e4ab5a4d3c` — SARIF schema validation
  exists in the gate, check, and audit adapter suites.
- `fnd_sig-feat-test-suite-e6d210468e-0_9ce4779547` — canonical ordering is
  directly covered by reordered YAML/JSON and canonicalisation tests.
- `fnd_sig-feat-test-suite-ef3d495453-1_f17e091ac2` — the full MCP launch entry
  is covered by the shared refresh path; pinning changes only reporting.

## Wave validation

- `pnpm --dir apps/website test:positioning`
- `pnpm --dir apps/website test:public-trust`
- `pnpm exec vitest run apps/website/app/api/early-access/install/route.test.ts`
- `cargo test -p eddacraft-anvil --test mcp_serve_stdio mcp_serve_stdio_shutdown_flushes_response_before_exit_notification -- --exact`
- `cargo test -p eddacraft-anvil --test hooks_config_mode --no-fail-fast`
- `cargo test -p eddacraft-anvil --test mcp_heal mcp_pin_and_unpin_round_trip -- --exact`
- `pnpm validate:changed`
- `pnpm docs:check`
- `pnpm aps:active-lint`
- `pnpm aps:index:check`
