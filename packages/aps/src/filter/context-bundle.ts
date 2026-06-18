/**
 * Context Bundle Builder
 *
 * Generates context bundles for LLM consumption from filtered plans.
 * Outputs both text (Markdown) and JSON formats.
 */

import type { FilteredPlan } from './index.js';

/**
 * Context bundle in JSON format
 */
export interface ContextBundleJSON {
  /** Plan title */
  title: string;

  /** Filter criteria that was applied */
  filter: {
    scopes?: string[];
    modules?: string[];
    tasks?: string[];
    owners?: string[];
    tags?: string[];
    priorities?: string[];
    confidences?: string[];
    statuses?: string[];
  };

  /** Summary statistics */
  summary: {
    totalModules: number;
    totalTasks: number;
    tasksByConfidence: {
      high: number;
      medium: number;
      low: number;
    };
    tasksByStatus: {
      open: number;
      locked: number;
      completed: number;
      cancelled: number;
    };
  };

  /** Modules with their tasks */
  modules: Array<{
    id: string;
    scope?: string;
    owner?: string;
    priority?: string;
    tags?: string[];
    tasks: Array<{
      id: string;
      title: string;
      intent: string;
      confidence: string;
      status: string;
      scopes?: string[];
      tags?: string[];
      dependencies?: string[];
      inputs?: string[];
      expectedOutcome?: string;
    }>;
  }>;

  /** Dependency graph (module -> dependencies) */
  dependencyGraph: Record<string, string[]>;
}

/**
 * Build a context bundle from a filtered plan
 *
 * @param filtered - The filtered plan
 * @returns Context bundle in JSON format
 */
export function buildContextBundleJSON(filtered: FilteredPlan): ContextBundleJSON {
  const { plan, modules, tasks, criteria } = filtered;

  // Build summary statistics
  const summary = {
    totalModules: modules.length,
    totalTasks: tasks.length,
    tasksByConfidence: {
      high: tasks.filter((t) => t.confidence === 'high').length,
      medium: tasks.filter((t) => t.confidence === 'medium').length,
      low: tasks.filter((t) => t.confidence === 'low').length,
    },
    tasksByStatus: {
      open: tasks.filter((t) => (t.status ?? 'open') === 'open').length,
      locked: tasks.filter((t) => t.status === 'locked').length,
      completed: tasks.filter((t) => t.status === 'completed').length,
      cancelled: tasks.filter((t) => t.status === 'cancelled').length,
    },
  };

  // Build module data with tasks
  const moduleData = modules.map((module) => {
    const moduleTasks = tasks.filter((t) => module.tasks.some((mt) => mt.id === t.id));

    return {
      id: module.id,
      scope: module.metadata.scope,
      owner: module.metadata.owner,
      priority: module.metadata.priority,
      tags: module.metadata.tags,
      tasks: moduleTasks.map((t) => ({
        id: t.id,
        title: t.title,
        intent: t.intent,
        confidence: t.confidence,
        status: t.status ?? 'open',
        scopes: t.scopes,
        tags: t.tags,
        dependencies: t.dependencies,
        inputs: t.inputs,
        expectedOutcome: t.expectedOutcome,
      })),
    };
  });

  // Build dependency graph for filtered modules
  const dependencyGraph: Record<string, string[]> = {};
  for (const module of modules) {
    const deps = plan.dependencyGraph.get(module.id) ?? [];
    // Only include dependencies that are in the filtered set
    const filteredDeps = deps.filter((d) => modules.some((m) => m.id === d));
    dependencyGraph[module.id] = filteredDeps;
  }

  return {
    title: plan.title,
    filter: {
      scopes: criteria.scopes,
      modules: criteria.modules,
      tasks: criteria.tasks,
      owners: criteria.owners,
      tags: criteria.tags,
      priorities: criteria.priorities,
      confidences: criteria.confidences,
      statuses: criteria.statuses,
    },
    summary,
    modules: moduleData,
    dependencyGraph,
  };
}

/**
 * Build a context bundle in Markdown text format
 *
 * @param filtered - The filtered plan
 * @returns Markdown text suitable for LLM context
 */
