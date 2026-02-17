/**
 * Architecture baseline storage
 *
 * Handles reading/writing .anvil/architecture.json
 */

import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import {
  ArchitectureBaselineSchema,
  type ArchitectureBaseline,
  type Layers,
  type EntryPoint,
  type Boundary,
  type BaselineViolation,
  createDefaultLayers,
  createDefaultBoundaries,
} from './types.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('architecture');

/**
 * Default baseline file path
 */
export const BASELINE_FILENAME = 'architecture.json';
export const ANVIL_DIR = '.anvil';

/**
 * Get the full path to the baseline file
 */
export function getBaselinePath(workspaceRoot: string): string {
  return join(workspaceRoot, ANVIL_DIR, BASELINE_FILENAME);
}

/**
 * Check if a baseline exists
 */
export function baselineExists(workspaceRoot: string): boolean {
  return existsSync(getBaselinePath(workspaceRoot));
}

/**
 * Load the architecture baseline
 */
export function loadBaseline(workspaceRoot: string): ArchitectureBaseline | null {
  const path = getBaselinePath(workspaceRoot);

  if (!existsSync(path)) {
    debug('no baseline file found', path);
    return null;
  }

  try {
    debug('loading baseline from', path);
    const content = readFileSync(path, 'utf-8');
    const data = JSON.parse(content);

    // Validate against schema
    const result = ArchitectureBaselineSchema.safeParse(data);

    if (!result.success) {
      debug('invalid baseline schema', result.error.format());
      console.error('Invalid architecture baseline:', result.error.format());
      return null;
    }

    debug('baseline loaded', {
      modules: result.data.baseline_snapshot.module_count,
      violations: result.data.baseline_snapshot.violations.length,
    });
    return result.data;
  } catch (error) {
    debug('failed to load baseline', error instanceof Error ? error : undefined);
    console.error('Failed to load architecture baseline:', error);
    return null;
  }
}

/**
 * Save the architecture baseline
 */
export function saveBaseline(workspaceRoot: string, baseline: ArchitectureBaseline): void {
  debug('saving baseline', { modules: baseline.baseline_snapshot.module_count });
  const path = getBaselinePath(workspaceRoot);
  const dir = dirname(path);

  // Ensure directory exists
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }

  // Validate before saving
  const result = ArchitectureBaselineSchema.safeParse(baseline);
  if (!result.success) {
    throw new Error(`Invalid baseline data: ${result.error.format()}`);
  }

  // Write with pretty formatting for human readability
  writeFileSync(path, JSON.stringify(baseline, null, 2) + '\n', 'utf-8');
}

/**
 * Create a new baseline with defaults
 */
export function createBaseline(options: {
  entryPoints?: EntryPoint[];
  layers?: Layers;
  boundaries?: Boundary[];
  violations?: BaselineViolation[];
  moduleCount?: number;
}): ArchitectureBaseline {
  const now = new Date().toISOString();
  const layers = options.layers ?? createDefaultLayers();
  const boundaries = options.boundaries ?? createDefaultBoundaries(layers);

  return {
    schema_version: '0.1.0',
    created_at: now,
    updated_at: now,
    entry_points: options.entryPoints ?? [],
    layers,
    boundaries,
    baseline_snapshot: {
      module_count: options.moduleCount ?? 0,
      timestamp: now,
      violations: options.violations ?? [],
    },
  };
}

/**
 * Update an existing baseline
 */
export function updateBaseline(
  existing: ArchitectureBaseline,
  updates: Partial<{
    entryPoints: EntryPoint[];
    layers: Layers;
    boundaries: Boundary[];
    violations: BaselineViolation[];
    moduleCount: number;
  }>
): ArchitectureBaseline {
  const now = new Date().toISOString();

  return {
    ...existing,
    updated_at: now,
    entry_points: updates.entryPoints ?? existing.entry_points,
    layers: updates.layers ?? existing.layers,
    boundaries: updates.boundaries ?? existing.boundaries,
    baseline_snapshot: {
      ...existing.baseline_snapshot,
      module_count: updates.moduleCount ?? existing.baseline_snapshot.module_count,
      violations: updates.violations ?? existing.baseline_snapshot.violations,
      timestamp: now,
    },
  };
}

