# Edda API Contracts & Integration Points

**Version:** 1.0.0 **Status:** Draft **Related:**
`/docs/architecture/edda-system-architecture.md` (Section 8)

---

## Overview

This document specifies all API contracts for Edda, including:

1. REST API (HTTP/JSON)
2. CLI Commands (anvil edda)
3. Port Interfaces (TypeScript)
4. External Integration Contracts

---

## 1. REST API Specification

### 1.1 Base Endpoint

```
https://edda.example.com/api/v1
```

### 1.2 Authentication

All API requests require authentication via Bearer token:

```
Authorization: Bearer <jwt-token>
```

**JWT Claims:**

```json
{
  "sub": "user:alice",
  "type": "human",
  "roles": ["org:admin"],
  "exp": 1705329600,
  "iat": 1705326000
}
```

### 1.3 Error Responses

**Standard Error Format:**

```json
{
  "error": {
    "code": "PERMISSION_DENIED",
    "message": "Principal user:bob lacks permission: update_memory",
    "details": {
      "required_permission": "update_memory",
      "principal": "user:bob"
    }
  }
}
```

**Error Codes:**

- `UNAUTHORIZED` (401)
- `PERMISSION_DENIED` (403)
- `NOT_FOUND` (404)
- `VALIDATION_ERROR` (400)
- `CONFLICT` (409)
- `RATE_LIMIT_EXCEEDED` (429)
- `INTERNAL_ERROR` (500)

---

## 2. Memory Endpoints

### 2.1 List Memories

**GET /memories**

Query memories with filters.

**Query Parameters:**

```typescript
{
  types?: string[]                 // decision,pattern,warning
  statuses?: string[]              // active,superseded,retired
  tags?: string[]                  // security,database
  scope_type?: string              // team,project,global
  scope_identifier?: string        // team:platform
  min_confidence?: string          // high,medium,low
  visibility?: string[]            // public,team,private
  created_after?: string           // ISO8601
  created_before?: string          // ISO8601
  search?: string                  // Full-text search
  limit?: number                   // Default: 50, max: 200
  offset?: number                  // Default: 0
  sort_by?: string                 // created_at,updated_at,confidence
  sort_order?: string              // asc,desc (default: desc)
}
```

**Response:**

```json
{
  "memories": [
    {
      "id": "EDDA-M-decision-01h2...",
      "type": "decision",
      "status": "active",
      "statement": "Use TypeScript for all new backend services",
      "context": {
        "when": "For new services started after 2025-01-01",
        "why": "Type safety reduces runtime errors",
        "conditions": ["Applies to new services only"],
        "tags": ["backend", "typescript", "standards"]
      },
      "confidence": "high",
      "scope": {
        "type": "project",
        "identifier": "anvil"
      },
      "authority": {
        "owner": {
          "type": "team",
          "identifier": "team:platform"
        },
        "visibility": "public"
      },
      "enforcement": {
        "mode": "warning"
      },
      "created_at": "2025-01-15T10:00:00Z",
      "updated_at": "2025-01-15T10:00:00Z"
    }
  ],
  "total_count": 142,
  "page_info": {
    "has_next_page": true,
    "has_previous_page": false
  },
  "facets": {
    "by_type": {
      "decision": 45,
      "pattern": 38,
      "warning": 22,
      "constraint": 18,
      "doctrine": 12,
      "lesson": 7
    },
    "by_status": {
      "active": 120,
      "superseded": 15,
      "retired": 7
    }
  }
}
```

**Example:**

```bash
curl -H "Authorization: Bearer $TOKEN" \
  "https://edda.example.com/api/v1/memories?types=decision&tags=security&limit=20"
```

---

### 2.2 Get Memory by ID

**GET /memories/:id**

Retrieve a single memory.

**Response:**