export function buildContextBundleText(filtered: FilteredPlan): string {
  const { plan, modules, tasks, criteria } = filtered;
  const lines: string[] = [];

  // Header
  lines.push(`# ${plan.title}`);
  lines.push('');

  // Filter info
  const filterParts: string[] = [];
  if (criteria.scopes?.length) filterParts.push(`scopes: ${criteria.scopes.join(', ')}`);
  if (criteria.modules?.length) filterParts.push(`modules: ${criteria.modules.join(', ')}`);
  if (criteria.tasks?.length) filterParts.push(`tasks: ${criteria.tasks.join(', ')}`);
  if (criteria.owners?.length) filterParts.push(`owners: ${criteria.owners.join(', ')}`);
  if (criteria.tags?.length) filterParts.push(`tags: ${criteria.tags.join(', ')}`);
  if (criteria.priorities?.length)
    filterParts.push(`priorities: ${criteria.priorities.join(', ')}`);
  if (criteria.confidences?.length)
    filterParts.push(`confidences: ${criteria.confidences.join(', ')}`);
  if (criteria.statuses?.length) filterParts.push(`statuses: ${criteria.statuses.join(', ')}`);

  if (filterParts.length > 0) {
    lines.push(`> Filtered by: ${filterParts.join('; ')}`);
    lines.push('');
  }

  // Summary
  lines.push('## Summary');
  lines.push('');
  lines.push(`- **Modules:** ${modules.length}`);
  lines.push(`- **Tasks:** ${tasks.length}`);

  const highConfidence = tasks.filter((t) => t.confidence === 'high').length;
  const mediumConfidence = tasks.filter((t) => t.confidence === 'medium').length;
  const lowConfidence = tasks.filter((t) => t.confidence === 'low').length;
  lines.push(
    `- **Confidence:** ${highConfidence} high, ${mediumConfidence} medium, ${lowConfidence} low`
  );

  const openTasks = tasks.filter((t) => (t.status ?? 'open') === 'open').length;
  const lockedTasks = tasks.filter((t) => t.status === 'locked').length;
  const completedTasks = tasks.filter((t) => t.status === 'completed').length;
  lines.push(`- **Status:** ${openTasks} open, ${lockedTasks} locked, ${completedTasks} completed`);
  lines.push('');

  // Modules and tasks
  for (const module of modules) {
    lines.push(`## Module: ${module.id}`);
    lines.push('');

    if (module.metadata.scope) lines.push(`**Scope:** ${module.metadata.scope}`);
    if (module.metadata.owner) lines.push(`**Owner:** ${module.metadata.owner}`);
    if (module.metadata.priority) lines.push(`**Priority:** ${module.metadata.priority}`);
    if (module.metadata.tags?.length) lines.push(`**Tags:** ${module.metadata.tags.join(', ')}`);

    const deps = plan.dependencyGraph.get(module.id) ?? [];
    if (deps.length > 0) lines.push(`**Dependencies:** ${deps.join(', ')}`);
    lines.push('');

    // Tasks for this module
    const moduleTasks = tasks.filter((t) => module.tasks.some((mt) => mt.id === t.id));

    if (moduleTasks.length === 0) {
      lines.push('*No tasks match the filter criteria.*');
      lines.push('');
      continue;
    }

    lines.push('### Tasks');
    lines.push('');

    for (const task of moduleTasks) {
      lines.push(`#### ${task.id}: ${task.title}`);
      lines.push('');
      lines.push(`**Intent:** ${task.intent}`);
      lines.push(`**Confidence:** ${task.confidence}`);
      lines.push(`**Status:** ${task.status ?? 'open'}`);

      if (task.scopes?.length) lines.push(`**Scopes:** ${task.scopes.join(', ')}`);
      if (task.tags?.length) lines.push(`**Tags:** ${task.tags.join(', ')}`);
      if (task.dependencies?.length)
        lines.push(`**Dependencies:** ${task.dependencies.join(', ')}`);
      if (task.expectedOutcome) lines.push(`**Expected Outcome:** ${task.expectedOutcome}`);

      if (task.inputs?.length) {
        lines.push('**Inputs:**');
        for (const input of task.inputs) {
          lines.push(`- ${input}`);
        }
      }

      lines.push('');
    }
  }

  return lines.join('\n');
}

/**
 * Build context for a single task (focused view)
 */
export function buildTaskContext(filtered: FilteredPlan, taskId: string): string | null {
  const task = filtered.tasks.find((t) => t.id === taskId);
  if (!task) return null;

  const lines: string[] = [];

  lines.push(`# Task: ${task.id}`);
  lines.push('');
  lines.push(`## ${task.title}`);
  lines.push('');
  lines.push(`**Intent:** ${task.intent}`);
  lines.push('');
  lines.push(`**Confidence:** ${task.confidence}`);
  lines.push(`**Status:** ${task.status ?? 'open'}`);

  if (task.scopes?.length) {
    lines.push('');
    lines.push(`**Scopes:** ${task.scopes.join(', ')}`);
    lines.push('');
    lines.push('> These scopes define what files/modules this task is allowed to modify.');
  }

  if (task.tags?.length) {
    lines.push('');
    lines.push(`**Tags:** ${task.tags.join(', ')}`);
  }

  if (task.dependencies?.length) {
    lines.push('');
    lines.push('## Dependencies');
    lines.push('');
    lines.push('This task depends on:');
    for (const dep of task.dependencies) {
      const depTask = filtered.plan.allTasks.find((t) => t.id === dep);
      if (depTask) {
        lines.push(`- **${dep}:** ${depTask.title} (${depTask.status ?? 'open'})`);
      } else {
        lines.push(`- **${dep}:** (not found)`);
      }
    }
  }

  if (task.inputs?.length) {
    lines.push('');
    lines.push('## Inputs');
    lines.push('');
    for (const input of task.inputs) {
      lines.push(`- ${input}`);
    }
  }

  if (task.expectedOutcome) {
    lines.push('');
    lines.push('## Expected Outcome');
    lines.push('');
    lines.push(task.expectedOutcome);
  }

  return lines.join('\n');
}
