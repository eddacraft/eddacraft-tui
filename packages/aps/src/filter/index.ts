/**
 * Filter module - Task and module filtering for APS plans
 *
 * Provides filtering capabilities for:
 * - Scope-based filtering (by module scope or task scopes)
 * - Module-based filtering (by module ID)
 * - Task-based filtering (by task ID)
 * - Metadata filtering (owner, tags, priority, confidence)
 *
 * Also provides context bundle generation for LLM consumption.
 */

// Re-export context bundle functions
export {
  buildContextBundleJSON,
  buildContextBundleText,
  buildTaskContext,
  type ContextBundleJSON,
} from './context-bundle.js';

import type { LoadedPlan, LoadedModule } from '../loader/index.js';
import type { Task, Priority, Confidence } from '../types/index.js';

/**
 * Filter criteria for tasks and modules
 */
export interface FilterCriteria {
  /** Filter by scope (matches module scope or task scopes) */
  scopes?: string[];

  /** Filter by module ID */
  modules?: string[];

  /** Filter by specific task IDs */
  tasks?: string[];

  /** Filter by owner (e.g., @alice) */
  owners?: string[];

  /** Filter by tags (matches any) */
  tags?: string[];

  /** Filter by priority levels */
  priorities?: Priority[];

  /** Filter by confidence levels */
  confidences?: Confidence[];

  /** Filter by task status */
  statuses?: Array<'open' | 'locked' | 'completed' | 'cancelled'>;
}

/**
 * Result of filtering a plan
 */
export interface FilteredPlan {
  /** Original plan reference */
  plan: LoadedPlan;

  /** Filtered modules (only those matching criteria) */
  modules: LoadedModule[];

  /** Filtered tasks (only those matching criteria) */
  tasks: Task[];

  /** Applied filter criteria */
  criteria: FilterCriteria;
}

/**
 * Filter a loaded plan by the given criteria
 *
 * @param plan - The loaded plan to filter
 * @param criteria - Filter criteria to apply
 * @returns Filtered plan with matching modules and tasks
 */