```json
{
  "memory": {
    "id": "EDDA-M-decision-01h2...",
    "type": "decision",
    "status": "active",
    "statement": "...",
    "context": { ... },
    "confidence": "high",
    "confidence_rationale": "Ratified by platform team after incident analysis",
    "scope": { ... },
    "authority": { ... },
    "enforcement": { ... },
    "provenance": {
      "ember_source": {
        "proposal_id": "EMBER-P-decision-01h1...",
        "proposal_type": "decision",
        "confidence": 0.85,
        "created_at": "2025-01-14T15:00:00Z"
      },
      "kindling_sources": [
        {
          "observation_id": "KINDLING-OBS-01h0...",
          "kind": "error_recorded",
          "session_id": "SESSION-001"
        }
      ]
    },
    "attribution": {
      "promoted_by": "user:alice",
      "promoted_at": "2025-01-15T10:00:00Z",
      "reason": "Post-incident decision"
    },
    "evolution": {
      "supersedes": [],
      "superseded_by": null
    },
    "review_policy": {
      "strategy": "time_based",
      "interval_days": 90,
      "last_reviewed_at": "2025-01-15T10:00:00Z"
    },
    "created_at": "2025-01-15T10:00:00Z",
    "updated_at": "2025-01-15T10:00:00Z",
    "metadata": {
      "decision_maker": { "type": "team", "identifier": "team:platform" },
      "alternatives_considered": ["Python", "Go"]
    }
  }
}
```

**Errors:**

- `404 NOT_FOUND` - Memory does not exist
- `403 PERMISSION_DENIED` - No access to this memory

---

### 2.3 Search Memories

**POST /memories/search**

Semantic search across memories.

**Request:**

```json
{
  "query": "how should we handle database migrations?",
  "scope": {
    "type": "project",
    "identifier": "anvil"
  },
  "limit": 10,
  "filters": {
    "types": ["decision", "pattern", "lesson"],
    "min_confidence": "medium"
  }
}
```

**Response:**

```json
{
  "memories": [
    {
      "memory": { ... },
      "relevance_score": 0.92,
      "match_explanation": "Matches migration patterns and database decisions"
    }
  ],
  "total_count": 8
}
```

---

### 2.4 Create Memory (Direct)

**POST /memories**

Create a memory directly (requires `create_memory_direct` permission).

**Request:**

```json
{
  "type": "decision",
  "statement": "All database schema changes must use migrations",
  "context": {
    "when": "For all database schema modifications",
    "why": "Direct schema changes cause production incidents",
    "conditions": [
      "Use Prisma migrations",
      "Review migrations before deployment"
    ],
    "tags": ["database", "schema", "migrations"]
  },
  "confidence": "high",
  "scope": {
    "type": "project",
    "identifier": "anvil"
  },
  "authority": {
    "owner": {
      "type": "team",
      "identifier": "team:platform"
    },
    "visibility": "public"
  },
  "enforcement": {
    "mode": "blocking",
    "hooks": ["pre_execution"]
  },
  "review_policy": {
    "strategy": "time_based",
    "interval_days": 180
  },
  "attribution": {
    "reason": "Post-incident policy"
  },
  "metadata": {
    "incident_id": "INC-2025-001"
  }
}
```

**Response:**

```json
{
  "memory": { ... },
  "audit_entry_id": "EDDA-AUDIT-01h3..."
}
```

**Errors:**

- `403 PERMISSION_DENIED` - Lacks `create_memory_direct`
- `400 VALIDATION_ERROR` - Invalid memory structure

---

### 2.5 Update Memory

**PATCH /memories/:id**

Update an existing memory (creates new version).

**Request:**

```json
{
  "statement": "Updated statement...",
  "context": {
    "tags": ["database", "schema", "migrations", "critical"]
  },
  "change_reason": "Added 'critical' tag after recent incident"
}
```

**Response:**

```json
{
  "memory": { ... },
  "version": 2,
  "audit_entry_id": "EDDA-AUDIT-01h4..."
}
```

**Errors:**

- `403 PERMISSION_DENIED` - Not owner or reviewer
- `404 NOT_FOUND` - Memory does not exist
- `409 CONFLICT` - Memory was updated concurrently

---

### 2.6 Retire Memory

**POST /memories/:id/retire**

Mark memory as retired.

**Request:**

```json
{
  "reason": "Replaced by new migration system",
  "superseded_by": "EDDA-M-decision-02h5..."
}
```

**Response:**

```json
{
  "memory": {
    "id": "EDDA-M-decision-01h2...",
    "status": "retired",
    "evolution": {
      "retired_at": "2025-02-01T10:00:00Z",
      "retired_by": "user:alice",
      "retired_reason": "Replaced by new migration system",
      "superseded_by": "EDDA-M-decision-02h5..."
    }
  },
  "audit_entry_id": "EDDA-AUDIT-01h5..."
}
```

---

### 2.7 Get Memory History

**GET /memories/:id/history**

