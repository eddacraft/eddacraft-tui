import { writeFileSync, mkdirSync, existsSync, readFileSync, appendFileSync } from 'node:fs';
import { join } from 'node:path';
import type { EnvironmentInfo } from './environment-detector.js';

export type PlanningFormat = 'aps' | 'speckit' | 'bmad' | 'generic' | 'skip';
export type ConfigTemplate = 'basic' | 'strict' | 'ci';

export interface InitOptions {
  projectRoot: string;
  planningDir: string;
  format: PlanningFormat;
  createExample: boolean;
  configTemplate: ConfigTemplate;
  enabledChecks: string[];
  coverageThreshold: number;
  /** When provided, overrides the template-derived overall_score threshold. */
  overallScoreOverride?: number;
}

/**
 * Generates configuration files and example documents for Anvil
 */
export class TemplateGenerator {
  constructor(private readonly options: InitOptions) {}

  /**
   * Generate .anvilrc configuration file
   */
  public generateAnvilrc(): void {
    const config = this.buildAnvilConfig();
    const configPath = join(this.options.projectRoot, '.anvilrc');

    writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n', 'utf-8');
  }

  /**
   * Create .anvil directory
   */
  public createAnvilDirectory(): void {
    const anvilDir = join(this.options.projectRoot, '.anvil');
    if (!existsSync(anvilDir)) {
      mkdirSync(anvilDir, { recursive: true });
    }
  }

  /**
   * Create planning documents directory
   */
  public createPlanningDirectory(): void {
    const planningDir = join(this.options.projectRoot, this.options.planningDir);
    if (!existsSync(planningDir)) {
      mkdirSync(planningDir, { recursive: true });
    }
  }

  /**
   * Generate example planning document
   */
  public generateExamplePlan(env: EnvironmentInfo): string[] {
    if (!this.options.createExample || this.options.format === 'skip') {
      return [];
    }

    const planningDir = join(this.options.projectRoot, this.options.planningDir);

    switch (this.options.format) {
      case 'aps':
        return this.generateApsExample(planningDir, env);
      case 'speckit':
        return this.generateSpecKitExample(planningDir, env);
      case 'bmad':
        return this.generateBmadExample(planningDir, env);
      case 'generic':
        return this.generateGenericExample(planningDir, env);
      default:
        return [];
    }
  }

  /**
   * Update .gitignore to include Anvil patterns
   */
  public updateGitignore(): void {
    const gitignorePath = join(this.options.projectRoot, '.gitignore');

    const patterns = ['', '# Anvil', '.anvil/cache/', '.anvil/evidence/', '.anvil/*.log'];

    if (existsSync(gitignorePath)) {
      const content = readFileSync(gitignorePath, 'utf-8');
      if (!content.includes('# Anvil')) {
        appendFileSync(gitignorePath, '\n' + patterns.join('\n') + '\n', 'utf-8');
      }
    } else {
      writeFileSync(gitignorePath, patterns.join('\n') + '\n', 'utf-8');
    }
  }

  private buildAnvilConfig(): object {
    const checks = this.options.enabledChecks.map((checkName) => {
      const check: {
        name: string;
        description: string;
        enabled: boolean;
        config: { min_score: number; thresholds?: object };
      } = {
        name: checkName,
        description: this.getCheckDescription(checkName),
        enabled: true,
        config: { min_score: 80 },
      };

      if (checkName === 'coverage') {
        check.config.thresholds = {
          lines: this.options.coverageThreshold,
          functions: this.options.coverageThreshold,
          branches: this.options.coverageThreshold,
          statements: this.options.coverageThreshold,
        };
      }

      return check;
    });

    const overallScore =
      this.options.overallScoreOverride ?? (this.options.configTemplate === 'strict' ? 90 : 80);

    return {
      version: 1,
      checks,
      thresholds: {
        overall_score: overallScore,
      },
    };
  }

  private getCheckDescription(checkName: string): string {
    const descriptions: Record<string, string> = {
      eslint: 'Code quality checks',
      test: 'Unit test validation',
      coverage: 'Test coverage validation',
      secret: 'Secret scanning',
    };
    return descriptions[checkName] || 'Quality check';
  }

