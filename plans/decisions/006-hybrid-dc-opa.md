# ADR-006: Hybrid Dependency-Cruiser + OPA Architecture

## Status

Accepted

## Date

2025-12-31

## Context

Anvil needs to enforce architecture boundaries and evaluate policies. Two
systems currently exist:

### Dependency-Cruiser (DC)

- Static analysis tool for JavaScript/TypeScript dependency graphs
- Detects circular dependencies, layer violations, orphaned modules
- Mature, battle-tested (v16.x)
- Configuration via `.dependency-cruiser.js`
- Already integrated in Anvil's ArchitectureCheck

### Open Policy Agent (OPA)

- CNCF-graduated policy engine
- Evaluates Rego policies against structured input
- Industry standard for policy-as-code
- Already integrated in Anvil's PolicyCheck
- Supports custom business rules, change scope limits, security reviews

### The Gap

These systems operate independently:

- ArchitectureCheck runs DC, produces warnings
- PolicyCheck runs OPA, produces violations
- No data flows between them
- Users must configure both separately

### Options Considered

1. **DC Only** — Use dependency-cruiser for everything
   - Pro: Single tool, simpler
   - Con: DC can't express business rules (coverage thresholds, change scope)
   - Con: DC config is JavaScript, not portable

2. **OPA Only** — Build custom dependency analyser, evaluate all rules in Rego
   - Pro: Unified policy language, testable
   - Con: Must build accurate TypeScript import analyser
   - Con: Significant effort to match DC's accuracy

3. **Hybrid** — DC for static analysis, OPA for policy evaluation, bridge
   between
   - Pro: Best of both worlds
   - Pro: Users without OPA still get DC benefits
   - Pro: DC results inform OPA policies
   - Con: Two systems to maintain

## Decision

**Use hybrid approach: DC for static analysis, OPA for policy evaluation, with
DC results fed into OPA input.**

```
┌───────────────┐      ┌───────────────┐
│ dependency-   │      │ OPA/Rego      │
│ cruiser       │─────▶│ Policies      │
│               │      │               │
│ - Dep graph   │      │ - Biz rules   │
│ - Cycles      │      │ - Scope       │
│ - Layers      │      │ - Security    │
│ - Orphans     │      │ - Architecture│
└───────────────┘      └───────────────┘
        │                      │
        │  OPA disabled        │  OPA enabled
        ▼                      ▼
┌─────────────────────────────────────┐
│          Unified Warnings           │
└─────────────────────────────────────┘
```

## Rationale

### 1. Incremental Adoption

Not all users want or need OPA. The hybrid approach allows:

- **Without OPA:** Full DC analysis, architecture boundary enforcement
- **With OPA:** DC analysis + custom policies + DC-informed Rego rules

This matches Anvil's "planless-first" philosophy — value without prerequisites.

### 2. Leverage Existing Strengths

| Capability                 | Best Tool |
| -------------------------- | --------- |
| TypeScript import parsing  | DC        |
| Circular dependency detect | DC        |
| Layer violation detection  | DC        |
| Orphaned module detection  | DC        |
| Business rule evaluation   | OPA       |
| Change scope enforcement   | OPA       |
| Security review policies   | OPA       |
| Architecture rule testing  | OPA       |
| Remote policy distribution | OPA       |

Neither tool does everything well. Combining them provides comprehensive
coverage.

### 3. Future-Proofing

The hybrid approach allows:

- Replacing DC with custom TypeScript analyser later (Phase E)
- Auto-generating Rego from architecture.yaml (single source of truth)
- Adding new policy types without changing DC integration
- Enterprise policy bundles for centralised governance

### 4. User-Defined Architecture

With YAML-first architecture definition:

```yaml
# .anvil/architecture.yaml
template: hexagonal
layers:
  domain:
    paths: ['src/domain/**']
  infrastructure:
    paths: ['src/infrastructure/**']
    depends_on: [domain]
```

Generates both:

- `.anvil/dependency-cruiser.js` — DC rules for static analysis
- `.anvil/policies/.generated/architecture.rego` — OPA policies for evaluation

Users define architecture once, both systems enforce it.

## Consequences

### Positive

- Users without OPA still get full architecture enforcement
- DC accuracy for import/dependency analysis
- OPA flexibility for business rules
- Single architecture definition (YAML) for both systems
- Clear upgrade path: DC → DC+OPA → Custom analyser+OPA

### Negative

- Two policy systems to understand (DC rules + Rego)
- Generated files require sync management
- Slightly more complex codebase

### Mitigations

- Auto-generate DC config from YAML (users don't write DC rules)
- Auto-generate Rego from YAML (users don't write architecture Rego)
- Hash-based change detection for regeneration
- Clear documentation distinguishing DC vs OPA roles

## Implementation

1. **Phase A:** Architecture YAML schema, parser, DC generator
2. **Phase B:** DC → OPA bridge (inject architecture context)
3. **Phase C:** Rego generator from YAML
4. **Phase D:** Architecture templates (Layered, Hexagonal, Clean, DDD)
5. **Phase E:** Custom TS analyser (optional DC replacement)
6. **Phase F:** Remote policy bundles

See
[OPA & Architecture Integration](../modules/opa-architecture-integration.aps.md)
for full task breakdown.

## References

- [Dependency-Cruiser](https://github.com/sverweij/dependency-cruiser)
- [Open Policy Agent](https://www.openpolicyagent.org/)
- [OPA Rego Language](https://www.openpolicyagent.org/docs/latest/policy-language/)
- [Existing OPA planning doc](../../docs/planning/opa-policy-engine.md)