Get version history of a memory.

**Response:**

```json
{
  "memory_id": "EDDA-M-decision-01h2...",
  "current_version": 3,
  "versions": [
    {
      "version": 1,
      "snapshot": { ... },
      "change_type": "created",
      "changed_by": { "type": "human", "identifier": "user:alice" },
      "changed_at": "2025-01-15T10:00:00Z",
      "change_reason": "Initial creation"
    },
    {
      "version": 2,
      "snapshot": { ... },
      "change_type": "updated",
      "changed_by": { "type": "human", "identifier": "user:bob" },
      "changed_at": "2025-01-20T14:30:00Z",
      "change_reason": "Added exclusions",
      "diff": {
        "context": {
          "conditions": ["Added: Excludes test utilities"]
        }
      }
    }
  ]
}
```

---

### 2.8 Get Provenance

**GET /memories/:id/provenance**

Trace the full provenance chain.

**Query Parameters:**

```typescript
{
  include_kindling?: boolean   // Include source observations
  include_versions?: boolean   // Include version history
}
```

**Response:**

```json
{
  "memory": { ... },
  "chain": {
    "ember_source": {
      "proposal_id": "EMBER-P-decision-01h1...",
      "proposal": { ... }
    },
    "kindling_sources": [
      {
        "observation_id": "KINDLING-OBS-01h0...",
        "observation": { ... }
      }
    ]
  },
  "versions": [ ... ],
  "graph": {
    "nodes": [
      {
        "id": "KINDLING-OBS-01h0...",
        "type": "observation",
        "label": "Error recorded"
      },
      {
        "id": "EMBER-P-decision-01h1...",
        "type": "proposal",
        "label": "Decision proposed"
      },
      {
        "id": "EDDA-M-decision-01h2...",
        "type": "memory",
        "label": "Decision ratified"
      }
    ],
    "edges": [
      {
        "from": "KINDLING-OBS-01h0...",
        "to": "EMBER-P-decision-01h1...",
        "relationship": "observed"
      },
      {
        "from": "EMBER-P-decision-01h1...",
        "to": "EDDA-M-decision-01h2...",
        "relationship": "promoted"
      }
    ]
  }
}
```

---

## 3. Promotion Endpoints

### 3.1 List Pending Promotions

**GET /promotions**

List promotion requests awaiting review.

**Query Parameters:**

```typescript
{
  status?: string[]            // awaiting_review,under_review,approved,rejected
  priority?: string            // low,normal,high
  limit?: number
  offset?: number
}
```

**Response:**

```json
{
  "promotions": [
    {
      "id": "EDDA-PR-01h6...",
      "proposal_id": "EMBER-P-pattern-01h5...",
      "status": "awaiting_review",
      "proposed_memory": { ... },
      "requested_by": { "type": "agent", "identifier": "agent:anvil" },
      "requested_at": "2025-01-16T09:00:00Z",
      "priority": "normal"
    }
  ],
  "total_count": 12
}
```

---

### 3.2 Get Promotion Diff

**GET /promotions/:id/diff**

Get transformation diff for review.

**Response:**

```json
{
  "proposal": {
    "id": "EMBER-P-pattern-01h5...",
    "type": "pattern",
    "confidence": 0.78,
    "summary": "Use async/await for all async operations"
  },
  "memory": { ... },
  "transformations": {
    "type_mapping": "pattern → pattern",
    "confidence_mapping": "0.78 → medium",
    "scope_inference": "Inferred from observation sessions: project:anvil",
    "enforcement_recommendation": "advisory (pattern typically not blocking)"
  },
  "conflicts": [
    {
      "memory_id": "EDDA-M-pattern-01h1...",
      "conflict_type": "duplication",
      "severity": "medium",
      "explanation": "Similar pattern exists: 'Modern async patterns'"
    }
  ],
  "provenance_summary": "Based on 15 observations across 5 sessions"
}
```

---

### 3.3 Submit Review

**POST /promotions/:id/review**

Approve, reject, or request revisions.

**Request:**

```json
{
  "decision": "approve",
  "rationale": "Good pattern, well-evidenced. Adjusted scope to backend only.",
  "modifications": {
    "scope": {
      "type": "domain",
      "identifier": "backend",
      "exclusions": ["frontend"]
    },
    "context": {
      "tags": ["async", "backend", "node"]
    }
  }
}
```

**Response:**

