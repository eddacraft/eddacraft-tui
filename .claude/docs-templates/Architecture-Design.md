# Architecture Design Document

## 1. Scope & Risks

### In Scope

- [List specific features/components to be built]

### Out of Scope

- [List explicitly excluded items]

### Risks & Unknowns

- ⚠️ **[Risk category]**: [Description and impact]
- 🔍 **Unknown**: [What needs investigation]
- ⏱️ **Performance**: [Scalability concerns]

## 2. Interface Contract

### REST API Example

```typescript
// HTTP endpoints
POST /api/v1/resource/:id/action
Request: ActionDto { field1: string, field2?: number }
Response: ResourceDto { id, field1, field2, updatedAt }
```

### Service Interface Example

```typescript
interface ResourceService {
  performAction(id: string, data: ActionDto): Promise<Resource>;
  validateInput(data: ActionDto): ValidationResult;
  // Events: 'resource.created', 'resource.updated'
}
```

### GraphQL Example

```graphql
type Mutation {
  updateResource(id: ID!, input: ResourceInput!): Resource!
}
```

## 3. Data & State

### Database Schema

```sql
-- Schema changes
ALTER TABLE resources ADD COLUMN new_field VARCHAR(255);
CREATE INDEX idx_resources_new_field ON resources(new_field);
```

### Cache Strategy

```yaml
Key Pattern: resource:${id}
TTL: 3600 seconds
Invalidation: On resource.updated event
```

### State Management

```typescript
// Frontend state shape
interface AppState {
  resources: Record<string, Resource>;
  loading: boolean;
  error: string | null;
}
```

### PII & Compliance

⚠️ **PII Fields**: [List fields containing personal data]

- Encryption: [at rest/in transit requirements]
- Retention: [data retention policy]
- GDPR/CCPA: [compliance requirements]

## 4. Files to Touch

```yaml
Added:
  - src/[module]/[feature]/ - [feature].controller.ts - [feature].service.ts -
    [feature].dto.ts - [feature].test.ts

Modified:
  - src/routes/index.ts # Register new routes
  - src/entities/[entity].ts # Add new fields
  - package.json # New dependencies (if any)

Deleted:
  - src/legacy/[old-feature].js # If replacing
```

## 5. Guardrails & Acceptance Criteria

### Functional Requirements

- ✓ [Specific behaviour requirement]
- ✓ [Input validation rule]
- ✓ [Business logic constraint]

### Non-Functional Requirements

- **Performance**: Response time < 100ms p95
- **Scale**: Support 1000 concurrent users
- **Rate Limit**: 100 requests/minute per user
- **Availability**: 99.9% uptime SLO

### Security Requirements

- ✓ Input sanitisation (XSS, SQLi)
- ✓ Authentication required
- ✓ Authorization: [role-based/attribute-based]
- ✓ Audit logging for [sensitive operations]

### Testing Requirements

- Unit tests: Service logic, validators
- Integration tests: API endpoints, database
- E2E tests: Critical user flows
- Performance tests: Load testing for [specific scenarios]

## 6. Architecture Patterns

### Current Stack Patterns

<!-- Document discovered patterns from codebase -->

- **API Pattern**: [Controller → Service → Repository]
- **Validation**: [DTOs with class-validator]
- **Error Handling**: [Centralised error middleware]
- **Logging**: [Structured logging with correlation IDs]

### Design Decisions

- **Why [Pattern X]**: [Rationale based on existing code]
- **Alternative Considered**: [What else was evaluated]
- **Trade-offs**: [Pros/cons of chosen approach]

## 7. Handoffs

### → Coder

**Implementation Notes:**

- Start with: [specific file/component]
- Use existing: [pattern/utility] from [location]
- Dependencies: [required libraries already in project]
- Watch out for: [common pitfalls]

**Code Examples to Follow:**

- Similar feature: `src/modules/[example]/`
- Pattern reference: `src/core/[pattern]/`

### → Tester

**Test Focus Areas:**

- Edge cases: [specific scenarios]
- Security: [specific attack vectors to test]
- Performance: [specific bottlenecks to verify]
- Integration: [external service behaviour]

**Test Data Requirements:**

- Valid inputs: [examples]
- Invalid inputs: [examples]
- Boundary conditions: [examples]

### → Reviewer

**Review Checklist:**

- [ ] Follows existing patterns
- [ ] No PII in logs
- [ ] Error handling complete
- [ ] Performance considerations addressed
- [ ] Security requirements met

## 8. Migration & Rollout

### Migration Strategy

- [ ] Backward compatible
- [ ] Feature flag: `ENABLE_NEW_FEATURE`
- [ ] Data migration script required
- [ ] Rollback plan documented

### Monitoring & Observability

- **Metrics**: [what to measure]
- **Alerts**: [threshold and conditions]
- **Dashboards**: [key indicators]
- **Logs**: [important events to track]

## 9. Related Documentation

- ADR: [Link to Architecture Decision Record if created]
- API Docs: [Link to OpenAPI/Swagger]
- Runbook: [Link to operational guide]
- Original Requirement: [Link to PRD/ticket]
