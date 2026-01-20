/**
 * SpecKit Import Adapter v2 - Official Format
 *
 * Supports official GitHub spec-kit format:
 * - spec.md: Requirements and user scenarios (WHAT and WHY)
 * - plan.md: Technical implementation details (HOW)
 * - tasks.md: Executable task breakdown
 *
 * Can import individual files or complete feature directories
 */

import { createPlan, type APSPlan, type Change, type Provenance } from '@eddacraft/anvil-core';
import type {
  AdapterConfig,
  ConversionError,
  ConversionResult,
  ConversionWarning,
  ExternalSpec,
  SpecContext,
} from '../common/types.js';
import { BaseAdapter } from '../common/types.js';
import { SpecParser, type ParsedSpec } from './parsers/spec-parser.js';
import { PlanParser, type ParsedPlan } from './parsers/plan-parser.js';
import { TasksParser, type ParsedTasks } from './parsers/tasks-parser.js';

interface SpecKitDocuments {
  spec?: {
    content: string;
    parsed?: ParsedSpec;
  };
  plan?: {
    content: string;
    parsed?: ParsedPlan;
  };
  tasks?: {
    content: string;
    parsed?: ParsedTasks;
  };
  metadata?: Record<string, unknown>;
}

export class SpecKitImportAdapterV2 extends BaseAdapter {
  readonly name = 'speckit-import-v2';
  readonly version = '2.0.0';
  readonly supportedFormats = ['speckit', 'spec-kit', 'spec.md', 'plan.md', 'tasks.md'] as const;

  private specParser: SpecParser;
  private planParser: PlanParser;
  private tasksParser: TasksParser;

  constructor(config: AdapterConfig = {}) {
    super(config);
    this.specParser = new SpecParser();
    this.planParser = new PlanParser();
    this.tasksParser = new TasksParser();
  }

  async generateSpec(intent: string, context: SpecContext): Promise<APSPlan> {
    const provenance: Provenance = {
      timestamp: new Date().toISOString(),
      source: 'cli',
      version: this.version,
      author: context.author,
      repository: context.repositoryPath,
      branch: context.branch,
      commit: context.commit,
    };

    // Generate a simple spec.md file creation change
    const changes: Change[] = [
      {
        type: 'file_create',
        path: 'specs/new-feature/spec.md',
        description: 'Create specification file following spec-kit format',
        content: this.generateSpecTemplate(intent),
      },
    ];

    const planId = `aps-${Date.now().toString(16).substring(0, 8)}`;

    return {
      ...createPlan({
        id: planId,
        intent,
        provenance,
        changes,
      }),
      schema_version: '0.1.0' as const,
      hash: '0'.repeat(64),
    } as APSPlan;
  }

  async validateSpec(spec: APSPlan): Promise<import('@eddacraft/anvil-core').ValidationResult> {
    const errors: Array<{ field: string; message: string }> = [];
    const warnings: Array<{ field: string; message: string }> = [];

    if (spec.proposed_changes.length === 0) {
      warnings.push({
        field: 'proposed_changes',
        message: 'No changes specified in the plan',
      });
    }

    const issues: Array<{
      path: string;
      message: string;
      code: string;
      severity: 'error' | 'warning';
    }> = [
      ...errors.map((e) => ({
        path: e.field,
        message: e.message,
        code: 'VALIDATION_ERROR',
        severity: 'error' as const,
      })),
      ...warnings.map((w) => ({
        path: w.field,
        message: w.message,
        code: 'VALIDATION_WARNING',
        severity: 'warning' as const,
      })),
    ];

    return {
      valid: errors.length === 0,
      data: spec,
      issues: issues.length > 0 ? issues : undefined,
      summary: errors.length === 0 ? 'Validation passed' : `Found ${errors.length} error(s)`,
    };
  }