```json
{
  "promotion": {
    "id": "EDDA-PR-01h6...",
    "status": "approved",
    "reviewer": { "type": "human", "identifier": "user:alice" },
    "review_completed_at": "2025-01-16T10:30:00Z"
  },
  "memory": { ... },
  "audit_entry_id": "EDDA-AUDIT-01h7..."
}
```

---

### 3.4 Reject Promotion

**POST /promotions/:id/reject**

**Request:**

```json
{
  "reason_category": "insufficient_evidence",
  "explanation": "Only 3 observations, need more data points before promoting",
  "false_positive": false,
  "insufficient_evidence": true
}
```

**Response:**

```json
{
  "promotion": {
    "id": "EDDA-PR-01h6...",
    "status": "rejected"
  },
  "rejection": {
    "rejection_id": "EDDA-REJ-01h8...",
    "rejected_by": { "type": "human", "identifier": "user:alice" },
    "rejected_at": "2025-01-16T10:30:00Z",
    "reason_category": "insufficient_evidence"
  },
  "feedback": {
    "adjust_confidence_by": -0.15,
    "rationale": "Require 10+ observations for pattern promotion"
  },
  "audit_entry_id": "EDDA-AUDIT-01h9..."
}
```

---

## 4. Enforcement Endpoints

### 4.1 List Hooks

**GET /hooks**

List enforcement hooks.

**Query Parameters:**

```typescript
{
  types?: string[]             // pre_execution,validation,guidance
  enabled?: boolean
  priority_min?: number
  priority_max?: number
}
```

**Response:**

```json
{
  "hooks": [
    {
      "hook_id": "EDDA-HOOK-db-migration",
      "type": "pre_execution",
      "name": "Require Migration for Schema Changes",
      "description": "Block direct database schema changes",
      "trigger": {
        "event": "action_about_to_execute",
        "conditions": [
          {
            "field": "action.command",
            "operator": "matches",
            "value": "ALTER TABLE|DROP TABLE"
          }
        ]
      },
      "applicable_memories": {
        "types": ["constraint"],
        "tags": ["database", "schema"],
        "enforcement_modes": ["blocking"]
      },
      "action": {
        "mode": "block",
        "message_template": "..."
      },
      "enabled": true,
      "priority": 100
    }
  ],
  "total_count": 23
}
```

---

### 4.2 Create Hook

**POST /hooks**

Register a new enforcement hook.

**Request:**

```json
{
  "type": "pre_execution",
  "name": "Require Security Review",
  "description": "Require security team approval for auth changes",
  "trigger": {
    "event": "file_about_to_change",
    "conditions": [
      {
        "field": "file.path",
        "operator": "matches",
        "value": "^src/auth/.*\\.ts$"
      }
    ]
  },
  "applicable_memories": {
    "types": ["doctrine"],
    "tags": ["security", "authentication"]
  },
  "action": {
    "mode": "require_approval",
    "message_template": "Authentication changes require security review",
    "approval_required_from": ["team_lead", "org_admin"]
  },
  "enabled": true,
  "priority": 150
}
```

**Response:**

```json
{
  "hook": { ... },
  "audit_entry_id": "EDDA-AUDIT-01ha..."
}
```

---

### 4.3 Check Action (Dry Run)

**POST /hooks/check**

Test what would happen if an action executes.

**Request:**

```json
{
  "event": "action_about_to_execute",
  "context": {
    "principal": { "type": "human", "identifier": "user:bob" },
    "action": {
      "action_type": "shell_command",
      "action_details": {
        "command": "ALTER TABLE users ADD COLUMN email VARCHAR(255)"
      }
    },
    "scope": {
      "type": "project",
      "identifier": "anvil"
    }
  }
}
```

**Response:**

```json
{
  "allowed": false,
  "violations": [
    {
      "action": "block",
      "memory": {
        "id": "EDDA-M-constraint-01h1...",
        "statement": "All database schema changes must use migrations"
      },
      "message": "Blocked: Direct database schema modification\n\nSchema changes must go through migration system...",
      "can_override": false
    }
  ],
  "warnings": [],
  "suggestions": []
}
```

---

### 4.4 Request Guidance

**POST /hooks/guidance**

Get contextual guidance during planning.

**Request:**

