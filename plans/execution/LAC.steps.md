# Steps: LAC

| Field | Value |
|-------|-------|
| Source | [../modules/lineage-authorship-confidence.aps.md](../modules/lineage-authorship-confidence.aps.md) |
| Task | LAC — Full module execution |
| Status | Draft |

## Prerequisites

- [ ] Attribution schema approved by CLI/runtime stakeholders
- [ ] Baseline collector sources available (git + session/tool metadata)
- [ ] Language allocation decision accepted (ADR-014)

## Steps

### 1. Define attribution contract

- **Checkpoint:** Canonical schema validates all expected attribution fields.
- **Validate:** `pnpm nx test contracts --testNamePattern="attribution schema"`

### 2. Capture attribution evidence

- **Checkpoint:** Collectors produce normalized, source-tagged evidence.
- **Validate:** `pnpm nx test anvil-cli --testNamePattern="provenance collector"`

### 3. Reconcile attribution outcomes

- **Checkpoint:** Line outcomes include actor, model, confidence, reasons.
- **Validate:** `pnpm nx test anvil-cli --testNamePattern="authorship confidence"`

### 4. Persist and query attribution

- **Checkpoint:** File-line and PR queries return deterministic records.
- **Validate:** `pnpm nx test anvil-cli --testNamePattern="authorship store"`

### 5. Expose blame and summary commands

- **Checkpoint:** CLI surfaces answer line and PR attribution questions.
- **Validate:** `pnpm nx test anvil-cli --testNamePattern="authorship command"`

### 6. Export/sign evidence bundles

- **Checkpoint:** Export includes confidence rationale and signing state.
- **Validate:** `pnpm nx test anvil-cli --testNamePattern="authorship export"`

## Completion

- [ ] All checkpoints validated
- [ ] Task marked complete in source module
