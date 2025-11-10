---
description: Generate a PRD and contract-first API spec
---

# Full Specification Generation

Creates comprehensive specifications for complex features:

1. **Product Requirements** - Detailed PRD with user research, use cases, and
   acceptance criteria
2. **System Architecture** - Technical design with interfaces and data modelling
3. **Data Modelling** - Schema design, migrations, and PII considerations

## Agent Sequence

- **product-manager**: Produces detailed PRD using
  `.claude/docs-templates/PRD.md` template
- **architect**: Creates system design with interface contracts, preferring
  TypeScript examples
- **data-modeller**: Designs schemas, indexes, migration steps, and flags PII
  concerns

## Usage

```
/full-spec "multi-tenant SaaS billing system"
```

Produces contract-first specifications that can guide implementation teams and
stakeholders.

## Output Artifacts

- Complete PRD document
- API interface specifications
- Database schema and migration plans
- Architecture decision records (ADRs)
