# Architecture Decision Records (ADR)

**Last Updated:** 2025-10-23

## What are ADRs?

Architecture Decision Records (ADRs) document significant architectural
decisions made during development, including:

- The context and problem
- Considered alternatives
- The decision made
- Consequences and trade-offs

## Active ADRs

### [ADR-0001: Use Zod for APS Schema Definition](0001-use-zod-for-aps-schema-definition.md)

**Date:** 2025-09-30 **Status:** Accepted **Decision:** Use Zod for schema
validation instead of JSON Schema

**Key Points:**

- TypeScript-first with automatic type inference
- Runtime validation with rich error messages
- Single source of truth for both types and validation
- Better developer experience

## ADR Process

### Creating a New ADR

1. **Copy template** from `.claude/docs-templates/ADR.md`
2. **Number sequentially** (next would be ADR-0002)
3. **Fill in all sections**:
   - Status (Proposed/Accepted/Rejected/Superseded)
   - Context (problem being solved)
   - Decision (what was decided)
   - Consequences (trade-offs)
   - Alternatives Considered

4. **Get review** before marking as Accepted
5. **Update this README** with a summary

### ADR Statuses

- **Proposed** - Under discussion
- **Accepted** - Decision is final and being implemented
- **Rejected** - Alternative was chosen
- **Superseded** - Replaced by a newer ADR

## Navigation

- [Back to Documentation Index](../INDEX.md)
- [Architecture Document](../../ARCHITECTURE.md)
