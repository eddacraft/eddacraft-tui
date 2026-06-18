/**
 * Tests for filter module
 */

import { describe, it, expect } from 'vitest';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadPlan } from '../loader/index.js';
import {
  filterPlan,
  filterByScope,
  filterByModule,
  filterByTags,
  filterByOwner,
  filterByPriority,
  filterByConfidence,
  getTasksById,
  buildContextBundleJSON,
  buildContextBundleText,
  buildTaskContext,
} from './index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EXAMPLES_DIR = join(__dirname, '../../examples');

describe('filterPlan', () => {
  describe('scope filtering', () => {
    it('should filter tasks by scope', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { scopes: ['AUTH'] });

      expect(filtered.modules.length).toBe(1);
      expect(filtered.modules[0].id).toBe('auth');
      // Tasks are filtered by having AUTH in their scopes array, not just ID prefix
      // This includes AUTH-* tasks and any task that lists AUTH in its scopes
      expect(
        filtered.tasks.every((t) => t.scopes?.includes('AUTH') || t.id.startsWith('AUTH-'))
      ).toBe(true);
    });

    it('should filter by multiple scopes', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { scopes: ['AUTH', 'PROD'] });

      expect(filtered.modules.length).toBe(2);
      const moduleIds = filtered.modules.map((m) => m.id);
      expect(moduleIds).toContain('auth');
      expect(moduleIds).toContain('products');
    });

    it('should be case-insensitive for scopes', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { scopes: ['auth'] });

      expect(filtered.modules.length).toBe(1);
      expect(filtered.modules[0].id).toBe('auth');
    });
  });

  describe('module filtering', () => {
    it('should filter by module ID', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { modules: ['cart'] });

      expect(filtered.modules.length).toBe(1);
      expect(filtered.modules[0].id).toBe('cart');
      expect(filtered.tasks.every((t) => t.id.startsWith('CART-'))).toBe(true);
    });

    it('should filter by multiple modules', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { modules: ['auth', 'payments'] });

      expect(filtered.modules.length).toBe(2);
    });
  });

  describe('task filtering', () => {
    it('should filter by specific task IDs', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { tasks: ['AUTH-001', 'AUTH-002'] });

      expect(filtered.tasks.length).toBe(2);
      expect(filtered.tasks.map((t) => t.id)).toEqual(['AUTH-001', 'AUTH-002']);
    });
  });

  describe('owner filtering', () => {
    it('should filter by owner', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { owners: ['@alice'] });

      expect(filtered.modules.length).toBe(1);
      expect(filtered.modules[0].metadata.owner).toBe('@alice');
    });

    it('should be case-insensitive for owners', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { owners: ['@ALICE'] });

      expect(filtered.modules.length).toBe(1);
    });
  });

  describe('tag filtering', () => {
    it('should filter by tags', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { tags: ['security'] });

      // Auth module has 'security' tag
      expect(filtered.modules.some((m) => m.id === 'auth')).toBe(true);
    });

    it('should match any tag (OR logic)', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { tags: ['security', 'billing'] });

      // Should match both auth (security) and payments (billing)
      expect(filtered.modules.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('priority filtering', () => {
    it('should filter by priority', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { priorities: ['high'] });

      expect(filtered.modules.every((m) => m.metadata.priority === 'high')).toBe(true);
    });

    it('should filter by multiple priorities', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { priorities: ['high', 'medium'] });

      expect(filtered.modules.length).toBeGreaterThan(0);
    });
  });

  describe('confidence filtering', () => {
    it('should filter tasks by confidence', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
      const filtered = filterPlan(plan, { confidences: ['high'] });

      expect(filtered.tasks.every((t) => t.confidence === 'high')).toBe(true);
    });

    it('should filter by multiple confidence levels', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
      const filtered = filterPlan(plan, { confidences: ['high', 'medium'] });

      expect(
        filtered.tasks.every((t) => t.confidence === 'high' || t.confidence === 'medium')
      ).toBe(true);
    });
  });

  describe('combined filters', () => {
    it('should apply multiple filters (AND logic)', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, {
        scopes: ['AUTH'],
        priorities: ['high'],
      });

      expect(filtered.modules.length).toBe(1);
      expect(filtered.modules[0].id).toBe('auth');
      expect(filtered.modules[0].metadata.priority).toBe('high');
    });
  });
});

