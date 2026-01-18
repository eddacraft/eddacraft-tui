# ADR-0001: Use Zod for APS Schema Definition

## Status

Accepted

## Context

The Anvil Plan Specification (APS) requires a robust schema definition system
that provides:

- Runtime validation with clear error messages
- TypeScript type safety and inference
- Ability to export to JSON Schema for external tooling
- Maintainable and developer-friendly API
- Single source of truth for schema and types

We evaluated two approaches:

1. **JSON Schema First**: Define schema in JSON Schema format, generate
   TypeScript types
2. **Zod First**: Define schema using Zod (TypeScript), export JSON Schema when
   needed

## Decision

We will use **Zod as the primary schema definition** for APS, with the ability
to export to JSON Schema when needed.

## Rationale

### Advantages of Zod-First Approach

1. **Type Safety and Inference**
   - TypeScript types are automatically inferred from Zod schemas using
     `z.infer<typeof schema>`
   - No code generation step required - types stay in sync automatically
   - Better IDE support with autocomplete and type checking

2. **Runtime Validation**
   - Zod provides runtime validation out of the box
   - Error messages are more user-friendly than JSON Schema/Ajv errors
   - Built-in transformations and refinements for complex validation logic

3. **Developer Experience**
   - TypeScript-native API that feels natural to TS developers
   - Composable schemas that can be built from smaller pieces
   - Easier to read and maintain than JSON Schema

4. **Single Source of Truth**
   - Schema definition and TypeScript types come from the same source
   - No synchronization issues between schema and types
   - Reduces potential for bugs from mismatched definitions

5. **Flexibility**
   - Can export to JSON Schema using Zod v4's native `z.toJSONSchema()` function
   - Supports advanced features like branding, transforms, and custom error
     messages
   - Can still use Ajv for JSON Schema validation if required for compatibility

### Disadvantages Considered

1. **Additional Dependency**
   - Adds Zod as a runtime dependency
   - Mitigation: Zod is lightweight (~8KB gzipped) and well-maintained

2. **JSON Schema Export Limitations**
   - Not all Zod features translate perfectly to JSON Schema
   - Mitigation: Core validation rules we need are well-supported

3. **Learning Curve**
   - Developers need to learn Zod API
   - Mitigation: Zod has excellent documentation and is widely adopted

## Implementation

### File Structure

```
core/src/schema/
├── aps.schema.ts       # Zod schema definition (primary source)
├── index.ts           # Exports schema, types, and utilities
└── aps.schema.json    # Generated JSON Schema (CI artifact)
```

### Example Implementation

```typescript
// aps.schema.ts
import { z } from 'zod';

// Define schema with Zod
export const APSPlanSchema = z
  .object({
    id: z.string().regex(/^aps-[a-f0-9]{8}$/),
    hash: z.string().regex(/^[a-f0-9]{64}$/),
    intent: z.string().min(10).max(500),
    schema_version: z.literal('0.1.0'),
    proposed_changes: z.array(ChangeSchema),
    provenance: ProvenanceSchema,
    validations: ValidationSchema,
    evidence: z.array(EvidenceSchema).optional(),
  })
  .brand<'APSPlan'>();

// Infer TypeScript type
export type APSPlan = z.infer<typeof APSPlanSchema>;

// Export JSON Schema when needed (using Zod v4 native converter)
import * as z from 'zod';
export const APSPlanJSONSchema = z.toJSONSchema(APSPlanSchema, {
  target: 'draft-7',
});
```

### Validation Usage

```typescript
// Using Zod for validation
const result = APSPlanSchema.safeParse(unknownData);
if (!result.success) {
  // Zod provides detailed, formatted errors
  console.error(result.error.format());
} else {
  // result.data is fully typed as APSPlan
  const plan: APSPlan = result.data;
}
```

## Consequences

### Positive

- TypeScript types automatically stay in sync with schema
- Better developer experience with TypeScript-first approach
- More maintainable codebase with single source of truth
- Improved error messages for end users
- Can still provide JSON Schema for external tooling

### Negative

- Adds Zod as a core dependency
- Team needs to learn Zod API (minimal learning curve)
- JSON Schema is generated rather than hand-crafted

### Neutral

- JSON Schema becomes a build artifact rather than source
- Validation logic moves from Ajv to Zod (can keep both if needed)

## References

- [Zod Documentation](https://zod.dev)
- [Zod v4 JSON Schema Documentation](https://zod.dev/json-schema)
- [TypeScript Handbook - Type Inference](https://www.typescriptlang.org/docs/handbook/type-inference.html)
- [JSON Schema Specification](https://json-schema.org/)

## Decision Date

2025-09-23

## Participants

- Architecture Team
- Development Team

## Amendment History

- 2025-09-23: Initial decision documented
