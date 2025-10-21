import type { APSPlan, Change, ValidationResult } from '@anvil/core';
import type {
  ConversionError,
  ConversionResult,
  ConversionWarning,
  ExternalSpec,
  SpecContext,
} from '../common/types.js';
import { BaseAdapter } from '../common/types.js';

export class SpecKitExportAdapter extends BaseAdapter {
  readonly name = 'speckit-export';
  readonly version = '1.0.0';
  readonly supportedFormats = ['speckit', 'spec.md'] as const;

  async generateSpec(_intent: string, _context: SpecContext): Promise<APSPlan> {
    throw new Error('Use speckit-import adapter for generating new specs');
  }

  async validateSpec(spec: APSPlan): Promise<ValidationResult> {
    const issues: Array<{
      path: string;
      message: string;
      code: string;
      severity: 'error' | 'warning';
    }> = [];

    if (!spec.intent || spec.intent.length < 10) {
      issues.push({
        path: 'intent',
        message: 'Intent is required and must be at least 10 characters',
        code: 'INVALID_INTENT',
        severity: 'error',
      });
    }

    if (spec.proposed_changes.length === 0) {
      issues.push({
        path: 'proposed_changes',
        message: 'No changes to export',
        code: 'EMPTY_CHANGES',
        severity: 'warning',
      });
    }

    return {
      valid: issues.length === 0,
      data: spec,
      issues: issues.length > 0 ? issues : undefined,
      summary: issues.length === 0 ? 'Validation passed' : `Found ${issues.length} issue(s)`,
    };
  }

  async convertToAPS(_spec: ExternalSpec): Promise<ConversionResult<APSPlan>> {
    return {
      success: false,
      errors: [
        {
          code: 'NOT_IMPLEMENTED',
          message: 'Import from SpecKit format is handled by speckit-import adapter',
        },
      ],
    };
  }

  async convertFromAPS(spec: APSPlan): Promise<ConversionResult<ExternalSpec>> {
    const errors: ConversionError[] = [];
    const warnings: ConversionWarning[] = [];

    try {
      const specContent = this.generateSpecMarkdown(spec);
      const planContent = this.generatePlanMarkdown(spec);
      const tasksContent = this.generateTasksMarkdown(spec);

      const result: ExternalSpec = {
        format: 'speckit',
        version: '1.0.0',
        content: {
          specContent,
          planContent,
          tasksContent,
          metadata: spec.metadata,
        },
        metadata: {
          generated_at: new Date().toISOString(),
          generator: this.name,
          generator_version: this.version,
          aps_id: spec.id,
          aps_hash: spec.hash,
        },
      };

      return {
        success: true,
        data: result,
        warnings: warnings.length > 0 ? warnings : undefined,
      };
    } catch (error) {
      errors.push({
        code: 'EXPORT_ERROR',
        message: error instanceof Error ? error.message : 'Failed to export to SpecKit format',
      });
      return { success: false, errors };
    }
  }

  private generateSpecMarkdown(spec: APSPlan): string {
    const sections: string[] = [];

    sections.push(`# Specification`);
    sections.push('');

    sections.push(`## Intent`);
    sections.push('');
    sections.push(spec.intent);
    sections.push('');

    if (spec.metadata?.['overview']) {
      sections.push(`## Overview`);
      sections.push('');
      sections.push(spec.metadata['overview'] as string);
      sections.push('');
    }

    if (spec.metadata?.['goals'] && Array.isArray(spec.metadata['goals'])) {
      sections.push(`## Goals`);
      sections.push('');
      for (const goal of spec.metadata['goals'] as string[]) {
        sections.push(`- ${goal}`);
      }
      sections.push('');
    }

    if (spec.metadata?.['requirements'] && Array.isArray(spec.metadata['requirements'])) {
      sections.push(`## Requirements`);
      sections.push('');
      for (const req of spec.metadata['requirements'] as string[]) {
        sections.push(`- ${req}`);
      }
      sections.push('');
    }

    if (spec.proposed_changes.length > 0) {
      sections.push(`## Changes`);
      sections.push('');

      const changesByType = this.groupChangesByType(spec.proposed_changes);

      for (const [type, changes] of Object.entries(changesByType)) {
        if (changes.length > 0) {
          sections.push(`### ${this.formatChangeType(type)}`);
          sections.push('');

          for (const change of changes) {
            sections.push(`#### ${change.description}`);
            sections.push('');

            if (change.path) {
              sections.push(`Path: \`${change.path}\``);
              sections.push('');
            }

            if (change.content) {
              const extension = this.getFileExtension(change.path || '');
              sections.push(`\`\`\`${extension}`);
              sections.push(change.content);
              sections.push(`\`\`\``);
              sections.push('');
            }

            if (change.diff) {
              sections.push(`\`\`\`diff`);
              sections.push(change.diff);
              sections.push(`\`\`\``);
              sections.push('');
            }
          }
        }
      }
    }

    if (spec.metadata) {
      const customMetadata = this.filterCustomMetadata(spec.metadata);
      if (Object.keys(customMetadata).length > 0) {
        sections.push(`## Metadata`);
        sections.push('');
        sections.push(`\`\`\`json`);
        sections.push(JSON.stringify(customMetadata, null, 2));
        sections.push(`\`\`\``);
        sections.push('');
      }
    }

    return sections.join('\n');
  }

