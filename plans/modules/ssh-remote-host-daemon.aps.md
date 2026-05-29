# SSH Remote Host Daemon

| ID        | Owner  | Status   | Progress |
| --------- | ------ | -------- | -------- |
| SSHREMOTE | @aneki | Proposed | 0/8 |

**Last reviewed:** 2026-05-14 (created from the accepted planning direction that
SSH support should run the daemon on the remote host, not bridge remote files
into a local daemon. ADR-043 is Proposed; this module remains Proposed until the
remote-state names, first surface, and validation harness are reviewed.)

## Purpose

Make SSH remote development a first-class Anvil workflow by running the daemon,
hooks, launcher, witness writes, and process control on the remote host where the
checkout and file writes happen. Local tools may initiate SSH commands and render
results, but the remote execution scope owns the protection claim.

## In Scope

- SSH remote-host daemon model from ADR-043
- Remote `anvil intercept ensure` bootstrap and status probing
- Remote binary/protocol/version checks
- Remote checkout identity verification using `anvil/project-id`
- Remote protection-claim states and contract fixtures
- Remote `anvil-run` session launch and heartbeat flow
- Remote hook/bootstrap behaviour inside the remote checkout
- Remote witness/L4 integration through existing git federation
- Operator/user runbook for SSH remote adoption and troubleshooting

## Out of Scope

- SSHFS/local-daemon file watching for remote paths
- TCP transport for daemon IPC
- Direct tunnelling of local daemon IPC as the first implementation path
- Cross-Windows/WSL bridge semantics
- Hosted Anvil cloud, GitHub App, or central policy dependency
- Remote attestation beyond the user's SSH trust boundary

## Interfaces

- **Depends on:**
  - ADR-036 execution-scope and `os_locality_token` model
  - ADR-037 witness chain and L4 policy framework
  - ADR-038 hook surface and noise discipline
  - INTD daemon lifecycle and IPC
  - INTL `anvil-run` launcher/session model
  - MLP witness, hooks, L4, and protection-claim contracts
  - DRVR and RMCP/RMCPF surface-driver paths for local display/control
- **Exposes:**
  - SSH remote driver contract
  - remote status/protection states
  - remote bootstrap and launch commands
  - SSH remote runbook and troubleshooting guidance

## ADRs cited

- **ADR-036** — daemon scope, discovery, and OS-boundary policy
- **ADR-037** — witness chain and L4 policy framework
- **ADR-038** — hook surface and noise discipline
- **ADR-043** — SSH remote host daemon

## Work Items

### SSHREMOTE-001: Finalise Remote-Host Daemon Decision

- **Intent:** Promote the SSH remote-host daemon model from Proposed planning to
  an accepted implementation contract.
- **Expected Outcome:** ADR-043 and the SSH remote design spec are reviewed;
  unresolved state-name and first-surface choices are either decided or recorded
  as explicit task constraints.
- **Files:** `plans/decisions/043-ssh-remote-host-daemon.md`,
  `plans/specs/2026-05-14-ssh-remote-host-daemon.md`,
  `plans/decisions/DECISION-LOG.md`
- **Validation:** `pnpm adr:check` and `pnpm format:check`
- **Status:** open
- **Confidence:** high
- **changeType:** docs
- **releaseIntent:** hold
- **holdCondition:** Remote support is not release-eligible until at least one
  surface and the protection-claim contract are implemented.
- **releaseScope:** none

### SSHREMOTE-002: SSH Status and Bootstrap Contract

- **Intent:** Define the command contract local tools use to bootstrap and query
  remote Anvil state over SSH.
- **Expected Outcome:** Local callers can run remote `anvil intercept ensure`,
  `anvil status --json`, and `anvil doctor --json` with deterministic error
  mapping for missing binary, incompatible version, unreachable daemon, and
  remote checkout mismatch.
- **Files:** `crates/anvil-cli/src/commands/status.rs`,
  `crates/anvil-cli/src/commands/doctor.rs`, remote-driver module path TBD
- **Validation:** Remote-status fixture tests cover success, daemon-down,
  missing-binary, and version-mismatch cases.
- **Status:** open
- **Confidence:** medium
- **Dependencies:** SSHREMOTE-001
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** Requires SSHREMOTE-005 and SSHREMOTE-006 before user-facing
  SSH protection claim.
- **releaseScope:** minor

### SSHREMOTE-003: Remote Checkout Identity and Path Certainty

- **Intent:** Ensure remote requests bind to the remote checkout that actually
  owns the files and git hooks.
- **Expected Outcome:** Remote probes report `project_uuid`, remote `repo_root`,
  remote `worktree_root`, cwd, origin cross-check, and path-certainty verdict;
  ambiguous local/remote path mapping downgrades or refuses the claim.