  private generateSpecKitExample(planningDir: string, env: EnvironmentInfo): string[] {
    const specPath = join(planningDir, 'example-spec.md');
    const planPath = join(planningDir, 'example-plan.md');
    const tasksPath = join(planningDir, 'example-tasks.md');

    // spec.md
    const specContent = `# Feature Specification: Example Feature

## Metadata
- **Author**: ${env.projectName ? 'Development Team' : 'Your Name'}
- **Created**: ${new Date().toISOString().split('T')[0]}
- **Status**: Draft

## Overview
This is an example specification demonstrating how to use Anvil with SpecKit format.

## Requirements

### Functional Requirements
- **REQ-1**: The system shall provide example functionality
- **REQ-2**: The system shall validate user inputs
- **REQ-3**: The system shall handle errors gracefully

### Non-Functional Requirements
- **NFR-1**: Response time shall be under 200ms
- **NFR-2**: Code coverage shall exceed 80%
- **NFR-3**: All code shall pass linting checks

## Architecture
The feature will be implemented as a modular component with clear separation of concerns.

## Acceptance Criteria
1. All functional requirements are met
2. Test coverage exceeds 80%
3. No linting errors
4. Documentation is complete
`;

    // plan.md
    const planContent = `# Implementation Plan: Example Feature

## Intent
Implement example feature with full test coverage and quality gates.

## Changes

### 1. Create Core Module
- **File**: \`src/example/core.ts\`
- **Action**: Create
- **Description**: Core functionality for example feature

### 2. Add Tests
- **File**: \`src/example/core.test.ts\`
- **Action**: Create
- **Description**: Comprehensive test suite

### 3. Update Documentation
- **File**: \`README.md\`
- **Action**: Modify
- **Description**: Add example feature documentation

## Dependencies
- None

## Validation
- Run tests: \`${this.getTestCommand(env)}\`
- Run linting: \`${this.getLintCommand(env)}\`
- Verify coverage: \`${this.getCoverageCommand(env)}\`
`;

    // tasks.md
    const tasksContent = `# Tasks: Example Feature

## Implementation Tasks
- [ ] Set up project structure
- [ ] Implement core functionality
- [ ] Write unit tests
- [ ] Add integration tests
- [ ] Update documentation
- [ ] Run quality gates

## Quality Gates
- [ ] All tests passing
- [ ] Coverage > 80%
- [ ] No linting errors
- [ ] No secrets detected

## Completion Criteria
- All tasks completed
- All quality gates passed
- Documentation updated
- Code reviewed and approved
`;

    writeFileSync(specPath, specContent, 'utf-8');
    writeFileSync(planPath, planContent, 'utf-8');
    writeFileSync(tasksPath, tasksContent, 'utf-8');

    return [specPath, planPath, tasksPath];
  }

  private generateBmadExample(planningDir: string, env: EnvironmentInfo): string[] {
    const prdPath = join(planningDir, 'example-prd.md');

    const content = `# Product Requirements Document: Example Feature

## Executive Summary
This PRD defines the requirements for an example feature demonstrating Anvil with BMAD format.

## Problem Statement
Users need a clear example of how to structure planning documents for Anvil validation.

## Goals and Success Metrics

### Goals
1. Provide clear example of BMAD format
2. Demonstrate quality gate integration
3. Show best practices for planning documents

### Success Metrics
- Documentation clarity score > 8/10
- All quality gates passing
- Test coverage > 80%

## User Stories

### As a Developer
I want to see example planning documents
So that I can create my own plans correctly

### As a Quality Engineer
I want automated quality checks
So that I can ensure code meets standards

## Technical Requirements

### Functional
- REQ-1: System shall validate planning documents
- REQ-2: System shall run quality gates
- REQ-3: System shall provide clear feedback

### Non-Functional
- NFR-1: Validation time < 5 seconds
- NFR-2: Test coverage > ${this.options.coverageThreshold}%
- NFR-3: Zero linting errors

## Architecture

### Components
1. **Validator** - Validates document structure
2. **Gate Runner** - Executes quality checks
3. **Reporter** - Displays results

### Technology Stack
${env.hasTypeScript ? '- TypeScript' : '- JavaScript'}
${env.hasVitest ? '- Vitest (testing)' : env.hasJest ? '- Jest (testing)' : ''}
${env.hasEslint ? '- ESLint (linting)' : ''}

## Implementation Plan

### Phase 1: Setup (Week 1)
- Configure Anvil
- Set up quality gates
- Create initial tests

### Phase 2: Development (Week 2-3)
- Implement core features
- Write comprehensive tests
- Document functionality

### Phase 3: Validation (Week 4)
- Run quality gates
- Fix any issues
- Finalise documentation

## Risks and Mitigation
- **Risk**: Low test coverage
  - **Mitigation**: Require 80% coverage threshold
- **Risk**: Linting errors
  - **Mitigation**: Pre-commit hooks with auto-fix

## Appendix

### References
- Anvil Documentation: https://github.com/EddaCraft/anvil-001
- BMAD Format Guide: See adapters documentation

### Revision History
- v0.1.0 (${new Date().toISOString().split('T')[0]}): Initial draft
`;

    writeFileSync(prdPath, content, 'utf-8');
    return [prdPath];
  }

