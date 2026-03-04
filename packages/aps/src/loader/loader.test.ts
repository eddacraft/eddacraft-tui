/**
 * Tests for plan loader module
 */

import { describe, it, expect } from 'vitest';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  loadPlan,
  resolvePath,
  getModuleTasks,
  getDependentModules,
  getModulesInOrder,
  detectCycles,
} from './index.js';
import { ParseError } from '../types/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const EXAMPLES_DIR = join(__dirname, '../../examples');

describe('loadPlan', () => {
  describe('single-file plans', () => {
    it('should load a single-file plan (leaf spec)', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));

      expect(plan.title).toBe('Feature: User Authentication');
      expect(plan.isMultiModule).toBe(false);
      expect(plan.modules.size).toBe(1);
      expect(plan.allTasks.length).toBe(8);
    });

    it('should create a single module for leaf specs', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'feature-auth.aps.md'));

      const module = plan.modules.get('AUTH');
      expect(module).toBeDefined();
      expect(module!.id).toBe('AUTH');
      expect(module!.tasks.length).toBe(8);
      expect(module!.dependsOn).toEqual([]);
    });
  });

  describe('multi-module plans', () => {
    it('should load a multi-module plan (index file)', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

      expect(plan.title).toBe('E-commerce Platform MVP');
      expect(plan.isMultiModule).toBe(true);
      expect(plan.modules.size).toBe(4);
    });

    it('should load all modules recursively', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

      // Check all modules are loaded
      expect(plan.modules.has('auth')).toBe(true);
      expect(plan.modules.has('products')).toBe(true);
      expect(plan.modules.has('cart')).toBe(true);
      expect(plan.modules.has('payments')).toBe(true);

      // Check tasks are loaded
      expect(plan.allTasks.length).toBeGreaterThan(0);

      // Check auth module has tasks
      const authModule = plan.modules.get('auth');
      expect(authModule!.tasks.length).toBeGreaterThan(0);
    });

    it('should build dependency graph correctly', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

      expect(plan.dependencyGraph.get('auth')).toEqual([]);
      expect(plan.dependencyGraph.get('products')).toEqual(['auth']);
      expect(plan.dependencyGraph.get('cart')).toEqual(['auth', 'products']);
      expect(plan.dependencyGraph.get('payments')).toEqual(['auth', 'cart']);
    });

    it('should skip loading module content when recursive=false', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'), {
        recursive: false,
      });

      expect(plan.modules.size).toBe(4);
      expect(plan.allTasks.length).toBe(0);

      const authModule = plan.modules.get('auth');
      expect(authModule!.tasks.length).toBe(0);
    });
  });

  describe('nested index files', () => {
    it('should recursively load nested index files', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'nested-index/APS.md'));

      expect(plan.isMultiModule).toBe(true);
      // core (leaf) + subsystem (virtual parent) + api and workers (from nested subsystem index)
      expect(plan.modules.has('core')).toBe(true);
      expect(plan.modules.has('subsystem')).toBe(true);
      expect(plan.modules.has('api')).toBe(true);
      expect(plan.modules.has('workers')).toBe(true);

      // Virtual parent node has no tasks, depends on its children
      const subsystem = plan.modules.get('subsystem')!;
      expect(subsystem.tasks).toEqual([]);
      expect(subsystem.dependsOn).toEqual(expect.arrayContaining(['api', 'workers']));

      // Dependency graph: parent deps propagated to nested modules
      expect(plan.dependencyGraph.get('core')).toEqual([]);
      expect(plan.dependencyGraph.get('api')).toEqual(expect.arrayContaining(['core']));
      expect(plan.dependencyGraph.get('workers')).toEqual(expect.arrayContaining(['core', 'api']));
      // Sibling modules depending on 'subsystem' resolve correctly
      expect(plan.dependencyGraph.get('subsystem')).toEqual(
        expect.arrayContaining(['core', 'api', 'workers'])
      );
    });

    it('should load tasks from nested modules', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'nested-index/APS.md'));

      const coreTasks = getModuleTasks(plan, 'core');
      expect(coreTasks.length).toBe(1);
      expect(coreTasks[0].id).toBe('CORE-001');

      const apiTasks = getModuleTasks(plan, 'api');
      expect(apiTasks.length).toBe(1);
      expect(apiTasks[0].id).toBe('API-001');

      const workerTasks = getModuleTasks(plan, 'workers');
      expect(workerTasks.length).toBe(1);
      expect(workerTasks[0].id).toBe('WORK-001');
    });

    it('should throw when maxDepth is exceeded', async () => {
      await expect(
        loadPlan(join(EXAMPLES_DIR, 'nested-index/APS.md'), { maxDepth: 0 })
      ).rejects.toThrow(ParseError);
      await expect(
        loadPlan(join(EXAMPLES_DIR, 'nested-index/APS.md'), { maxDepth: 0 })
      ).rejects.toThrow(/Maximum nesting depth/);
    });

    it('should succeed when maxDepth is sufficient', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'nested-index/APS.md'), { maxDepth: 1 });

      expect(plan.modules.has('api')).toBe(true);
      expect(plan.modules.has('workers')).toBe(true);
    });
  });

  describe('error handling', () => {
    it('should throw ParseError for non-existent file', async () => {
      await expect(loadPlan('/non/existent/path.aps.md')).rejects.toThrow(ParseError);
      await expect(loadPlan('/non/existent/path.aps.md')).rejects.toThrow(/File not found/);
    });
  });

  describe('index file detection', () => {
    it('should detect index with lowercase "modules" heading', async () => {
      // Create a temporary test - we'll use inline content via the loader's behavior
      // This tests that case-insensitive detection works
      const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));
      expect(plan.isMultiModule).toBe(true);
    });

    it('should not misclassify leaf spec with "Modules impacted" heading as nested index', async () => {
      const plan = await loadPlan(join(EXAMPLES_DIR, 'false-positive-modules/APS.md'));

      expect(plan.isMultiModule).toBe(true);
      expect(plan.modules.has('leaf')).toBe(true);

      const leafModule = plan.modules.get('leaf')!;
      expect(leafModule.tasks.length).toBe(1);
      expect(leafModule.tasks[0].id).toBe('LEAF-001');
    });
  });
});