```json
{
  "context": {
    "intent": "implement user authentication with JWT",
    "scope": {
      "type": "project",
      "identifier": "anvil"
    },
    "technologies": ["node", "express", "jwt"],
    "current_phase": "planning"
  },
  "limit": 5
}
```

**Response:**

```json
{
  "relevant_memories": [
    {
      "memory": {
        "id": "EDDA-M-pattern-auth-01...",
        "type": "pattern",
        "statement": "Use refresh tokens for long-lived sessions"
      },
      "relevance_score": 0.95,
      "why_relevant": "Directly related to JWT authentication implementation",
      "when_to_apply": "During authentication token generation"
    }
  ],
  "patterns_to_consider": [ ... ],
  "warnings_to_avoid": [ ... ],
  "lessons_learned": [ ... ]
}
```

---

## 5. Authority Endpoints

### 5.1 List Roles

**GET /roles**

**Response:**

```json
{
  "roles": [
    {
      "role_id": "org:admin",
      "name": "Organisation Administrator",
      "authority_level": "org_admin",
      "permissions": [
        "read_all",
        "review_promotions",
        "create_memory_direct",
        "update_memory",
        "retire_memory"
      ],
      "principals": [{ "type": "human", "identifier": "user:alice" }]
    }
  ]
}
```

---

### 5.2 Assign Role

**POST /roles/:role_id/principals**

**Request:**

```json
{
  "principal": {
    "type": "human",
    "identifier": "user:charlie"
  },
  "expires_at": "2025-12-31T23:59:59Z"
}
```

**Response:**

```json
{
  "assignment": {
    "assignment_id": "ASGN-01hb...",
    "role_id": "team:lead",
    "principal": { ... },
    "assigned_by": { "type": "human", "identifier": "user:alice" },
    "assigned_at": "2025-01-16T11:00:00Z",
    "expires_at": "2025-12-31T23:59:59Z"
  },
  "audit_entry_id": "EDDA-AUDIT-01hc..."
}
```

---

### 5.3 Get Agent Trust Profile

**GET /agents/:agent_id/trust**

**Response:**

```json
{
  "trust_profile": {
    "profile_id": "TP-anvil-001",
    "agent_id": "agent:anvil",
    "trust_score": 0.78,
    "proposals_submitted": 145,
    "proposals_approved": 98,
    "proposals_rejected": 47,
    "approval_rate": 0.676,
    "factors": [
      {
        "factor": "historical_accuracy",
        "weight": 0.4,
        "current_value": 0.68,
        "rationale": "67.6% approval rate"
      },
      {
        "factor": "source_quality",
        "weight": 0.3,
        "current_value": 0.85,
        "rationale": "High-quality observations"
      }
    ],
    "can_propose": true,
    "confidence_adjustment": 0.056,
    "requires_human_review": true,
    "last_updated": "2025-01-16T10:00:00Z"
  }
}
```

---

### 5.4 Query Audit Log

**GET /audit**

**Query Parameters:**

```typescript
{
  principal?: string           // user:alice
  operation?: string           // memory_created,promotion_approved
  target_type?: string         // memory,promotion,authority
  target_id?: string
  from?: string                // ISO8601
  to?: string                  // ISO8601
  limit?: number
  offset?: number
}
```

**Response:**

```json
{
  "entries": [
    {
      "audit_id": "EDDA-AUDIT-01h3...",
      "timestamp": "2025-01-15T10:00:00Z",
      "principal": { "type": "human", "identifier": "user:alice" },
      "authority_level": "org_admin",
      "operation": "memory_created",
      "target_type": "memory",
      "target_id": "EDDA-M-decision-01h2...",
      "rationale": "Post-incident policy",
      "session_id": "SESSION-042"
    }
  ],
  "total_count": 1523
}
```

---

## 6. Export/Import Endpoints

### 6.1 Export Memories

**GET /export**

**Query Parameters:**

```typescript
{
  format?: string              // json,yaml,markdown (default: json)
  query?: EddaQuery            // Filter what to export
  include_provenance?: boolean
}
```

**Response (JSON):**

```json
{
  "export_id": "EXPORT-01hd...",
  "exported_at": "2025-01-16T12:00:00Z",
  "exported_by": { "type": "human", "identifier": "user:alice" },
  "memories": [ ... ],
  "provenance_chains": [ ... ],
  "schema_version": "1.0.0",
  "edda_version": "1.0.0"
}
```

**Response (Markdown):**