  private generateGenericExample(planningDir: string, env: EnvironmentInfo): string[] {
    const planPath = join(planningDir, 'example-plan.md');

    const content = `# Example Plan

## Overview
This is an example planning document for use with Anvil.

## Objectives
- Demonstrate generic markdown planning format
- Show integration with quality gates
- Provide template for new plans

## Tasks
1. Set up project structure
2. Implement core functionality
3. Write tests
4. Run quality gates
5. Document changes

## Quality Checks
${env.hasEslint ? '- ✓ ESLint configured' : '- ✗ ESLint not detected'}
${env.hasVitest || env.hasJest ? '- ✓ Testing framework configured' : '- ✗ No testing framework detected'}
- ✓ Secret scanning enabled

## Commands
- Test: \`${this.getTestCommand(env)}\`
- Lint: \`${this.getLintCommand(env)}\`
- Coverage: \`${this.getCoverageCommand(env)}\`

## Acceptance Criteria
- All tests passing
- Coverage > ${this.options.coverageThreshold}%
- No linting errors
- No secrets in code

## Notes
This example demonstrates a simple generic markdown format. You can customise this template for your specific needs.

---

**Generated by**: Anvil CLI
**Date**: ${new Date().toISOString().split('T')[0]}
`;

    writeFileSync(planPath, content, 'utf-8');
    return [planPath];
  }

  private generateApsExample(planningDir: string, env: EnvironmentInfo): string[] {
    const indexPath = join(planningDir, 'index.aps.md');
    const modulePath = join(planningDir, 'modules', 'example-feature.aps.md');

    // Create modules subdirectory
    const modulesDir = join(planningDir, 'modules');
    if (!existsSync(modulesDir)) {
      mkdirSync(modulesDir, { recursive: true });
    }

    const indexContent = `---
id: example-project
title: Example Project
version: 0.1.0
status: draft
---

# Example Project

> Example APS planning document demonstrating Anvil integration.

## Overview

This is an example index document for the Anvil Planning Spec (APS) format.
APS provides structured task tracking with dependencies, validation, and
provenance tracking.

## Modules

| ID | Module | Status | Tasks |
|----|--------|--------|-------|
| FEAT-001 | [Example Feature](modules/example-feature.aps.md) | draft | 3 |

## Quality Gates

- Test coverage: ${this.options.coverageThreshold}%
- Linting: ESLint
- Secrets: Scanning enabled

---

**Generated by**: Anvil CLI
**Date**: ${new Date().toISOString().split('T')[0]}
`;

    const moduleContent = `---
id: FEAT-001
title: Example Feature
status: draft
priority: medium
depends_on: []
---

# Example Feature

> Demonstrates APS module structure with tasks and acceptance criteria.

## Overview

This module shows how to structure an APS planning document with:
- Task definitions with unique IDs
- Dependencies between tasks
- Acceptance criteria
- File change specifications

## Tasks

### FEAT-001-001: Set up project structure

**Status**: pending
**Priority**: high

Create the initial project structure with required directories.

#### Changes
- \`src/example/\` — Create directory
- \`src/example/index.ts\` — Create entry point

#### Acceptance Criteria
- [ ] Directory structure created
- [ ] Entry point exports module

---

### FEAT-001-002: Implement core functionality

**Status**: pending
**Priority**: medium
**Depends on**: FEAT-001-001

Implement the core feature logic.

#### Changes
- \`src/example/core.ts\` — Create core module
- \`src/example/types.ts\` — Create type definitions

#### Acceptance Criteria
- [ ] Core logic implemented
- [ ] Types exported
- [ ] No linting errors

---

### FEAT-001-003: Add tests

**Status**: pending
**Priority**: medium
**Depends on**: FEAT-001-002

Add comprehensive test coverage.

#### Changes
- \`src/example/core.test.ts\` — Create test file

#### Acceptance Criteria
- [ ] Tests written
- [ ] Coverage > ${this.options.coverageThreshold}%
- [ ] All tests passing

## Validation Commands

\`\`\`bash
# Run tests
${this.getTestCommand(env)}

# Run linting
${this.getLintCommand(env)}

# Check coverage
${this.getCoverageCommand(env)}
\`\`\`

---

**Generated by**: Anvil CLI
**Date**: ${new Date().toISOString().split('T')[0]}
`;

    writeFileSync(indexPath, indexContent, 'utf-8');
    writeFileSync(modulePath, moduleContent, 'utf-8');
    return [indexPath, modulePath];
  }

  private getTestCommand(env: EnvironmentInfo): string {
    if (env.hasVitest) return `${env.packageManager} test`;
    if (env.hasJest) return `${env.packageManager} test`;
    return 'npm test';
  }

  private getLintCommand(env: EnvironmentInfo): string {
    if (env.hasEslint) return `${env.packageManager} lint`;
    return 'eslint .';
  }

  private getCoverageCommand(env: EnvironmentInfo): string {
    if (env.hasVitest) return `${env.packageManager} test:coverage`;
    if (env.hasJest) return `${env.packageManager} test -- --coverage`;
    return 'npm test -- --coverage';
  }
}