  async convertToAPS(spec: ExternalSpec): Promise<ConversionResult<APSPlan>> {
    const errors: ConversionError[] = [];
    const warnings: ConversionWarning[] = [];

    if (!this.canImport(spec.format)) {
      return {
        success: false,
        errors: [
          {
            code: 'UNSUPPORTED_FORMAT',
            message: `Format '${spec.format}' is not supported by this adapter`,
          },
        ],
      };
    }

    try {
      const docs = spec.content as SpecKitDocuments;

      // Parse all available documents
      if (docs.spec?.content) {
        docs.spec.parsed = this.specParser.parseSpec(docs.spec.content);
      }
      if (docs.plan?.content) {
        docs.plan.parsed = this.planParser.parsePlan(docs.plan.content);
      }
      if (docs.tasks?.content) {
        docs.tasks.parsed = this.tasksParser.parseTasks(docs.tasks.content);
      }

      // At minimum, we need spec.md
      if (!docs.spec?.parsed) {
        return {
          success: false,
          errors: [
            {
              code: 'MISSING_SPEC',
              message: 'spec.md content is required',
            },
          ],
        };
      }

      // Build APS plan from parsed documents
      const apsResult = this.buildAPSFromDocs(docs, spec.metadata, errors, warnings);

      if (errors.length > 0) {
        return { success: false, errors, warnings };
      }

      return {
        success: true,
        data: apsResult,
        warnings: warnings.length > 0 ? warnings : undefined,
      };
    } catch (error) {
      errors.push({
        code: 'CONVERSION_ERROR',
        message: error instanceof Error ? error.message : 'Unknown conversion error',
      });
      return { success: false, errors };
    }
  }

  async convertFromAPS(_spec: APSPlan): Promise<ConversionResult<ExternalSpec>> {
    return {
      success: false,
      errors: [
        {
          code: 'NOT_IMPLEMENTED',
          message: 'Export to SpecKit format is handled by speckit-export adapter',
        },
      ],
    };
  }

  private buildAPSFromDocs(
    docs: SpecKitDocuments,
    metadata: Record<string, unknown> | undefined,
    errors: ConversionError[],
    warnings: ConversionWarning[]
  ): APSPlan {
    const parsedSpec = docs.spec!.parsed!;
    const parsedPlan = docs.plan?.parsed;
    const parsedTasks = docs.tasks?.parsed;

    // Build intent from spec user scenarios and metadata
    const intent = this.buildIntent(parsedSpec);

    // Build proposed changes from user scenarios + plan details + tasks
    const changes = this.buildProposedChanges(parsedSpec, parsedPlan, parsedTasks, warnings);

    // Build provenance
    const provenance: Provenance = {
      timestamp: (metadata?.['timestamp'] as string) || new Date().toISOString(),
      source: 'cli',
      version: this.version,
      author: (metadata?.['author'] as string) || parsedSpec.metadata.branch,
      repository: metadata?.['repository'] as string,
      branch: parsedSpec.metadata.branch as string,
      commit: metadata?.['commit'] as string,
    };

    const planId = `aps-${Date.now().toString(16).substring(0, 8)}`;

    return {
      ...createPlan({
        id: planId,
        intent,
        provenance,
        changes,
      }),
      schema_version: '0.1.0' as const,
      hash: '0'.repeat(64),
      metadata: {
        source_format: 'speckit-v2',
        feature: parsedSpec.metadata.feature,
        // From spec.md
        userScenarios: parsedSpec.userScenarios,
        requirements: parsedSpec.requirements,
        successCriteria: parsedSpec.successCriteria,
        clarifications: parsedSpec.clarifications,
        // From plan.md
        technicalContext: parsedPlan?.technicalContext,
        constitutionCheck: parsedPlan?.constitutionCheck,
        projectStructure: parsedPlan?.projectStructure,
        implementationDetails: parsedPlan?.implementationDetails
          ? Object.fromEntries(parsedPlan.implementationDetails)
          : undefined,
        complexityDecisions: parsedPlan?.complexityDecisions,
        // From tasks.md
        phases: parsedTasks?.phases,
        taskDependencies: parsedTasks?.dependencies,
        implementationStrategies: parsedTasks?.strategies,
      },
    } as APSPlan;
  }

  private buildIntent(parsedSpec: ParsedSpec): string {
    // Build intent from feature name and P1 user scenarios
    const feature = parsedSpec.metadata.feature || 'Feature';
    const p1Scenarios = parsedSpec.userScenarios.filter((s) => s.priority === 'P1');

    if (p1Scenarios.length === 0) {
      return `Implement ${feature}`;
    }

    const scenarioDescriptions = p1Scenarios
      .map((s) => `${s.asA} wants to ${s.iWantTo} so that ${s.soThat}`)
      .join('. ');

    return `${feature}: ${scenarioDescriptions}`.substring(0, 500);
  }

