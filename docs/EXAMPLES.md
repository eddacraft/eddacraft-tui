# Anvil Examples

Real-world examples and workflows for using Anvil in your development process.

## Table of Contents

- [Basic Examples](#basic-examples)
- [Format-Specific Examples](#format-specific-examples)
- [Team Workflows](#team-workflows)
- [CI/CD Integration](#cicd-integration)
- [Advanced Use Cases](#advanced-use-cases)

## Basic Examples

### Example 1: Validate Your First Plan

You've written a simple plan and want to validate it:

**Input** (`plan.md`):

```markdown
# Feature: Add Dark Mode

## Overview

Add dark mode toggle to application settings.

## Tasks

- [ ] Create theme context provider
- [ ] Add dark mode styles
- [ ] Create settings toggle component
- [ ] Persist user preference
- [ ] Add tests

## Files to Modify

- `src/contexts/ThemeContext.tsx` - Create new
- `src/styles/themes.css` - Create new
- `src/components/Settings.tsx` - Modify
- `src/App.tsx` - Modify to use ThemeProvider
```

**Commands**:

```bash
# Validate the plan
anvil validate plan.md

# Expected output:
# ✓ Detected format: generic (35% confidence)
# ✓ Plan is valid
# ✓ All validation checks passed
```

### Example 2: Run Quality Gates

You want to ensure code quality before implementing:

```bash
# Run all quality gates
anvil gate plan.md

# Expected output:
# ┌──────────┬────────┬─────────┬─────────────────────────────┐
# │ Check    │ Status │ Score   │ Message                     │
# ├──────────┼────────┼─────────┼─────────────────────────────┤
# │ lint     │ ✓ PASS │ 100/100 │ No linting errors found     │
# │ test     │ ✓ PASS │ 100/100 │ All tests passing           │
# │ coverage │ ✓ PASS │  88/100 │ Coverage: 88% (≥80%)        │
# │ secrets  │ ✓ PASS │ 100/100 │ No secrets detected         │
# └──────────┴────────┴─────────┴─────────────────────────────┘
#
# Overall: ✓ PASSED (4/4 checks passed)
```

### Example 3: Convert Between Formats

You have a SpecKit document and want to convert it to APS:

```bash
# Convert SpecKit to APS
anvil export spec.md --to aps --output spec.aps.json

# Convert back to SpecKit
anvil export spec.aps.json --to speckit --output ./output/

# Convert to YAML for review
anvil export spec.md --to yaml --output spec.yaml
```

## Format-Specific Examples

### SpecKit Format Example

**Scenario**: You're planning a new API endpoint using SpecKit format.

**Input** (`spec.md`):

```markdown
# Spec: Add User Profile API

## Authors

- Jane Developer <jane@example.com>

## Status

Draft

## Overview

Create REST API endpoint for retrieving and updating user profiles.

## Requirements

- GET /api/users/:id - Retrieve user profile
- PATCH /api/users/:id - Update user profile
- Authentication required
- Input validation
- Rate limiting (100 req/min per user)

## Plan

### Phase 1: Database Schema

Update user model to include profile fields.

**Files to modify**:

- `prisma/schema.prisma` - Add profile fields
- `src/database/migrations/` - Create migration

**Rationale**: Need to store additional profile data (bio, avatar, social links)

### Phase 2: API Routes

Implement GET and PATCH endpoints.

**Files to create**:

- `src/routes/api/users.ts` - User profile routes
- `src/controllers/user-profile.controller.ts` - Business logic
- `src/validators/user-profile.validator.ts` - Input validation

**Files to modify**:

- `src/routes/index.ts` - Register new routes
- `src/middleware/auth.ts` - Add profile permissions

**Rationale**: Separate concerns (routing, logic, validation)

### Phase 3: Tests

Add comprehensive test coverage.

**Files to create**:

- `src/routes/api/__tests__/users.test.ts`
- `src/controllers/__tests__/user-profile.test.ts`

**Test cases**:

- Authenticated user can retrieve own profile
- Authenticated user can update own profile
- Unauthenticated requests are rejected
- Invalid data is rejected with proper errors
- Rate limiting works correctly

## Tasks

- [ ] Update Prisma schema
- [ ] Create database migration
- [ ] Implement GET endpoint
- [ ] Implement PATCH endpoint
- [ ] Add input validation
- [ ] Add rate limiting
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Update API documentation

## Acceptance Criteria

- All endpoints return proper HTTP status codes
- Input validation prevents invalid data
- Rate limiting enforces 100 req/min limit
- Test coverage >90% for new code
- API documentation is complete
```

**Usage**:

```bash
# Validate SpecKit format
anvil validate spec.md

# Run quality gates
anvil gate spec.md

# Export to APS for archival
anvil export spec.md --to aps --output specs/user-profile-api.aps.json
```

### BMAD Format Example

**Scenario**: You're writing a PRD for a new feature.

**Input** (`prd-notifications.md`):

````markdown
---
title: Real-time Notifications System
version: 1.0.0
status: approved
created: 2025-11-09
author: Product Team
reviewer: Engineering Lead
---

# Real-time Notifications System

## Executive Summary

Implement real-time notifications using WebSockets to improve user engagement
and reduce email volume.

## Problem Statement

Users miss important updates because they're only visible when checking the app.
Email notifications are delayed and create inbox clutter. We need real-time,
in-app notifications for time-sensitive events.

## Goals

- Reduce email notification volume by 60%
- Improve user engagement (daily active users +15%)
- Decrease time-to-action on critical events by 80%

## Non-Goals

- Push notifications (mobile) - planned for Q2 2026
- SMS notifications - out of scope
- Email replacement - emails still needed for critical alerts

## Success Metrics

- 70% of users enable in-app notifications within first week
- Average time-to-action on critical events: <5 minutes (currently 30 min)
- Email notification opt-out rate: 40% (currently 10%)

## Requirements

### Functional Requirements

#### FR-1: Real-time Notification Delivery

**Description**: Deliver notifications to users within 1 second of event
occurrence

**Acceptance Criteria**:

- WebSocket connection established on user login
- Notifications appear without page refresh
- Connection automatically recovers from failures
- Offline notifications queued and delivered on reconnect

#### FR-2: Notification Management

**Description**: Users can view, manage, and configure notifications

**Acceptance Criteria**:

- Notification center shows all notifications (read/unread)
- Mark as read/unread functionality
- Bulk actions (mark all read, clear all)
- Notification preferences per category

#### FR-3: Notification Categories

**Description**: Different notification types for different events

**Categories**:

- Mentions (@username in comments)
- Assignments (tasks assigned to user)
- Approvals (pending approvals requiring action)
- System (maintenance, updates)

### Non-Functional Requirements

#### NFR-1: Performance

- Notification delivery: <1 second (p95)
- Notification center load time: <200ms
- Support 10,000 concurrent connections per server

#### NFR-2: Reliability

- 99.9% uptime for WebSocket service
- Zero message loss (persisted before sending)
- Graceful degradation (polling fallback if WebSocket unavailable)

#### NFR-3: Security

- Authentication required for WebSocket connection
- Users only receive their own notifications
- Rate limiting: 100 notifications per user per hour

## Architecture

### Components

#### 1. WebSocket Server

- Technology: Socket.io
- Handles real-time connections
- Scales horizontally with Redis adapter
- Deployment: Separate service, auto-scaling enabled

#### 2. Notification Service

- Creates and persists notifications
- Publishes to WebSocket server via Redis pub/sub
- Handles batching and rate limiting
- API: REST endpoints for notification management

#### 3. Notification Center UI

- React component with real-time updates
- Notification list with pagination
- Preference management
- Toast notifications for real-time alerts

#### 4. Database

- PostgreSQL table: notifications
- Indexes: user_id, created_at, read_status
- Retention: 90 days (configurable)

### Data Model

```typescript
interface Notification {
  id: string;
  user_id: string;
  category: 'mention' | 'assignment' | 'approval' | 'system';
  title: string;
  message: string;
  link?: string;
  read: boolean;
  created_at: Date;
  metadata?: Record<string, any>;
}
```
````

## Implementation Plan

### Phase 1: Infrastructure (Week 1-2)

Set up WebSocket server and database schema.

**Files to create**:

- `services/websocket/server.ts` - WebSocket server
- `services/websocket/connection-manager.ts` - Connection lifecycle
- `prisma/schema.prisma` - Notification model
- `services/notification/service.ts` - Notification business logic

**Files to modify**:

- `docker-compose.yml` - Add Redis service
- `infrastructure/k8s/` - WebSocket deployment config

### Phase 2: Backend API (Week 3-4)

Implement notification creation and management APIs.

**Files to create**:

- `src/routes/api/notifications.ts` - REST endpoints
- `src/controllers/notification.controller.ts` - CRUD operations
- `src/services/notification/publisher.ts` - Publish to WebSocket
- `src/workers/notification-cleanup.ts` - Cleanup old notifications

**Files to modify**:

- `src/routes/index.ts` - Register notification routes
- `src/services/events/` - Emit notification events

### Phase 3: Frontend (Week 5-6)

Build notification center UI and real-time updates.

**Files to create**:

- `src/components/NotificationCenter.tsx`
- `src/components/NotificationList.tsx`
- `src/components/NotificationItem.tsx`
- `src/components/NotificationPreferences.tsx`
- `src/hooks/useNotifications.ts` - WebSocket hook
- `src/hooks/useNotificationPreferences.ts`

**Files to modify**:

- `src/App.tsx` - Add notification center
- `src/layouts/Header.tsx` - Add notification bell icon

### Phase 4: Testing & Rollout (Week 7-8)

Comprehensive testing and gradual rollout.

**Tasks**:

- Unit tests for all components
- Integration tests for WebSocket flow
- Load testing (10k concurrent connections)
- Beta rollout (10% of users)
- Full rollout with monitoring

## Acceptance Criteria

### Must Have (P0)

- [ ] WebSocket connections work for authenticated users
- [ ] Notifications delivered in real-time (<1 second)
- [ ] Notification center displays all notifications
- [ ] Mark as read/unread functionality
- [ ] Notification preferences per category
- [ ] Offline queue works correctly
- [ ] Test coverage >90%
- [ ] Load tested at 10k concurrent connections

### Should Have (P1)

- [ ] Bulk actions (mark all read)
- [ ] Notification search
- [ ] Desktop notifications (browser)
- [ ] Sound effects (configurable)

### Nice to Have (P2)

- [ ] Notification analytics dashboard
- [ ] Custom notification templates
- [ ] Notification scheduling

## Risks & Mitigation

| Risk                         | Impact | Probability | Mitigation                                          |
| ---------------------------- | ------ | ----------- | --------------------------------------------------- |
| WebSocket scaling issues     | High   | Medium      | Horizontal scaling with Redis adapter, load testing |
| Message loss during failures | High   | Low         | Persist before sending, acknowledge receipt         |
| Browser compatibility        | Medium | Medium      | Polling fallback for older browsers                 |
| Notification fatigue         | Medium | High        | Rate limiting, smart batching, user preferences     |

## Timeline

- **Week 1-2**: Infrastructure setup
- **Week 3-4**: Backend API implementation
- **Week 5-6**: Frontend development
- **Week 7**: Testing and bug fixes
- **Week 8**: Beta rollout and monitoring
- **Week 9**: Full rollout

**Launch Date**: December 15, 2025

## Dependencies

- Redis (for pub/sub and session storage)
- PostgreSQL migration for notifications table
- Socket.io library
- Frontend notification UI framework

## Open Questions

1. Should we support notification export (download history)?
2. What's the retention policy for read vs unread notifications?
3. Do we need notification categories beyond the 4 defined?

## Appendix

### Related Documents

- Technical Design Doc: `docs/tdd/notifications-system.md`
- API Specification: `docs/api/notifications-api.md`
- Security Review: `docs/security/notifications-review.md`

````

**Usage**:
```bash
# Validate BMAD PRD
anvil validate prd-notifications.md --verbose

# Run quality gates
anvil gate prd-notifications.md

# Export to APS
anvil export prd-notifications.md --to aps --output prds/notifications.aps.json

# Export to YAML for easier reading
anvil export prd-notifications.md --to yaml
````

## Team Workflows

### Workflow: Feature Development Cycle

**Scenario**: Team developing a new feature from planning to deployment.

**Steps**:

1. **Planning Phase**

```bash
# Product Manager creates PRD
vim prd-feature.md

# Validate PRD structure
anvil validate prd-feature.md

# Commit PRD
git add prd-feature.md
git commit -m "docs: Add feature PRD"
git push
```

2. **Design Phase**

```bash
# Technical Lead creates spec
vim spec-feature.md

# Validate spec
anvil validate spec-feature.md

# Export to APS for archival
anvil export spec-feature.md --to aps --output specs/feature.aps.json

# Commit both
git add spec-feature.md specs/feature.aps.json
git commit -m "docs: Add technical spec for feature"
```

3. **Implementation Phase**

```bash
# Developer starts implementation
git checkout -b feature/new-feature

# Before each commit, validate plans still valid
anvil validate spec-feature.md

# Run quality gates on codebase
anvil gate spec-feature.md

# If gates pass, commit
git commit -am "feat: Implement feature (part 1)"
```

4. **Review Phase**

```bash
# In PR, CI runs Anvil validation
# .github/workflows/anvil.yml triggers

# Reviewer checks plan compliance
anvil validate spec-feature.md
anvil gate spec-feature.md --verbose

# Ensure gates pass before merge
```

5. **Deployment Phase**

```bash
# After merge, update plan status
# (In future: anvil update-status spec-feature.md --status deployed)

# Archive evidence
git tag -a v1.0-feature -m "Feature release"
git push --tags
```

### Workflow: Multi-Format Team

**Scenario**: Team uses both SpecKit (engineers) and BMAD (product).

**Setup**:

```bash
# Project structure
project/
├── docs/
│   ├── prds/           # BMAD PRDs (product team)
│   ├── specs/          # SpecKit specs (engineering)
│   └── archive/        # APS archives
├── .anvilrc            # Gate configuration
└── .github/
    └── workflows/
        └── anvil.yml   # CI validation
```

**Product Manager Workflow**:

```bash
# Create PRD in BMAD format
vim docs/prds/feature-x.md

# Validate
anvil validate docs/prds/feature-x.md

# Export to APS for engineering
anvil export docs/prds/feature-x.md --to aps \
  --output docs/archive/feature-x.aps.json

# Commit
git add docs/prds/feature-x.md docs/archive/feature-x.aps.json
git commit -m "docs: Add feature X PRD"
```

**Engineer Workflow**:

```bash
# Read APS or create SpecKit spec
vim docs/specs/feature-x-spec.md

# Validate spec
anvil validate docs/specs/feature-x-spec.md

# Run gates
anvil gate docs/specs/feature-x-spec.md

# Link spec to PRD in commit message
git add docs/specs/feature-x-spec.md
git commit -m "docs: Add feature X spec (refs: feature-x.aps.json)"
```

**CI Validation** (`.github/workflows/anvil.yml`):

```yaml
name: Validate Plans

on:
  pull_request:
    paths:
      - 'docs/prds/*.md'
      - 'docs/specs/*.md'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'

      - name: Install Anvil
        run: |
          git clone https://github.com/EddaCraft/anvil-001.git /tmp/anvil
          cd /tmp/anvil && pnpm install && pnpm build
          cd /tmp/anvil/cli && pnpm link --global

      - name: Validate PRDs
        run: |
          for file in docs/prds/*.md; do
            anvil validate "$file" || exit 1
          done

      - name: Validate Specs
        run: |
          for file in docs/specs/*.md; do
            anvil validate "$file" || exit 1
            anvil gate "$file" || exit 1
          done
```

## CI/CD Integration

### Example: GitHub Actions Complete Setup

**Scenario**: Comprehensive CI pipeline with Anvil validation.

**File**: `.github/workflows/ci.yml`

```yaml
name: CI Pipeline

on:
  pull_request:
    branches: [main, develop]
  push:
    branches: [main, develop]

jobs:
  # Job 1: Validate planning documents
  validate-plans:
    name: Validate Planning Documents
    runs-on: ubuntu-latest
    if: |
      contains(github.event.head_commit.message, 'docs:') ||
      contains(github.event.pull_request.title, 'docs:')
    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'
          cache: 'pnpm'

      - name: Install pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 10

      - name: Install Anvil
        run: |
          git clone https://github.com/EddaCraft/anvil-001.git /tmp/anvil
          cd /tmp/anvil
          pnpm install
          pnpm build
          cd cli && pnpm link --global

      - name: Get changed files
        id: changed-files
        uses: tj-actions/changed-files@v39
        with:
          files: |
            **/*.md
            **/*.aps.json

      - name: Validate changed plans
        if: steps.changed-files.outputs.any_changed == 'true'
        run: |
          echo "Changed files:"
          echo "${{ steps.changed-files.outputs.all_changed_files }}"

          for file in ${{ steps.changed-files.outputs.all_changed_files }}; do
            if [[ "$file" == *.md ]] || [[ "$file" == *.aps.json ]]; then
              echo "Validating $file"
              anvil validate "$file" --verbose || exit 1
            fi
          done

  # Job 2: Run quality gates
  quality-gates:
    name: Quality Gates
    runs-on: ubuntu-latest
    needs: validate-plans
    steps:
      - name: Checkout code
        uses: actions/checkout@v3

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'

      - name: Install dependencies
        run: pnpm install

      - name: Install Anvil
        run: |
          git clone https://github.com/EddaCraft/anvil-001.git /tmp/anvil
          cd /tmp/anvil && pnpm install && pnpm build
          cd cli && pnpm link --global

      - name: Run quality gates on specs
        run: |
          if [ -d "docs/specs" ]; then
            for file in docs/specs/*.md; do
              echo "Running gates on $file"
              anvil gate "$file" --verbose || exit 1
            done
          fi

      - name: Upload gate results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: gate-results
          path: .anvil/evidence/

  # Job 3: Build and test
  build-test:
    name: Build and Test
    runs-on: ubuntu-latest
    needs: quality-gates
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '20'

      - name: Install dependencies
        run: pnpm install

      - name: Build
        run: pnpm build

      - name: Test
        run: pnpm test

      - name: Coverage
        run: pnpm test:coverage
```

### Example: GitLab CI Pipeline

**File**: `.gitlab-ci.yml`

```yaml
stages:
  - validate
  - quality-gates
  - build
  - test

variables:
  ANVIL_VERSION: 'latest'

# Install Anvil (reusable)
.install-anvil: &install-anvil
  before_script:
    - corepack enable
    - corepack prepare pnpm@latest --activate
    - git clone https://github.com/EddaCraft/anvil-001.git /tmp/anvil
    - cd /tmp/anvil && pnpm install && pnpm build
    - cd cli && pnpm link --global
    - cd $CI_PROJECT_DIR

validate-plans:
  stage: validate
  image: node:20
  <<: *install-anvil
  script:
    - echo "Validating planning documents..."
    - |
      for file in docs/**/*.md; do
        echo "Validating $file"
        anvil validate "$file" || exit 1
      done
  only:
    changes:
      - docs/**/*.md
      - '**/*.aps.json'

quality-gates:
  stage: quality-gates
  image: node:20
  <<: *install-anvil
  script:
    - echo "Running quality gates..."
    - pnpm install
    - |
      for file in docs/specs/*.md; do
        echo "Running gates on $file"
        anvil gate "$file" --verbose || exit 1
      done
  artifacts:
    paths:
      - .anvil/evidence/
    when: always
  only:
    changes:
      - docs/**/*.md
      - src/**/*
      - tests/**/*

build:
  stage: build
  image: node:20
  script:
    - pnpm install
    - pnpm build
  artifacts:
    paths:
      - dist/

test:
  stage: test
  image: node:20
  script:
    - pnpm install
    - pnpm test
    - pnpm test:coverage
  coverage: '/All files[^|]*\|[^|]*\s+([\d\.]+)/'
  artifacts:
    reports:
      coverage_report:
        coverage_format: cobertura
        path: coverage/cobertura-coverage.xml
```

## Advanced Use Cases

### Use Case: Format Conversion Pipeline

**Scenario**: Convert legacy planning docs to APS, validate, then export to
team's preferred format.

```bash
#!/bin/bash
# migrate-plans.sh - Convert legacy plans to APS

LEGACY_DIR="./legacy-docs"
OUTPUT_DIR="./docs/plans"
ARCHIVE_DIR="./docs/archive"

# Create directories
mkdir -p "$OUTPUT_DIR" "$ARCHIVE_DIR"

echo "Converting legacy planning documents..."

# Find all markdown files
find "$LEGACY_DIR" -name "*.md" | while read -r file; do
  filename=$(basename "$file" .md)

  echo "Processing: $file"

  # 1. Validate original
  if anvil validate "$file"; then
    echo "  ✓ Valid format detected"

    # 2. Convert to APS
    aps_file="$ARCHIVE_DIR/${filename}.aps.json"
    if anvil export "$file" --to aps --output "$aps_file"; then
      echo "  ✓ Converted to APS: $aps_file"

      # 3. Export to SpecKit for team use
      speckit_dir="$OUTPUT_DIR/${filename}"
      if anvil export "$aps_file" --to speckit --output "$speckit_dir/"; then
        echo "  ✓ Exported to SpecKit: $speckit_dir/"

        # 4. Validate SpecKit output
        if anvil validate "$speckit_dir/spec.md"; then
          echo "  ✓ SpecKit output validated"
        else
          echo "  ✗ SpecKit validation failed"
        fi
      fi
    fi
  else
    echo "  ✗ Validation failed, skipping"
  fi
  echo ""
done

echo "Migration complete!"
echo "APS files: $ARCHIVE_DIR/"
echo "SpecKit files: $OUTPUT_DIR/"
```

### Use Case: Custom Gate Configuration Per Project

**Scenario**: Different quality standards for different project types.

**Structure**:

```
company-monorepo/
├── services/
│   ├── api/
│   │   └── .anvilrc              # Strict: 90% coverage
│   ├── worker/
│   │   └── .anvilrc              # Moderate: 80% coverage
│   └── frontend/
│       └── .anvilrc              # Moderate: 75% coverage
├── tools/
│   └── .anvilrc                  # Lenient: 60% coverage
└── docs/
    └── .anvilrc                  # Validation only
```

**API Service** (`services/api/.anvilrc`):

```json
{
  "checks": {
    "lint": {
      "enabled": true,
      "command": "pnpm lint",
      "timeout": 30000
    },
    "test": {
      "enabled": true,
      "command": "pnpm test",
      "timeout": 120000
    },
    "coverage": {
      "enabled": true,
      "threshold": 90,
      "command": "pnpm test:coverage"
    },
    "secrets": {
      "enabled": true,
      "patterns": ["api_key", "secret", "password", "token"]
    }
  }
}
```

**Frontend** (`services/frontend/.anvilrc`):

```json
{
  "checks": {
    "lint": {
      "enabled": true,
      "command": "pnpm lint"
    },
    "test": {
      "enabled": true,
      "command": "pnpm test"
    },
    "coverage": {
      "enabled": true,
      "threshold": 75
    },
    "secrets": {
      "enabled": false
    }
  }
}
```

**Documentation** (`docs/.anvilrc`):

```json
{
  "checks": {
    "lint": {
      "enabled": false
    },
    "test": {
      "enabled": false
    },
    "coverage": {
      "enabled": false
    },
    "secrets": {
      "enabled": true
    }
  }
}
```

**Usage**:

```bash
# Each service uses its own configuration
cd services/api
anvil gate spec.md  # Uses strict 90% coverage

cd ../frontend
anvil gate spec.md  # Uses moderate 75% coverage

cd ../../docs
anvil gate prd.md   # Only checks for secrets
```

### Use Case: Batch Validation Script

**Scenario**: Validate all planning documents in repository.

```bash
#!/bin/bash
# validate-all.sh - Validate all planning documents

set -e

echo "Anvil Batch Validation"
echo "====================="
echo ""

# Counters
total=0
passed=0
failed=0

# Find all relevant files
files=$(find . -type f \( -name "spec.md" -o -name "plan.md" -o -name "prd.md" -o -name "*.aps.json" \) ! -path "*/node_modules/*" ! -path "*/.git/*")

echo "Found files:"
echo "$files"
echo ""

# Validate each file
for file in $files; do
  total=$((total + 1))
  echo "[$total] Validating: $file"

  if anvil validate "$file" --verbose; then
    echo "  ✓ PASS"
    passed=$((passed + 1))
  else
    echo "  ✗ FAIL"
    failed=$((failed + 1))
  fi
  echo ""
done

# Summary
echo "====================="
echo "Validation Summary"
echo "====================="
echo "Total:  $total"
echo "Passed: $passed"
echo "Failed: $failed"
echo ""

if [ $failed -gt 0 ]; then
  echo "❌ Some validations failed"
  exit 1
else
  echo "✅ All validations passed"
  exit 0
fi
```

**Usage**:

```bash
# Make executable
chmod +x validate-all.sh

# Run validation
./validate-all.sh

# Use in CI
npm run validate-all || exit 1
```

## Next Steps

- **User Guide**: See [USER_GUIDE.md](./USER_GUIDE.md) for complete reference
- **Troubleshooting**: See [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for common
  issues
- **CLI Reference**: See [cli/README.md](../cli/README.md) for all commands

---

**Version**: 0.0.0 (Pre-release) **Last Updated**: 2025-11-09