describe('convenience filter functions', () => {
  it('filterByScope should return tasks', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
    const tasks = filterByScope(plan, ['AUTH']);

    expect(tasks.length).toBeGreaterThan(0);
    // Tasks are filtered by having AUTH in their scopes array
    expect(tasks.every((t) => t.scopes?.includes('AUTH') || t.id.startsWith('AUTH-'))).toBe(true);
  });

  it('filterByModule should return tasks', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
    const tasks = filterByModule(plan, ['cart']);

    expect(tasks.length).toBeGreaterThan(0);
    expect(tasks.every((t) => t.id.startsWith('CART-'))).toBe(true);
  });

  it('filterByTags should return tasks', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
    const tasks = filterByTags(plan, ['security']);

    expect(tasks.length).toBeGreaterThan(0);
  });

  it('filterByOwner should return tasks', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
    const tasks = filterByOwner(plan, ['@alice']);

    expect(tasks.length).toBeGreaterThan(0);
  });

  it('filterByPriority should return tasks', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
    const tasks = filterByPriority(plan, ['high']);

    expect(tasks.length).toBeGreaterThan(0);
  });

  it('filterByConfidence should return tasks', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
    const tasks = filterByConfidence(plan, ['high']);

    expect(tasks.length).toBeGreaterThan(0);
    expect(tasks.every((t) => t.confidence === 'high')).toBe(true);
  });

  it('getTasksById should return specific tasks', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
    const tasks = getTasksById(plan, ['AUTH-001', 'AUTH-003']);

    expect(tasks.length).toBe(2);
    expect(tasks.map((t) => t.id).sort()).toEqual(['AUTH-001', 'AUTH-003']);
  });
});

describe('context bundle builders', () => {
  describe('buildContextBundleJSON', () => {
    it('should build JSON bundle with summary', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { scopes: ['AUTH'] });
      const bundle = buildContextBundleJSON(filtered);

      expect(bundle.title).toBe('E-commerce Platform MVP');
      expect(bundle.summary.totalModules).toBe(1);
      expect(bundle.summary.totalTasks).toBeGreaterThan(0);
      expect(bundle.modules.length).toBe(1);
      expect(bundle.modules[0].id).toBe('auth');
    });

    it('should include filter criteria in bundle', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, {
        scopes: ['AUTH'],
        tags: ['security'],
        statuses: ['open'],
      });
      const bundle = buildContextBundleJSON(filtered);

      expect(bundle.filter.scopes).toEqual(['AUTH']);
      expect(bundle.filter.tags).toEqual(['security']);
      expect(bundle.filter.statuses).toEqual(['open']);
    });

    it('should include task details', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
      const filtered = filterPlan(plan, {});
      const bundle = buildContextBundleJSON(filtered);

      expect(bundle.modules[0].tasks.length).toBeGreaterThan(0);
      const task = bundle.modules[0].tasks[0];
      expect(task.id).toBeDefined();
      expect(task.title).toBeDefined();
      expect(task.intent).toBeDefined();
      expect(task.confidence).toBeDefined();
    });
  });

  describe('buildContextBundleText', () => {
    it('should build Markdown text bundle', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { scopes: ['AUTH'] });
      const text = buildContextBundleText(filtered);

      expect(text).toContain('# E-commerce Platform MVP');
      expect(text).toContain('## Summary');
      expect(text).toContain('## Module: auth');
      expect(text).toContain('### Tasks');
    });

    it('should include filter info in text', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      const filtered = filterPlan(plan, { scopes: ['AUTH'], statuses: ['open'] });
      const text = buildContextBundleText(filtered);

      expect(text).toContain('Filtered by:');
      expect(text).toContain('scopes: AUTH');
      expect(text).toContain('statuses: open');
    });
  });

  describe('buildTaskContext', () => {
    it('should build focused context for a single task', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
      const filtered = filterPlan(plan, {});
      const context = buildTaskContext(filtered, 'AUTH-001');

      expect(context).not.toBeNull();
      expect(context).toContain('# Task: AUTH-001');
      expect(context).toContain('**Intent:**');
    });

    it('should return null for non-existent task', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
      const filtered = filterPlan(plan, {});
      const context = buildTaskContext(filtered, 'NONEXISTENT-999');

      expect(context).toBeNull();
    });

    it('should include dependencies with status', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));
      const filtered = filterPlan(plan, {});
      // AUTH-003 depends on AUTH-001 and AUTH-002
      const context = buildTaskContext(filtered, 'AUTH-003');

      expect(context).toContain('## Dependencies');
      expect(context).toContain('AUTH-001');
      expect(context).toContain('AUTH-002');
    });
  });
});