export function filterPlan(plan: LoadedPlan, criteria: FilterCriteria): FilteredPlan {
  // Start with all modules and tasks
  let filteredModules = Array.from(plan.modules.values());
  let filteredTasks = [...plan.allTasks];

  // Apply module-level filters first
  if (criteria.modules && criteria.modules.length > 0) {
    filteredModules = filteredModules.filter((m) => criteria.modules!.includes(m.id));
    // Also filter tasks to only those in matching modules
    const moduleIds = new Set(criteria.modules);
    filteredTasks = filteredTasks.filter((t) => {
      const taskModule = findTaskModule(plan, t.id);
      return taskModule && moduleIds.has(taskModule.id);
    });
  }

  // Apply scope filter (matches module scope OR task scopes)
  if (criteria.scopes && criteria.scopes.length > 0) {
    const scopeSet = new Set(criteria.scopes.map((s) => s.toUpperCase()));

    filteredModules = filteredModules.filter((m) => {
      const moduleScope = m.metadata.scope?.toUpperCase();
      return moduleScope && scopeSet.has(moduleScope);
    });

    filteredTasks = filteredTasks.filter((t) => {
      // Check if any of the task's scopes match
      if (t.scopes && t.scopes.length > 0) {
        return t.scopes.some((s) => scopeSet.has(s.toUpperCase()));
      }
      // Fall back to checking if task ID prefix matches scope
      const taskScope = t.id.split('-')[0];
      return scopeSet.has(taskScope);
    });
  }

  // Apply task ID filter
  if (criteria.tasks && criteria.tasks.length > 0) {
    const taskIdSet = new Set(criteria.tasks);
    filteredTasks = filteredTasks.filter((t) => taskIdSet.has(t.id));
  }

  // Apply owner filter
  if (criteria.owners && criteria.owners.length > 0) {
    const ownerSet = new Set(criteria.owners.map((o) => o.toLowerCase()));

    filteredModules = filteredModules.filter((m) => {
      const owner = m.metadata.owner?.toLowerCase();
      return owner && ownerSet.has(owner);
    });

    // For tasks, filter by the module owner (tasks don't have individual owners)
    filteredTasks = filteredTasks.filter((t) => {
      const taskModule = findTaskModule(plan, t.id);
      if (!taskModule) return false;
      const owner = taskModule.metadata.owner?.toLowerCase();
      return owner && ownerSet.has(owner);
    });
  }

  // Apply tag filter (matches any tag)
  if (criteria.tags && criteria.tags.length > 0) {
    const tagSet = new Set(criteria.tags.map((t) => t.toLowerCase()));

    filteredModules = filteredModules.filter((m) => {
      const moduleTags = m.metadata.tags?.map((t) => t.toLowerCase()) ?? [];
      return moduleTags.some((t) => tagSet.has(t));
    });

    filteredTasks = filteredTasks.filter((t) => {
      const taskTags = t.tags?.map((tag) => tag.toLowerCase()) ?? [];
      return taskTags.some((tag) => tagSet.has(tag));
    });
  }

  // Apply priority filter
  if (criteria.priorities && criteria.priorities.length > 0) {
    const prioritySet = new Set(criteria.priorities);

    filteredModules = filteredModules.filter((m) => {
      return m.metadata.priority && prioritySet.has(m.metadata.priority);
    });

    // Tasks don't have priority, so filter by module priority
    filteredTasks = filteredTasks.filter((t) => {
      const taskModule = findTaskModule(plan, t.id);
      return taskModule?.metadata.priority && prioritySet.has(taskModule.metadata.priority);
    });
  }

  // Apply confidence filter
  if (criteria.confidences && criteria.confidences.length > 0) {
    const confidenceSet = new Set(criteria.confidences);
    filteredTasks = filteredTasks.filter((t) => confidenceSet.has(t.confidence));
  }

  // Apply status filter
  if (criteria.statuses && criteria.statuses.length > 0) {
    const statusSet = new Set(criteria.statuses);
    filteredTasks = filteredTasks.filter((t) => {
      const status = t.status ?? 'open';
      return statusSet.has(status);
    });
  }

  return {
    plan,
    modules: filteredModules,
    tasks: filteredTasks,
    criteria,
  };
}

/**
 * Find the module that contains a task
 */
function findTaskModule(plan: LoadedPlan, taskId: string): LoadedModule | undefined {
  for (const module of plan.modules.values()) {
    if (module.tasks.some((t) => t.id === taskId)) {
      return module;
    }
  }
  return undefined;
}

/**
 * Filter tasks by scope (convenience function)
 */
export function filterByScope(plan: LoadedPlan, scopes: string[]): Task[] {
  return filterPlan(plan, { scopes }).tasks;
}

/**
 * Filter tasks by module (convenience function)
 */
export function filterByModule(plan: LoadedPlan, moduleIds: string[]): Task[] {
  return filterPlan(plan, { modules: moduleIds }).tasks;
}

/**
 * Filter tasks by tags (convenience function)
 */
export function filterByTags(plan: LoadedPlan, tags: string[]): Task[] {
  return filterPlan(plan, { tags }).tasks;
}

/**
 * Filter tasks by owner (convenience function)
 */
export function filterByOwner(plan: LoadedPlan, owners: string[]): Task[] {
  return filterPlan(plan, { owners }).tasks;
}

/**
 * Filter tasks by priority (convenience function)
 */
export function filterByPriority(plan: LoadedPlan, priorities: Priority[]): Task[] {
  return filterPlan(plan, { priorities }).tasks;
}

/**
 * Filter tasks by confidence (convenience function)
 */
export function filterByConfidence(plan: LoadedPlan, confidences: Confidence[]): Task[] {
  return filterPlan(plan, { confidences }).tasks;
}

/**
 * Get tasks matching specific IDs
 */
export function getTasksById(plan: LoadedPlan, taskIds: string[]): Task[] {
  return filterPlan(plan, { tasks: taskIds }).tasks;
}