/**
 * Merge new violations into baseline (for incremental updates)
 */
export function mergeViolations(
  existing: BaselineViolation[],
  newViolations: BaselineViolation[]
): BaselineViolation[] {
  const byId = new Map<string, BaselineViolation>();

  // Add existing
  for (const v of existing) {
    byId.set(v.id, v);
  }

  // Add/update new
  for (const v of newViolations) {
    byId.set(v.id, v);
  }

  return Array.from(byId.values());
}

/**
 * Find violations that are NEW (not in baseline)
 */
export function findNewViolations(
  current: BaselineViolation[],
  baseline: BaselineViolation[]
): BaselineViolation[] {
  const baselineIds = new Set(baseline.map((v) => v.id));
  return current.filter((v) => !baselineIds.has(v.id));
}

/**
 * Find violations that were FIXED (in baseline but not current)
 */
export function findFixedViolations(
  current: BaselineViolation[],
  baseline: BaselineViolation[]
): BaselineViolation[] {
  const currentIds = new Set(current.map((v) => v.id));
  return baseline.filter((v) => !currentIds.has(v.id));
}

/**
 * Baseline manager for convenient operations
 */
export class BaselineManager {
  private workspaceRoot: string;
  private baseline: ArchitectureBaseline | null = null;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
  }

  /**
   * Check if baseline exists
   */
  exists(): boolean {
    return baselineExists(this.workspaceRoot);
  }

  /**
   * Load baseline (cached)
   */
  load(): ArchitectureBaseline | null {
    if (!this.baseline) {
      this.baseline = loadBaseline(this.workspaceRoot);
    }
    return this.baseline;
  }

  /**
   * Force reload baseline
   */
  reload(): ArchitectureBaseline | null {
    this.baseline = loadBaseline(this.workspaceRoot);
    return this.baseline;
  }

  /**
   * Save baseline
   */
  save(baseline: ArchitectureBaseline): void {
    saveBaseline(this.workspaceRoot, baseline);
    this.baseline = baseline;
  }

  /**
   * Create and save a new baseline
   */
  create(options: Parameters<typeof createBaseline>[0]): ArchitectureBaseline {
    const baseline = createBaseline(options);
    this.save(baseline);
    return baseline;
  }

  /**
   * Update and save existing baseline
   */
  update(updates: Parameters<typeof updateBaseline>[1]): ArchitectureBaseline | null {
    const existing = this.load();
    if (!existing) {
      return null;
    }

    const updated = updateBaseline(existing, updates);
    this.save(updated);
    return updated;
  }

  /**
   * Get layers from baseline or defaults
   */
  getLayers(): Layers {
    const baseline = this.load();
    return baseline?.layers ?? createDefaultLayers();
  }

  /**
   * Get boundaries from baseline or defaults
   */
  getBoundaries(): Boundary[] {
    const baseline = this.load();
    if (baseline) {
      return baseline.boundaries;
    }
    return createDefaultBoundaries(createDefaultLayers());
  }

  /**
   * Check if a violation is new
   */
  isNewViolation(violation: BaselineViolation): boolean {
    const baseline = this.load();
    if (!baseline) {
      return true; // No baseline = all violations are new
    }

    return !baseline.baseline_snapshot.violations.some((v) => v.id === violation.id);
  }

  /**
   * Get baseline path
   */
  getPath(): string {
    return getBaselinePath(this.workspaceRoot);
  }
}

/**
 * Create a baseline manager
 */
export function createBaselineManager(workspaceRoot: string): BaselineManager {
  return new BaselineManager(workspaceRoot);
}