  private generatePlanMarkdown(spec: APSPlan): string {
    const sections: string[] = [];

    sections.push(`# Implementation Plan`);
    sections.push('');
    sections.push(`Generated from APS: ${spec.id}`);
    sections.push('');

    sections.push(`## Summary`);
    sections.push('');
    sections.push(spec.intent);
    sections.push('');

    sections.push(`## Implementation Steps`);
    sections.push('');

    const steps = this.convertChangesToSteps(spec.proposed_changes);
    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      sections.push(`${i + 1}. **${step.title}**`);
      if (step.details) {
        sections.push(`   - ${step.details}`);
      }
      if (step.dependencies.length > 0) {
        sections.push(`   - Dependencies: ${step.dependencies.join(', ')}`);
      }
      sections.push('');
    }

    if (spec.validations) {
      sections.push(`## Validation Requirements`);
      sections.push('');
      sections.push(`- Required checks: ${spec.validations.required_checks.join(', ')}`);
      if (spec.validations.skip_checks.length > 0) {
        sections.push(`- Skipped checks: ${spec.validations.skip_checks.join(', ')}`);
      }
      sections.push('');
    }

    return sections.join('\n');
  }

  private generateTasksMarkdown(spec: APSPlan): string {
    const sections: string[] = [];

    sections.push(`# Tasks`);
    sections.push('');
    sections.push(`Generated from APS: ${spec.id}`);
    sections.push(`Last updated: ${new Date().toISOString()}`);
    sections.push('');

    sections.push(`## Task List`);
    sections.push('');

    const tasks = this.convertChangesToTasks(spec.proposed_changes);
    let completedCount = 0;

    for (const task of tasks) {
      const status = task.completed ? 'x' : ' ';
      const statusEmoji = task.completed ? '✅' : '⏳';
      sections.push(`- [${status}] ${statusEmoji} ${task.description}`);

      if (task.subtasks.length > 0) {
        for (const subtask of task.subtasks) {
          const subStatus = subtask.completed ? 'x' : ' ';
          sections.push(`  - [${subStatus}] ${subtask.description}`);
        }
      }

      if (task.completed) {
        completedCount++;
      }
    }

    sections.push('');
    sections.push(`## Progress`);
    sections.push('');
    sections.push(`- Total tasks: ${tasks.length}`);
    sections.push(`- Completed: ${completedCount}`);
    sections.push(`- Remaining: ${tasks.length - completedCount}`);
    sections.push(`- Progress: ${Math.round((completedCount / tasks.length) * 100)}%`);
    sections.push('');

    if (spec.executions && spec.executions.length > 0) {
      sections.push(`## Execution History`);
      sections.push('');

      for (const execution of spec.executions) {
        sections.push(`### ${new Date(execution.timestamp).toLocaleString()}`);
        sections.push(`- Operation: ${execution.operation}`);
        sections.push(`- Status: ${execution.status}`);
        if (execution.executed_by) {
          sections.push(`- Executed by: ${execution.executed_by}`);
        }
        if (execution.changes_failed && execution.changes_failed.length > 0) {
          sections.push(`- Failed changes: ${execution.changes_failed.join(', ')}`);
        }
        if (execution.logs && execution.logs.length > 0) {
          sections.push(`- Logs: ${execution.logs.length} entries`);
        }
        sections.push('');
      }
    }

    return sections.join('\n');
  }

  private groupChangesByType(changes: Change[]): Record<string, Change[]> {
    const groups: Record<string, Change[]> = {};

    for (const change of changes) {
      if (!groups[change.type]) {
        groups[change.type] = [];
      }
      groups[change.type].push(change);
    }

    return groups;
  }

  private formatChangeType(type: string): string {
    const typeMap: Record<string, string> = {
      file_create: 'Files to Create',
      file_update: 'Files to Update',
      file_delete: 'Files to Delete',
      config_update: 'Configuration Changes',
      dependency_add: 'Dependencies to Add',
      dependency_remove: 'Dependencies to Remove',
      dependency_update: 'Dependencies to Update',
      script_execute: 'Scripts to Execute',
    };

    return typeMap[type] || type.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase());
  }

  private getFileExtension(path: string): string {
    const ext = path.split('.').pop()?.toLowerCase();
    const extMap: Record<string, string> = {
      js: 'javascript',
      ts: 'typescript',
      jsx: 'javascript',
      tsx: 'typescript',
      py: 'python',
      rb: 'ruby',
      go: 'go',
      rs: 'rust',
      java: 'java',
      cpp: 'cpp',
      c: 'c',
      h: 'c',
      hpp: 'cpp',
      cs: 'csharp',
      php: 'php',
      swift: 'swift',
      kt: 'kotlin',
      scala: 'scala',
      r: 'r',
      m: 'matlab',
      sh: 'bash',
      ps1: 'powershell',
      sql: 'sql',
      md: 'markdown',
      yml: 'yaml',
      yaml: 'yaml',
      json: 'json',
      xml: 'xml',
      html: 'html',
      css: 'css',
      scss: 'scss',
      less: 'less',
    };

    return extMap[ext || ''] || ext || 'text';
  }

  private filterCustomMetadata(metadata: Record<string, unknown>): Record<string, unknown> {
    const standardKeys = ['overview', 'goals', 'requirements', 'source_format'];

    const custom: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(metadata)) {
      if (!standardKeys.includes(key)) {
        custom[key] = value;
      }
    }

    return custom;
  }

  private convertChangesToSteps(changes: Change[]): Array<{
    title: string;
    details: string;
    dependencies: string[];
  }> {
    const steps: Array<{
      title: string;
      details: string;
      dependencies: string[];
    }> = [];

    const pathDependencies = new Map<string, number>();

    for (let i = 0; i < changes.length; i++) {
      const change = changes[i];
      const deps: string[] = [];

      if (change.path) {
        const previousIndex = pathDependencies.get(change.path);
        if (previousIndex !== undefined) {
          deps.push(`Step ${previousIndex + 1}`);
        }
        pathDependencies.set(change.path, i);
      }

      steps.push({
        title: change.description,
        details: change.path ? `Target: ${change.path}` : '',
        dependencies: deps,
      });
    }

    return steps;
  }

  private convertChangesToTasks(changes: Change[]): Array<{
    description: string;
    completed: boolean;
    subtasks: Array<{
      description: string;
      completed: boolean;
    }>;
  }> {
    const tasks: Array<{
      description: string;
      completed: boolean;
      subtasks: Array<{
        description: string;
        completed: boolean;
      }>;
    }> = [];

    for (const change of changes) {
      const task = {
        description: change.description,
        completed: false,
        subtasks: [] as Array<{
          description: string;
          completed: boolean;
        }>,
      };

      if (change.type === 'file_create' || change.type === 'file_update') {
        task.subtasks.push({
          description: `Edit ${change.path || 'file'}`,
          completed: false,
        });

        if (this.config.preserveMetadata) {
          task.subtasks.push({
            description: 'Add file metadata',
            completed: false,
          });
        }
      }

      if (change.type.startsWith('dependency_')) {
        task.subtasks.push({
          description: 'Update package manifest',
          completed: false,
        });
        task.subtasks.push({
          description: 'Run package manager',
          completed: false,
        });
      }

      tasks.push(task);
    }

    return tasks;
  }
}
