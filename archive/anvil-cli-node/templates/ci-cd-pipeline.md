---
id: ci-cd-pipeline
name: CI/CD Pipeline
description: Set up continuous integration and deployment pipeline
category: infrastructure
tags: [ci, cd, github-actions, deployment, automation]
variables:
  - name: platform
    description: CI/CD platform (github-actions, gitlab-ci, circleci)
    default: github-actions
    required: false
  - name: deploy_target
    description: Deployment target (vercel, aws, docker)
    default: vercel
    required: false
  - name: node_version
    description: Node.js version
    default: '22'
    required: false
---

# CI/CD Pipeline Setup

## Intent

Set up {{ platform }} CI/CD pipeline with automated testing, linting, and
deployment to {{ deploy_target }}.

## Changes

### 1. Create CI Workflow

- **File**: `.github/workflows/ci.yml`
- **Action**: Create
- **Description**: Main CI pipeline (lint, test, build)

### 2. Create CD Workflow

- **File**: `.github/workflows/deploy.yml`
- **Action**: Create
- **Description**: Deployment to {{ deploy_target }}

### 3. Create Release Workflow

- **File**: `.github/workflows/release.yml`
- **Action**: Create
- **Description**: Automated releases and changelog

### 4. Add Environment Files

- **File**: `.github/workflows/env/`
- **Action**: Create
- **Description**: Environment-specific configurations

### 5. Update Package Scripts

- **File**: `package.json`
- **Action**: Modify
- **Description**: Add CI-specific scripts

## CI Pipeline Jobs

```yaml
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '{{ node_version }}'
      - run: npm ci
      - run: npm run lint

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '{{ node_version }}'
      - run: npm ci
      - run: npm test -- --coverage

  build:
    needs: [lint, test]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '{{ node_version }}'
      - run: npm ci
      - run: npm run build
```

## CD Pipeline ({{ deploy_target }})

```yaml
deploy:
  needs: build
  if: github.ref == 'refs/heads/main'
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Deploy to {{ deploy_target }}
      # Deployment steps here
```

## Branch Strategy

| Branch     | Trigger | Environment |
| ---------- | ------- | ----------- |
| main       | Push    | Production  |
| develop    | Push    | Staging     |
| feature/\* | PR      | Preview     |

## Required Secrets

```
# GitHub Actions Secrets
{{ deploy_target | uppercase }}_TOKEN
DATABASE_URL
NODE_ENV
```

## Quality Gates

- [ ] All tests passing
- [ ] Linting passing
- [ ] Type checking passing
- [ ] Coverage threshold met
- [ ] Security scan passing
- [ ] Build successful

## Notifications

- [ ] Slack/Discord on failure
- [ ] Email on deployment
- [ ] PR comments with status

## Rollback Strategy

1. Automatic rollback on health check failure
2. Manual rollback via GitHub Actions UI
3. Database migrations with rollback support

## Acceptance Criteria

- [ ] CI runs on all PRs
- [ ] Tests must pass to merge
- [ ] Auto-deploy to staging on develop
- [ ] Auto-deploy to production on main
- [ ] Notifications working
- [ ] Rollback tested
