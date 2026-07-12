# Tool Decision Matrix (Completed Planning Pass)

## Adopt Directly (explicit ADR + APS)

- OPA
  - ADR: existing hybrid OPA architecture decision
  - Module: existing OPA modules + OPAG
- Eval harness framework
  - ADR: `plans/decisions/012-eval-harness-adoption.md`
  - Module: `plans/archive/modules/eval-harness-integration.aps.md`

## Borrow Patterns (Anvil-native design; no vendor naming in module docs)

- Compliance evidence workspace pattern
  - Module: `plans/modules/compliance-evidence-workspace.aps.md`
- Contextual policy assertion pattern
  - Module: `plans/archive/modules/contextual-policy-assertions.aps.md`
- IO risk control/scanner taxonomy pattern
  - Module: `plans/archive/modules/io-risk-controls.aps.md`
- Gateway control-plane deployment pattern
  - Module: `plans/modules/gateway-control-plane-patterns.aps.md`
- Adversarial probe catalog pattern
  - Module: `plans/archive/modules/adversarial-testing-catalog.aps.md`
- Prompt-attack regression pack pattern
  - Module: `plans/modules/prompt-attack-regression-packs.aps.md`
- Trust-center publishing automation pattern
  - Module: `plans/modules/trust-center-automation.aps.md`

## Notes

- Borrowed modules intentionally avoid external tool naming in their own documents.
- Adoption modules include explicit ADR entries and adapter boundaries.