```markdown
# Edda Memory Export

**Exported:** 2025-01-16T12:00:00Z **Exported by:** user:alice

---

## Decision: Use TypeScript for backend

**Status:** Active **Confidence:** High ...
```

---

### 6.2 Import Memories

**POST /import**

**Request:**

```json
{
  "memories": [ ... ],
  "schema_version": "1.0.0",
  "conflict_strategy": "skip"  // skip,overwrite,merge
}
```

**Response:**

```json
{
  "imported_count": 42,
  "skipped_count": 3,
  "errors": [
    {
      "memory_id": "EDDA-M-decision-old-01...",
      "error_type": "duplicate",
      "message": "Memory with this ID already exists"
    }
  ],
  "created_memory_ids": [ ... ],
  "skipped_memory_ids": [ ... ]
}
```

---

### 6.3 Get Stats

**GET /stats**

**Response:**

```json
{
  "memories": {
    "total": 142,
    "by_type": {
      "decision": 45,
      "pattern": 38,
      "warning": 22,
      "constraint": 18,
      "doctrine": 12,
      "lesson": 7
    },
    "by_status": {
      "active": 120,
      "superseded": 15,
      "retired": 7
    },
    "by_confidence": {
      "high": 65,
      "medium": 58,
      "low": 15,
      "inferred": 4
    }
  },
  "promotions": {
    "total": 189,
    "awaiting_review": 12,
    "approved": 98,
    "rejected": 79
  },
  "hooks": {
    "total": 23,
    "enabled": 18,
    "by_type": {
      "pre_execution": 8,
      "validation": 5,
      "guidance": 10
    }
  },
  "agents": {
    "total": 3,
    "average_trust_score": 0.72,
    "average_approval_rate": 0.68
  }
}
```

---

## 7. CLI Commands

### 7.1 Memory Commands

```bash
# List memories
anvil edda list [--type=<type>] [--tags=<tags>] [--status=<status>]

# Examples:
anvil edda list --type=decision --tags=security
anvil edda list --status=active --limit=10

# Show memory
anvil edda show <memory-id>

# Search memories
anvil edda search "database migration patterns"

# Trace provenance
anvil edda trace <memory-id> [--include-kindling] [--include-versions]

# Create memory (direct)
anvil edda create --type=decision --statement="..." [--file=memory.yaml]

# Update memory
anvil edda update <memory-id> --statement="..." [--reason="..."]

# Retire memory
anvil edda retire <memory-id> --reason="..." [--superseded-by=<id>]
```

---

### 7.2 Promotion Commands

```bash
# List pending promotions
anvil edda proposals [--priority=high]

# Show promotion details
anvil edda proposals show <promotion-id>

# Review promotion (interactive)
anvil edda promote <promotion-id>

# Approve with modifications
anvil edda promote <promotion-id> \
  --approve \
  --reason="Good pattern, adjusted scope" \
  --modify-scope="team:platform"

# Reject promotion
anvil edda reject <promotion-id> \
  --reason="Insufficient evidence" \
  --category=insufficient_evidence
```

---

### 7.3 Enforcement Commands

```bash
# List hooks
anvil edda hooks list [--type=pre_execution] [--enabled=true]

# Create hook
anvil edda hooks create --file=hook.yaml

# Update hook
anvil edda hooks update <hook-id> --enabled=false

# Delete hook
anvil edda hooks delete <hook-id>

# Check action (dry-run)
anvil edda check --action="ALTER TABLE users..." [--principal=user:bob]

# Request guidance
anvil edda guide --intent="implement authentication"

# Request override
anvil edda override --violation=<id> --justification="Emergency hotfix"
```

---

### 7.4 Authority Commands

```bash
# List roles
anvil edda roles list

# Assign role
anvil edda roles assign <principal> <role-id> [--expires=2025-12-31]

# Revoke role
anvil edda roles revoke <principal> <role-id>

# Show trust profile
anvil edda trust <agent-id>

# Query audit log
anvil edda audit [--principal=user:alice] [--days=30] [--operation=memory_created]
```

---

### 7.5 Lifecycle Commands

```bash
# Show memories needing review
anvil edda review-due [--overdue]

# Show stale memories
anvil edda stale [--threshold=90]

# Supersede memory
anvil edda supersede <old-id> <new-id> --reason="..."

# Detect conflicts
anvil edda conflicts
```

---

### 7.6 Export/Import Commands