  private buildProposedChanges(
    parsedSpec: ParsedSpec,
    parsedPlan: ParsedPlan | undefined,
    parsedTasks: ParsedTasks | undefined,
    warnings: ConversionWarning[]
  ): Change[] {
    const changes: Change[] = [];

    // Strategy: Convert user scenarios to proposed changes
    // Each scenario represents a feature to implement
    for (const scenario of parsedSpec.userScenarios) {
      // Create a change for each P1/P2 user scenario
      if (scenario.priority === 'P1' || scenario.priority === 'P2') {
        const change: Change = {
          type: 'file_create',
          path: this.inferPathFromScenario(scenario, parsedPlan),
          description: `Implement ${scenario.title}: ${scenario.iWantTo}`,
          metadata: {
            priority: scenario.priority,
            userStory: {
              asA: scenario.asA,
              iWantTo: scenario.iWantTo,
              soThat: scenario.soThat,
            },
            acceptanceScenarios: scenario.acceptanceScenarios,
            edgeCases: scenario.edgeCases,
          },
        };

        changes.push(change);
      }
    }

    // If we have plan details, add implementation-specific changes
    if (parsedPlan) {
      // Add database migrations if mentioned
      if (parsedPlan.implementationDetails.has('Database Schema')) {
        changes.push({
          type: 'file_create',
          path: 'database/migrations/',
          description: 'Create database migrations for required tables',
          metadata: {
            section: 'Database Schema',
          },
        });
      }

      // Add API endpoints if mentioned
      if (parsedPlan.implementationDetails.has('API Endpoints')) {
        changes.push({
          type: 'file_create',
          path: this.inferAPIPath(parsedPlan),
          description: 'Implement API endpoints',
          metadata: {
            section: 'API Endpoints',
          },
        });
      }
    }

    // Add dependency installation if we have technical context
    if (
      parsedPlan?.technicalContext.dependencies &&
      parsedPlan.technicalContext.dependencies.length > 0
    ) {
      changes.push({
        type: 'dependency_add',
        path: 'package.json',
        description: `Install dependencies: ${parsedPlan.technicalContext.dependencies.slice(0, 3).join(', ')}${parsedPlan.technicalContext.dependencies.length > 3 ? '...' : ''}`,
        metadata: {
          dependencies: parsedPlan.technicalContext.dependencies,
        },
      });
    }

    if (changes.length === 0) {
      warnings.push({
        code: 'NO_CHANGES',
        message: 'No actionable changes could be derived from spec',
      });
    }

    return changes;
  }

  private inferPathFromScenario(scenario: { title: string }, plan?: ParsedPlan): string {
    const title = scenario.title.toLowerCase().replace(/\s+/g, '-');

    // Use plan structure if available
    if (plan?.projectStructure.sourceCode) {
      // Try to extract common patterns
      if (plan.projectStructure.sourceCode.includes('src/modules/')) {
        return `src/modules/${title}/`;
      }
      if (plan.projectStructure.sourceCode.includes('src/features/')) {
        return `src/features/${title}/`;
      }
    }

    return `src/${title}/`;
  }

  private inferAPIPath(plan: ParsedPlan): string {
    if (plan.projectStructure.sourceCode?.includes('controllers')) {
      return 'src/controllers/';
    }
    if (plan.projectStructure.sourceCode?.includes('routes')) {
      return 'src/routes/';
    }
    return 'src/api/';
  }

  private generateSpecTemplate(intent: string): string {
    return `# Feature: ${intent}

**Branch**: \`feature/xxx-feature-name\`
**Date**: ${new Date().toISOString().split('T')[0]}
**Status**: Draft

## User Scenarios & Testing

### P1: [User Scenario Title]
**As a** [user type]
**I want to** [action]
**So that** [benefit]

**Acceptance Scenarios:**
- [Scenario 1]
- [Scenario 2]

**Edge Cases:**
- [Edge case 1]
- [NEEDS CLARIFICATION: Question?]

## Requirements

### Functional Requirements

**FR-001**: System MUST [requirement]
**FR-002**: [NEEDS CLARIFICATION: What should happen when...]

### Key Entities

**EntityName**
- Represents: [What this entity represents]
- Key Attributes: [attribute1, attribute2]
- Relationships: [relationships to other entities]

## Success Criteria

### Quantitative Metrics
- [Metric 1]
- [Metric 2]

### Qualitative Metrics
- [Metric 1]
- [Metric 2]
`;
  }
}
