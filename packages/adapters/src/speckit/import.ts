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
import { SpecKitParser } from './parser.js';

interface SpecKitSpec {
  specContent?: string;
  planContent?: string;
  tasksContent?: string;
  metadata?: Record<string, unknown>;
}

export class SpecKitImportAdapter extends BaseAdapter {
  readonly name = 'speckit-import';
  readonly version = '1.0.0';
  readonly supportedFormats = ['speckit', 'spec.md'] as const;

  private parser: SpecKitParser;

  constructor(config: AdapterConfig = {}) {
    super(config);
    this.parser = new SpecKitParser();
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

    const changes: Change[] = [
      {
        type: 'file_create',
        path: 'spec.md',
        description: 'Create initial specification file',
        content: `# Specification\n\n## Intent\n\n${intent}\n\n## Overview\n\n[Describe the overall approach]\n\n## Requirements\n\n- [List prerequisites]\n\n## Changes\n\n- [List proposed changes]\n`,
      },
    ];

    const planId = `aps-${Date.now().toString(16).substring(0, 8)}`;

    const plan = {
      ...createPlan({
        id: planId,
        intent,
        provenance,
        changes,
      }),
      schema_version: '0.1.0' as const,
      hash: '0'.repeat(64), // Placeholder hash
    } as APSPlan;
    return plan;
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

    for (let i = 0; i < spec.proposed_changes.length; i++) {
      const change = spec.proposed_changes[i];
      if (!change.description || change.description.length < 10) {
        warnings.push({
          field: `proposed_changes[${i}].description`,
          message: 'Change description is too short or missing',
        });
      }

      if (!change.path && change.type !== 'script_execute') {
        errors.push({
          field: `proposed_changes[${i}].path`,
          message: `Path is required for change type '${change.type}'`,
        });
      }
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
      const specKitSpec = spec.content as SpecKitSpec;

      if (!specKitSpec.specContent) {
        errors.push({
          code: 'MISSING_SPEC_CONTENT',
          message: 'spec.md content is required',
        });
        return { success: false, errors };
      }

      const parsed = this.parser.parseSpecMarkdown(specKitSpec.specContent);

      if (!parsed.intent && !parsed.overview) {
        errors.push({
          code: 'MISSING_INTENT',
          message: 'No intent or overview section found in spec.md',
        });
      } else if (!parsed.intent) {
        warnings.push({
          code: 'MISSING_INTENT',
          message: 'No intent section found, using overview as fallback',
        });
      }

      const changes = this.convertChangesToAPS(parsed.changes || [], errors, warnings);

      if (errors.length > 0) {
        return { success: false, errors, warnings };
      }

      const intent = parsed.intent || parsed.overview || 'Specification from SpecKit';

      const provenance: Provenance = {
        timestamp: (spec.metadata?.['timestamp'] as string) || new Date().toISOString(),
        source: 'cli',
        version: this.version,
        author: spec.metadata?.['author'] as string,
        repository: spec.metadata?.['repository'] as string,
        branch: spec.metadata?.['branch'] as string,
        commit: spec.metadata?.['commit'] as string,
      };

      const planId = `aps-${Date.now().toString(16).substring(0, 8)}`;

      try {
        const plan = {
          ...createPlan({
            id: planId,
            intent: intent.substring(0, 500),
            provenance,
            changes,
          }),
          schema_version: '0.1.0' as const,
          hash: '0'.repeat(64), // Placeholder hash
          metadata: {
            ...parsed.metadata,
            source_format: 'speckit',
            goals: parsed.goals,
            requirements: parsed.requirements,
            overview: parsed.overview,
          },
        } as APSPlan;
        return {
          success: true,
          data: plan,
          warnings: warnings.length > 0 ? warnings : undefined,
        };
      } catch (error) {
        errors.push({
          code: 'APS_CREATION_FAILED',
          message: error instanceof Error ? error.message : 'Failed to create APS plan',
        });
        return { success: false, errors };
      }
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

  private convertChangesToAPS(
    changes: Array<{ type: string; description: string; path?: string; content?: string }>,
    errors: ConversionError[],
    warnings: ConversionWarning[]
  ): Change[] {
    const apsChanges: Change[] = [];

    for (let i = 0; i < changes.length; i++) {
      const change = changes[i];

      if (!change.description) {
        warnings.push({
          code: 'EMPTY_DESCRIPTION',
          message: `Change ${i + 1} has no description`,
          path: `changes[${i}]`,
        });
        continue;
      }

      const validTypes = [
        'file_create',
        'file_update',
        'file_delete',
        'config_update',
        'dependency_add',
        'dependency_remove',
        'dependency_update',
        'script_execute',
      ];

      if (!validTypes.includes(change.type)) {
        warnings.push({
          code: 'UNKNOWN_CHANGE_TYPE',
          message: `Unknown change type '${change.type}', defaulting to 'script_execute'`,
          path: `changes[${i}].type`,
        });
        change.type = 'script_execute';
      }

      const apsChange: Change = {
        type: change.type as Change['type'],
        path: change.path || '',
        description: change.description,
      };

      if (change.content) {
        apsChange.content = change.content;
      }

      if (!apsChange.path && apsChange.type !== 'script_execute') {
        warnings.push({
          code: 'MISSING_PATH',
          message: `Path not specified for ${apsChange.type}, using placeholder`,
          path: `changes[${i}].path`,
        });
        apsChange.path = '<path-to-be-specified>';
      }

      apsChanges.push(apsChange);
    }

    if (apsChanges.length === 0) {
      warnings.push({
        code: 'NO_CHANGES',
        message: 'No valid changes found in specification',
      });
    }

    return apsChanges;
  }
}