```bash
# Export memories
anvil edda export [--format=json|yaml|markdown] [--output=export.json]

# Export with filters
anvil edda export --type=decision --tags=security --output=security-decisions.yaml

# Import memories
anvil edda import <file> [--conflict-strategy=skip|overwrite]

# Show stats
anvil edda stats [--detailed]
```

---

### 7.7 Stack Commands (Existing)

```bash
# Show stack status
anvil stack status

# Validate stack configuration
anvil stack validate
```

---

## 8. Port Interfaces (TypeScript)

### 8.1 Extended Edda Port

```typescript
interface IEddaPortExtended extends IEddaPort {
  // Memory operations (extended)
  createMemory(
    input: MemoryObjectInput,
    principal: Principal
  ): Promise<MemoryObjectExtended>;

  updateMemory(
    id: MemoryId,
    patch: MemoryObjectPatch,
    principal: Principal,
    reason: string
  ): Promise<MemoryObjectExtended>;

  retireMemory(
    id: MemoryId,
    reason: string,
    supersededBy: MemoryId | undefined,
    principal: Principal
  ): Promise<MemoryObjectExtended>;

  // Query
  queryMemories(query: EddaQuery): Promise<EddaQueryResult>;
  searchMemories(query: SemanticQuery): Promise<SemanticResult>;
  getEvolutionChain(id: MemoryId): Promise<EvolutionChain>;

  // Provenance
  getProvenance(query: ProvenanceQuery): Promise<ProvenanceResult>;

  // Stats
  getStats(): Promise<EddaStats>;

  // Export/Import
  exportMemories(query?: EddaQuery): Promise<MemoryExport>;
  importMemories(data: MemoryExport): Promise<ImportResult>;
}
```

---

### 8.2 Promotion Port

```typescript
interface IPromotionPort {
  // Request management
  createPromotionRequest(
    proposalId: ProposalId,
    requestedBy: Principal
  ): Promise<PromotionRequest>;

  getPromotionRequest(id: PromotionRequestId): Promise<PromotionRequest>;

  listPendingReviews(filter?: PromotionFilter): Promise<PromotionRequest[]>;

  // Review workflow
  startReview(id: PromotionRequestId, reviewer: Principal): Promise<void>;

  submitReview(
    id: PromotionRequestId,
    review: PromotionReview
  ): Promise<PromotionResult>;

  // Diff generation
  getPromotionDiff(id: PromotionRequestId): Promise<PromotionDiff>;

  // Rejection
  recordRejection(
    proposalId: ProposalId,
    rejection: RejectionRecord
  ): Promise<void>;

  // Stats
  getStats(): Promise<PromotionStats>;
}
```

---

### 8.3 Enforcement Port

```typescript
interface IEnforcementPort {
  // Hook management
  registerHook(hook: EnforcementHook): Promise<void>;
  getHook(id: HookId): Promise<EnforcementHook>;
  updateHook(id: HookId, updates: Partial<EnforcementHook>): Promise<void>;
  deleteHook(id: HookId): Promise<void>;
  listHooks(filter?: HookFilter): Promise<EnforcementHook[]>;

  // Execution
  executeHooks(
    event: HookEvent,
    context: ExecutionContext
  ): Promise<HookExecutionResult>;

  // Override
  requestOverride(request: OverrideRequest): Promise<OverrideDecision>;

  // Guidance
  getGuidance(context: PlanningContext): Promise<GuidanceResponse>;

  // Stats
  getStats(): Promise<EnforcementStats>;
}
```

---

### 8.4 Authority Port

```typescript
interface IAuthorityPort {
  // Principals
  resolvePrincipal(identifier: string): Promise<Principal>;
  registerPrincipal(principal: Principal): Promise<void>;
  getPrincipalInfo(principal: Principal): Promise<PrincipalInfo>;

  // Roles
  assignRole(
    principal: Principal,
    role: Role,
    assignedBy: Principal
  ): Promise<RoleAssignment>;

  revokeRole(
    principal: Principal,
    roleId: string,
    revokedBy: Principal
  ): Promise<void>;

  getRolesForPrincipal(principal: Principal): Promise<Role[]>;

  // Permissions
  hasPermission(
    principal: Principal,
    permission: Permission,
    context?: PermissionContext
  ): boolean;

  canAccessMemory(principal: Principal, memory: MemoryObjectExtended): boolean;

  getAuthorityLevel(principal: Principal): AuthorityLevel;

  // Trust
  getTrustProfile(agentId: string): Promise<AgentTrustProfile>;

  updateTrustProfile(
    agentId: string,
    event: TrustEvent
  ): Promise<AgentTrustProfile>;

  // Audit
  audit(entry: AuditEntry): Promise<void>;
  queryAudit(query: AuditQuery): Promise<AuditQueryResult>;
}
```