- **Files:** remote-driver module path TBD, `crates/anvil-cli/src/activation/identity.rs`
- **Validation:** Fixture tests cover matching identity, missing identity,
  origin mismatch warning, and `remote-path-uncertain` refusal.
- **Status:** open
- **Confidence:** medium
- **Dependencies:** SSHREMOTE-002, MLP-001
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** Requires protection-state contract coverage.
- **releaseScope:** minor

### SSHREMOTE-004: Remote Protection-State Contract

- **Intent:** Add closed-set remote protection states so SSH support is rendered
  honestly across CLI, MCP, doctor, and future editor surfaces.
- **Expected Outcome:** Remote states are named, schema-pinned, and covered by
  contract fixtures before any release claims SSH remote protection.
- **Files:** `crates/anvil-cli/tests/protection_claim_states.rs`,
  `apps/e2e/src/protection_claim_states.spec.ts`, status schema paths TBD
- **Validation:** Contract tests cover remote unconfigured, daemon down,
  attached, protected, degraded, path-uncertain, and version-mismatch states.
- **Status:** open
- **Confidence:** high
- **Dependencies:** SSHREMOTE-002, SSHREMOTE-003, MLP-009
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** Blocks public SSH remote protection claim.
- **releaseScope:** minor

### SSHREMOTE-005: Remote `anvil-run` Launch Flow

- **Intent:** Route SSH remote agent sessions through the remote launcher so
  session registration, process control, heartbeats, and cleanup happen on the
  remote host.
- **Expected Outcome:** A local invocation can start a remote agent command via
  SSH while the remote `anvil-run` registers with the remote daemon and reports
  launch/fence failures using INTL's noise discipline.
- **Files:** `crates/anvil-run/` once INTL lands, remote-driver module path TBD
- **Validation:** Remote-launch tests cover successful launch, fenced worktree,
  daemon unavailable, heartbeat, and cleanup.
- **Status:** open
- **Confidence:** medium
- **Dependencies:** INTL-001, INTL-002, INTL-003, INTL-004, INTL-005, INTL-009,
  SSHREMOTE-002
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** Requires INTL launcher implementation.
- **releaseScope:** minor

### SSHREMOTE-006: Surface Driver Integration

- **Intent:** Let selected local surfaces display and control remote protection
  without using local daemon discovery for remote files.
- **Expected Outcome:** The first supported surface routes remote status and
  validation through the SSH driver contract, clearly labelling remote scope in
  user-visible output.
- **Files:** DRVR/RMCP/RMCPF surface path TBD after first-surface selection
- **Validation:** Surface tests prove local daemon absence does not downgrade a
  healthy remote claim, and local daemon health does not upgrade an unhealthy
  remote claim.
- **Status:** open
- **Confidence:** medium
- **Dependencies:** SSHREMOTE-002, SSHREMOTE-004
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** First surface must be selected during SSHREMOTE-001 review.
- **releaseScope:** minor

### SSHREMOTE-007: Remote Hooks and Witness Integration

- **Intent:** Ensure remote checkouts install hooks and write witnesses locally
  on the remote host while preserving ADR-037 L4 verification.
- **Expected Outcome:** `anvil hook bootstrap`, pre-commit, pre-push,
  post-commit, post-merge, and post-rewrite work in an SSH remote checkout;
  remote witness lines carry the remote scope and pass L4 verification.
- **Files:** `crates/anvil-cli/src/commands/hook.rs`, `crates/anvil-hook/`,
  `crates/anvil-witness/`
- **Validation:** Remote git fixture proves commit, witness append, push-range
  verification, and missing-witness recovery in a remote checkout.
- **Status:** open
- **Confidence:** medium
- **Dependencies:** MLP-002, MLP-003, MLP-004, MLP-005, MLP-008,
  SSHREMOTE-003
- **changeType:** feature
- **releaseIntent:** hold
- **holdCondition:** Requires remote protection-state contract coverage.
- **releaseScope:** minor

### SSHREMOTE-008: SSH Remote Runbook and E2E Harness

- **Intent:** Make remote support operable and testable before release.
- **Expected Outcome:** A runbook documents supported SSH setup, bootstrap,
  protection states, common failures, and recovery commands; E2E harness covers
  the minimum remote workflow under a local SSH test fixture or documented skip.
- **Files:** `docs/runbooks/anvil-ssh-remote.md`, `apps/e2e/src/**/*.e2e.test.ts`
- **Validation:** E2E remote smoke test passes or skips with a deterministic
  unsupported-host reason; runbook commands are reviewed against the harness.
- **Status:** open
- **Confidence:** medium
- **Dependencies:** SSHREMOTE-004, SSHREMOTE-005, SSHREMOTE-007
- **changeType:** docs
- **releaseIntent:** hold
- **holdCondition:** Required before any SSH remote support release note.
- **releaseScope:** none
