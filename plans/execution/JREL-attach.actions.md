# JREL attach reconciliation checkpoints

Authority: [JREL](../modules/journey-reliability.aps.md).
Context: [attach review and proposed evidence contract](../specs/2026-09-06-mcp-attach-reconciliation.md).
These checkpoints do not create new work items or change existing claims.
JREL-002 is already being implemented in PR #4416 / claim #4408.

## Wave 0: establish evidence and settle contracts

### 1. Reconcile JREL-002 with its open implementation

- **Checkpoint:** Attach, readiness and validation evidence have explicit accepted meanings.
- **Validate:** Review ADR-141 and JREL-002 outcomes against the pinned PR head.

### 2. Establish JREL-012 failing product fixtures

- **Checkpoint:** Real binaries reproduce attach failure without fabricated client evidence.
- **Validate:** Use the JREL-012 harness; missing binaries and scenarios must fail.

### 3. Settle JREL-010 identity inputs

- **Checkpoint:** Attach and tool requests share one admitted worktree contract.
- **Validate:** Review launch-root and negative-admission scenarios before attachment changes.

## Wave 1: join the runtime planes

### 4. Complete JREL-002 live evidence

- **Checkpoint:** Session leases and validation observations remain distinct and independently attributed.
- **Validate:** `cargo test -p eddacraft-anvil -p eddacraft-anvil-run -p eddacraft-anvil-intercept --no-fail-fast`

### 5. Complete JREL-004 daemon convergence

- **Checkpoint:** Same-scope clients converge without collapsing intentional isolation boundaries.
- **Validate:** Run the JREL-004 endpoint/start/recycle matrix through real processes.

### 6. Complete JREL-003 watcher recovery

- **Checkpoint:** Failed watchers recover once or expose a bounded typed failure.
- **Validate:** Kill a fixture child, rerun the CLI, observe save validation.

## Wave 2: integrate identity, status and budgets

### 7. Complete JREL-010 admitted-root integration

- **Checkpoint:** Every supported launch location preserves identity and containment.
- **Validate:** Run positive and negative root scenarios through actual MCP tools.

### 8. Complete JREL-005 shared status projection

- **Checkpoint:** CLI and MCP report consistent provenance and component-specific recovery.
- **Validate:** Compare status and validation across the spec's evidence matrix.

### 9. Complete JREL-011 lifecycle and scale gates

- **Checkpoint:** Session bursts cannot starve status, validation or bounded shutdown.
- **Validate:** Measure deadlines and payloads with around 100 concurrent sessions.

## Wave 3: product acceptance and release

### 10. Close JOURNEY-014 reliability evidence

- **Checkpoint:** All required JREL scenarios pass without skipped execution legs.
- **Validate:** Run JREL-012's published no-skip command and reconcile all JREL owners.

### 11. Complete JOURNEY-015 installed-build rehearsal

- **Checkpoint:** Distributed binaries reproduce verified attach and truthful recovery beyond expiry.
- **Validate:** Record source, release, package, platform and real-client evidence.

JREL-001 continuity and JREL-006..009 retain their existing scope and dependencies;
this attach slice does not discharge the full JOURNEY-014 gate on its own.