---

## 9. External Integration Contracts

### 9.1 Anvil Gate Integration

```typescript
// Anvil provides this interface
interface IAnvilGateSystem {
  /**
   * Register a pre-execution hook
   */
  registerPreExecutionHook(name: string, hook: PreExecutionHook): void;

  /**
   * Register a file change hook
   */
  registerFileChangeHook(name: string, hook: FileChangeHook): void;

  /**
   * Register a planning hook
   */
  registerPlanningHook(name: string, hook: PlanningHook): void;
}

// Edda implements these
type PreExecutionHook = (
  action: Action,
  context: ActionContext
) => Promise<HookExecutionResult>;

type FileChangeHook = (
  file: FileChange,
  context: FileContext
) => Promise<HookExecutionResult>;

type PlanningHook = (
  plan: PlanDraft,
  context: PlanningContext
) => Promise<GuidanceResponse>;
```

**Usage (in Anvil):**

```typescript
// Anvil registers Edda hooks on startup
const eddaHooks = new EddaEnforcementService(...)

anvilGates.registerPreExecutionHook('edda-enforcement', async (action, context) => {
  return await eddaHooks.executeHooks('action_about_to_execute', {
    principal: context.principal,
    action: {
      action_type: action.type,
      action_details: action.parameters
    },
    scope: context.scope
  })
})
```

---

### 9.2 Identity Provider Integration

```typescript
// Identity provider must implement
interface IIdentityProvider {
  /**
   * Authenticate user from token
   */
  authenticate(token: string): Promise<Principal>;

  /**
   * Get principal information
   */
  getPrincipalInfo(identifier: string): Promise<PrincipalInfo>;

  /**
   * List team members
   */
  getTeamMembers(teamId: string): Promise<Principal[]>;

  /**
   * Check if principal is in team
   */
  isMemberOfTeam(principal: Principal, teamId: string): Promise<boolean>;
}

interface PrincipalInfo {
  identifier: string;
  name: string;
  email?: string;
  teams: string[];
  metadata: Record<string, unknown>;
}
```

**Implementations:**

- GitHub OAuth provider
- LDAP/Active Directory provider
- Custom JWT provider

---

### 9.3 Embedding Service Integration (Optional)

```typescript
// For semantic search
interface IEmbeddingService {
  /**
   * Generate embedding for text
   */
  embed(text: string): Promise<number[]>; // Vector

  /**
   * Find similar texts
   */
  findSimilar(
    query: string,
    candidates: string[],
    limit: number
  ): Promise<SimilarityResult[]>;
}

interface SimilarityResult {
  text: string;
  similarity: number; // 0.0 - 1.0
}
```

**Implementations:**

- Ollama provider (local)
- OpenAI provider (API)
- HuggingFace provider (API)

---

## 10. Webhook Support (Future)

### 10.1 Webhook Events

```typescript
type WebhookEvent =
  | 'memory.created'
  | 'memory.updated'
  | 'memory.retired'
  | 'promotion.created'
  | 'promotion.approved'
  | 'promotion.rejected'
  | 'enforcement.violated'
  | 'enforcement.overridden';
```

### 10.2 Webhook Payload

```json
{
  "event": "memory.created",
  "timestamp": "2025-01-16T12:00:00Z",
  "data": {
    "memory": { ... },
    "principal": { "type": "human", "identifier": "user:alice" }
  },
  "webhook_id": "WH-01he..."
}
```

---

## Summary

This document defines all public APIs for Edda:

- **REST API** for programmatic access (Phase 6)
- **CLI** for human interaction (Phases 1-6)
- **Port Interfaces** for TypeScript integration (All phases)
- **External Integrations** for Anvil, Identity, and optional services

All APIs follow consistent patterns:

- Authentication via Bearer tokens
- Standard error responses
- Audit logging for mutations
- Permission checks enforced

---

**Next:** See implementation phases in `/plans/edda-phase-breakdown.md`
