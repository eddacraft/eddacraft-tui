# Architecture Templates Expansion

## Intent

Expand the architecture template library with 4 new templates and improve descriptions on existing templates for better TUI presentation.

## Changes

### 1. Update Architecture Template Schema

- **File**: `core/src/architecture/definition-schema.ts`
- **Action**: Modify
- **Description**: Add new template types to `ArchitectureTemplateSchema` enum

### 2. Create Starter Template

- **File**: `core/src/architecture/templates/starter.yaml`
- **Action**: Create
- **Description**: Minimal, flexible architecture for MVPs and learning

```yaml
name: starter
description: Simple and flexible structure for new projects, MVPs, and learning - evolves with your needs

layers:
  components:
    patterns:
      - src/components/**
      - src/ui/**
    depends_on:
      - lib
    description: UI components and visual elements

  lib:
    patterns:
      - src/lib/**
      - src/utils/**
      - src/helpers/**
    depends_on: []
    description: Shared utilities and helper functions

  services:
    patterns:
      - src/services/**
      - src/api/**
    depends_on:
      - lib
    description: API calls and external service integrations
```

### 3. Create Monorepo Template

- **File**: `core/src/architecture/templates/monorepo.yaml`
- **Action**: Create
- **Description**: Multi-package workspace with shared libraries

```yaml
name: monorepo
description: Multi-package workspace structure with shared libraries and clear package boundaries

layers:
  apps:
    patterns:
      - apps/**
      - packages/app-*/**
    depends_on:
      - packages
      - shared
    description: Application packages (web, mobile, cli)

  packages:
    patterns:
      - packages/**
      - libs/**
    depends_on:
      - shared
    description: Reusable library packages

  shared:
    patterns:
      - shared/**
      - packages/shared/**
      - packages/common/**
    depends_on: []
    description: Shared utilities, types, and configurations
```

### 4. Create Serverless Template

- **File**: `core/src/architecture/templates/serverless.yaml`
- **Action**: Create
- **Description**: Functions-as-a-Service architecture

```yaml
name: serverless
description: Functions-as-a-Service architecture for AWS Lambda, Azure Functions, or similar platforms

layers:
  functions:
    patterns:
      - src/functions/**
      - src/handlers/**
      - src/lambdas/**
    depends_on:
      - services
      - shared
    description: Serverless function handlers

  services:
    patterns:
      - src/services/**
      - src/business/**
    depends_on:
      - shared
    description: Business logic shared across functions

  shared:
    patterns:
      - src/shared/**
      - src/utils/**
      - src/lib/**
    depends_on: []
    description: Shared utilities, types, and configurations
```

### 5. Create Nx Workspace Template

- **File**: `core/src/architecture/templates/nx-workspace.yaml`
- **Action**: Create
- **Description**: Nx monorepo with libs/apps structure

```yaml
name: nx-workspace
description: Nx monorepo structure with apps, feature libs, and shared libs following Nx conventions

layers:
  apps:
    patterns:
      - apps/**
    depends_on:
      - feature-libs
      - shared-libs
    description: Deployable applications

  feature-libs:
    patterns:
      - libs/feature-*/**
      - libs/*/feature-*/**
    depends_on:
      - data-access-libs
      - ui-libs
      - shared-libs
    description: Feature libraries containing smart components

  data-access-libs:
    patterns:
      - libs/data-access-*/**
      - libs/*/data-access-*/**
    depends_on:
      - shared-libs
    description: Data access libraries for API and state

  ui-libs:
    patterns:
      - libs/ui-*/**
      - libs/*/ui-*/**
    depends_on:
      - shared-libs
    description: Presentational UI component libraries

  shared-libs:
    patterns:
      - libs/shared/**
      - libs/util-*/**
      - libs/*/util-*/**
    depends_on: []
    description: Shared utilities, types, and configurations
```

### 6. Update Template Files Mapping

- **File**: `core/src/architecture/templates/index.ts`
- **Action**: Modify
- **Description**: Add new templates to `TEMPLATE_FILES` and update `list()` method

### 7. Improve Existing Template Descriptions

- **File**: `core/src/architecture/templates/layered.yaml`
- **Action**: Modify
- **Description**: Enhance description for TUI display

- **File**: `core/src/architecture/templates/hexagonal.yaml`
- **Action**: Modify
- **Description**: Enhance description for TUI display

- **File**: `core/src/architecture/templates/clean.yaml`
- **Action**: Modify
- **Description**: Enhance description for TUI display

- **File**: `core/src/architecture/templates/ddd.yaml`
- **Action**: Modify
- **Description**: Enhance description for TUI display

### 8. Update Template Tests

- **File**: `core/src/architecture/templates/templates.test.ts`
- **Action**: Modify
- **Description**: Add tests for new templates

## Improved Descriptions

| Template | Current | Improved |
|----------|---------|----------|
| layered | Traditional layered architecture with presentation, business, and data tiers | Classic 3-tier architecture - ideal for APIs, web backends, and traditional applications |
| hexagonal | Ports and Adapters architecture with isolated domain core | Ports & Adapters pattern - keeps business logic isolated from external dependencies |
| clean | Clean Architecture with entities at the core and frameworks at the edge | Uncle Bob's Clean Architecture - strict dependency rules with entities at the core |
| ddd | Domain-Driven Design architecture with bounded contexts | Domain-Driven Design - organise code around business domains and bounded contexts |

## Acceptance Criteria

- [ ] All 4 new templates created and loadable
- [ ] Template validation passes for all templates
- [ ] Existing template descriptions improved
- [ ] Tests pass for all templates
- [ ] Templates appear correctly in TUI selection