describe('resolvePath', () => {
  // On Windows, path.resolve('/project/plan', ...) prepends the drive letter (e.g. 'D:')
  const toFwd = (p: string): string => p.replace(/\\/g, '/').replace(/^[A-Z]:/, '');

  it('should resolve relative paths', () => {
    expect(toFwd(resolvePath('./modules/auth.aps.md', '/project/plan'))).toBe(
      '/project/plan/modules/auth.aps.md'
    );
  });

  it('should handle paths without ./', () => {
    expect(toFwd(resolvePath('modules/auth.aps.md', '/project/plan'))).toBe(
      '/project/plan/modules/auth.aps.md'
    );
  });

  it('should reject absolute paths', () => {
    expect(() => resolvePath('/absolute/path.md', '/project')).toThrow(
      'Absolute module paths are not allowed'
    );
  });
});

describe('getModuleTasks', () => {
  it('should return tasks for a specific module', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

    const authTasks = getModuleTasks(plan, 'auth');
    expect(authTasks.length).toBeGreaterThan(0);
    expect(authTasks.every((t) => t.id.startsWith('AUTH-'))).toBe(true);
  });

  it('should return empty array for unknown module', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

    const tasks = getModuleTasks(plan, 'nonexistent');
    expect(tasks).toEqual([]);
  });
});

describe('getDependentModules', () => {
  it('should find modules that depend on a given module', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

    const authDependents = getDependentModules(plan, 'auth');
    expect(authDependents).toContain('products');
    expect(authDependents).toContain('cart');
    expect(authDependents).toContain('payments');
  });

  it('should return empty array for module with no dependents', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

    const paymentsDependents = getDependentModules(plan, 'payments');
    expect(paymentsDependents).toEqual([]);
  });
});

describe('getModulesInOrder', () => {
  it('should return modules in topological order', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

    const order = getModulesInOrder(plan);

    // Auth should come before everything
    const authIndex = order.indexOf('auth');
    const productsIndex = order.indexOf('products');
    const cartIndex = order.indexOf('cart');
    const paymentsIndex = order.indexOf('payments');

    expect(authIndex).toBeLessThan(productsIndex);
    expect(authIndex).toBeLessThan(cartIndex);
    expect(authIndex).toBeLessThan(paymentsIndex);

    // Products should come before cart
    expect(productsIndex).toBeLessThan(cartIndex);

    // Cart should come before payments
    expect(cartIndex).toBeLessThan(paymentsIndex);
  });
});

describe('detectCycles', () => {
  it('should return empty array for acyclic graph', async () => {
    const plan = await loadPlan(join(EXAMPLES_DIR, 'system-ecommerce/APS.md'));

    const cycles = detectCycles(plan);
    expect(cycles).toEqual([]);
  });

  it('should detect cycles in dependency graph', async () => {
    // Create a plan with cycles manually
    const plan = {
      title: 'Cyclic Plan',
      rootPath: '/test',
      isMultiModule: true,
      modules: new Map([
        ['a', { id: 'a', metadata: {}, tasks: [], resolvedPath: '/a', dependsOn: ['b'] }],
        ['b', { id: 'b', metadata: {}, tasks: [], resolvedPath: '/b', dependsOn: ['c'] }],
        ['c', { id: 'c', metadata: {}, tasks: [], resolvedPath: '/c', dependsOn: ['a'] }],
      ]),
      allTasks: [],
      dependencyGraph: new Map([
        ['a', ['b']],
        ['b', ['c']],
        ['c', ['a']],
      ]),
    };

    const cycles = detectCycles(plan);
    expect(cycles.length).toBeGreaterThan(0);
    // The cycle should include a, b, c
    expect(cycles[0]).toContain('a');
    expect(cycles[0]).toContain('b');
    expect(cycles[0]).toContain('c');
  });
});
