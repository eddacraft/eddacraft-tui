# External Tool Integration Strategy (2026-03-07)

## Decision Frame

For each candidate tool, decide one of:

1. **Borrow pattern** (do not name vendor/tool in product-facing design)
2. **Adopt directly** (name tool explicitly; create ADR + APS module)

Decision criteria:

- Strategic differentiation risk
- Time-to-value for beta users
- Runtime coupling and migration cost
- Data/control-plane ownership
- Compliance and auditability requirements

---

## Candidate Outcomes

### 1) OPA
- **Decision:** Adopt directly (already established)
- **Reason:** Core policy engine with strong ecosystem fit and low differentiation risk.

### 2) Compliance evidence workspace capability
- **Decision:** Borrow pattern (no external tool name in implementation plans)
- **Reason:** High strategic differentiation for Anvil's trust UX; we should own information architecture and evidence model.

### 3) Eval regression harness capability
- **Decision:** Adopt directly (framework integration)
- **Reason:** Fastest path to CI-native trust regression loops; low lock-in risk if wrapped behind adapter.

### 4) Contextual policy assertion capability
- **Decision:** Borrow pattern (no external tool name)
- **Reason:** Assertions are strategic product behavior and should map to Anvil policy pack semantics.

### 5) IO risk taxonomy + scanners capability
- **Decision:** Borrow pattern (no external tool name)
- **Reason:** Scanner taxonomy is useful, but direct runtime dependency can create policy drift; better to implement provider-agnostic controls.

### 6) LLM gateway control-plane pattern
- **Decision:** Borrow pattern (no external tool name)
- **Reason:** Architecture pattern is useful for topology docs and deployment options; avoid unnecessary product coupling.

---

## Specific Thinking Requested

## A) Compliance evidence workspace capability (borrow)

Why borrow, not adopt:
- This is close to Anvil’s differentiated value proposition (governance trust UX).
- The competitive moat is in evidence model quality, workflow ergonomics, and reporting clarity.
- Adopting a third-party implementation too early risks matching their IA and constraints.

What to borrow:
- Framework crosswalk shape (control -> evidence -> status -> owner)
- Audit export posture
- “Explainable compliance state” interaction model

What to avoid borrowing:
- Their domain ontology wholesale
- Their screen and workflow semantics
- Their data model assumptions around tenancy/workflows

## B) Eval regression harness capability (adopt)

Why adopt:
- Immediate beta value: move trust checks into developer CI loops now.
- Existing ecosystem momentum and ready-made primitives.
- Can be safely wrapped with an internal adapter/port to preserve optionality.

Adoption guardrails:
- No framework-specific constructs leaked into domain layer.
- All eval execution via `EvalHarnessPort`.
- Store canonical results in Anvil schema, not provider-native schema.
- Include migration test proving adapter swap feasibility.

---

## Delivery Order

1. Adopt eval harness (MVP integration)
2. Build compliance evidence workspace (borrowed pattern, Anvil-native model)
3. Add contextual policy assertion layer
4. Add IO risk scanner controls
5. Add gateway control-plane deployment patterns
