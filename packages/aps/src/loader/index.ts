/**
 * Plan loader module
 * Loads and resolves APS planning documents into a graph structure
 */

import { promises as fs } from 'node:fs';
import { dirname, resolve, isAbsolute, sep } from 'node:path';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import { visit } from 'unist-util-visit';
import type { Root, Heading } from 'mdast';
import { parseDocument } from '../parser/parse-document.js';
import { parseIndex } from '../parser/parse-index.js';
import { ParseError, type Task, type ModuleMetadata } from '../types/index.js';

/**
 * A loaded module with its tasks and metadata
 */
export interface LoadedModule {
  /** Module identifier */
  id: string;

  /** Module metadata from index or leaf spec */
  metadata: ModuleMetadata;

  /** All tasks in this module */
  tasks: Task[];

  /** Resolved absolute path to the module file */
  resolvedPath: string;

  /** IDs of modules this module depends on */
  dependsOn: string[];
}

/**
 * A complete loaded plan with all modules resolved
 */
export interface LoadedPlan {
  /** Plan title */
  title: string;

  /** Root file path */
  rootPath: string;

  /** Whether this is a single-file plan or multi-module */
  isMultiModule: boolean;

  /** All loaded modules */
  modules: Map<string, LoadedModule>;

  /** All tasks from all modules (flattened) */
  allTasks: Task[];

  /** Dependency graph (module ID -> dependent module IDs) */
  dependencyGraph: Map<string, string[]>;
}

/**
 * Options for loading a plan
 */
export interface LoadOptions {
  /** Base directory for resolving relative paths (defaults to directory of root file) */
  baseDir?: string;

  /** Whether to recursively load linked modules (default: true) */
  recursive?: boolean;

  /** Maximum depth for recursive loading (default: 10) */
  maxDepth?: number;
}

/**
 * Load an APS plan from a file path
 *
 * @param filePath - Path to the root plan file (index or leaf spec)
 * @param options - Loading options
 * @returns Loaded plan with all modules resolved
 */
export async function loadPlan(filePath: string, options: LoadOptions = {}): Promise<LoadedPlan> {
  const absolutePath = isAbsolute(filePath) ? filePath : resolve(filePath);
  const baseDir = options.baseDir ?? dirname(absolutePath);
  const recursive = options.recursive ?? true;
  const maxDepth = options.maxDepth ?? 10;

  const content = await readFile(absolutePath);

  // Try to detect if this is an index file or leaf spec
  const isIndex = detectIndexFile(content);

  if (isIndex) {
    return loadMultiModulePlan(absolutePath, content, baseDir, recursive, maxDepth);
  } else {
    return loadSingleFilePlan(absolutePath, content);
  }
}

/**
 * Detect if content is an index file (has ## Modules section)
 * Uses AST parsing for robust detection (case-insensitive, handles variants)
 */
function detectIndexFile(content: string): boolean {
  const processor = unified().use(remarkParse);
  const ast = processor.parse(content) as Root;

  let hasModulesSection = false;

  visit(ast, 'heading', (node: Heading) => {
    if (node.depth === 2) {
      // Extract heading text and normalize
      let text = '';
      visit(node, 'text', (textNode: { value: string }) => {
        text += textNode.value;
      });

      // Check if heading starts with "modules" (case-insensitive)
      // This handles "## Modules", "## modules", "## Modules & Scopes", etc.
      const normalizedText = text.trim().toLowerCase();
      if (normalizedText === 'modules' || normalizedText.startsWith('modules')) {
        hasModulesSection = true;
      }
    }
  });

  return hasModulesSection;
}

/**
 * Load a single-file plan (leaf spec with tasks)
 */
async function loadSingleFilePlan(filePath: string, content: string): Promise<LoadedPlan> {
  const doc = await parseDocument(content, filePath);

  const moduleId = doc.metadata?.scope ?? 'main';
  const module: LoadedModule = {
    id: moduleId,
    metadata: doc.metadata ?? {},
    tasks: doc.tasks,
    resolvedPath: filePath,
    dependsOn: [],
  };

  const modules = new Map<string, LoadedModule>();
  modules.set(moduleId, module);

  return {
    title: doc.title,
    rootPath: filePath,
    isMultiModule: false,
    modules,
    allTasks: doc.tasks,
    dependencyGraph: new Map([[moduleId, []]]),
  };
}

/**
 * Load a multi-module plan from an index file
 */
async function loadMultiModulePlan(
  indexPath: string,
  content: string,
  baseDir: string,
  recursive: boolean,
  _maxDepth: number // TODO: implement depth limiting for nested index files
): Promise<LoadedPlan> {
  const index = await parseIndex(content, indexPath);

  const modules = new Map<string, LoadedModule>();
  const allTasks: Task[] = [];
  const dependencyGraph = new Map<string, string[]>();

  // Load each module
  for (const moduleMeta of index.modules) {
    const moduleId = moduleMeta.id;
    if (!moduleId) {
      throw new ParseError('Module is missing required id field', indexPath);
    }

    if (!moduleMeta.path) {
      throw new ParseError(`Module "${moduleId}" is missing required Path field`, indexPath);
    }

    const resolvedPath = resolvePath(moduleMeta.path, baseDir);

    if (recursive) {
      const moduleContent = await readFile(resolvedPath);
      const moduleDoc = await parseDocument(moduleContent, resolvedPath);

      // Merge metadata from index with any from the leaf spec
      const mergedMetadata: ModuleMetadata = {
        ...moduleDoc.metadata,
        ...moduleMeta,
        id: moduleId,
      };

      const loadedModule: LoadedModule = {
        id: moduleId,
        metadata: mergedMetadata,
        tasks: moduleDoc.tasks,
        resolvedPath,
        dependsOn: moduleMeta.dependencies ?? [],
      };

      modules.set(moduleId, loadedModule);
      allTasks.push(...moduleDoc.tasks);
      dependencyGraph.set(moduleId, moduleMeta.dependencies ?? []);
    } else {
      // Non-recursive: just record module metadata without loading content
      const loadedModule: LoadedModule = {
        id: moduleId,
        metadata: moduleMeta,
        tasks: [],
        resolvedPath,
        dependsOn: moduleMeta.dependencies ?? [],
      };

      modules.set(moduleId, loadedModule);
      dependencyGraph.set(moduleId, moduleMeta.dependencies ?? []);
    }
  }

  return {
    title: index.title,
    rootPath: indexPath,
    isMultiModule: true,
    modules,
    allTasks,
    dependencyGraph,
  };
}

/**
 * Resolve a relative path against a base directory.
 * Rejects absolute paths and paths that escape the base directory.
 */
export function resolvePath(relativePath: string, baseDir: string): string {
  // Reject absolute paths — module paths must be relative to baseDir
  if (isAbsolute(relativePath)) {
    throw new ParseError(`Absolute module paths are not allowed: ${relativePath}`, relativePath);
  }

  // Remove leading ./ if present
  const cleanPath = relativePath.replace(/^\.\//, '');
  const resolved = resolve(baseDir, cleanPath);
  const resolvedBase = resolve(baseDir);

  // Validate the resolved path stays within baseDir
  if (resolved !== resolvedBase && !resolved.startsWith(resolvedBase + sep)) {
    throw new ParseError(`Module path escapes base directory: ${relativePath}`, relativePath);
  }

  return resolved;
}

/**
 * Read a file with proper error handling
 */
async function readFile(filePath: string): Promise<string> {
  try {
    return await fs.readFile(filePath, 'utf-8');
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      throw new ParseError(`File not found: ${filePath}`, filePath);
    }
    throw new ParseError(
      `Failed to read file: ${error instanceof Error ? error.message : String(error)}`,
      filePath
    );
  }
}

/**
 * Get all tasks for a specific module
 */
export function getModuleTasks(plan: LoadedPlan, moduleId: string): Task[] {
  const module = plan.modules.get(moduleId);
  return module?.tasks ?? [];
}

/**
 * Get all modules that depend on a specific module
 */
export function getDependentModules(plan: LoadedPlan, moduleId: string): string[] {
  const dependents: string[] = [];

  for (const [id, deps] of plan.dependencyGraph) {
    if (deps.includes(moduleId)) {
      dependents.push(id);
    }
  }

  return dependents;
}

/**
 * Get modules in topological order (dependencies first)
 */
export function getModulesInOrder(plan: LoadedPlan): string[] {
  const visited = new Set<string>();
  const result: string[] = [];

  function visit(moduleId: string) {
    if (visited.has(moduleId)) return;
    visited.add(moduleId);

    const deps = plan.dependencyGraph.get(moduleId) ?? [];
    for (const dep of deps) {
      visit(dep);
    }

    result.push(moduleId);
  }

  for (const moduleId of plan.modules.keys()) {
    visit(moduleId);
  }

  return result;
}

/**
 * Check for circular dependencies in the plan
 */
export function detectCycles(plan: LoadedPlan): string[][] {
  const cycles: string[][] = [];
  const visited = new Set<string>();
  const recursionStack = new Set<string>();
  const path: string[] = [];

  function dfs(moduleId: string): boolean {
    visited.add(moduleId);
    recursionStack.add(moduleId);
    path.push(moduleId);

    const deps = plan.dependencyGraph.get(moduleId) ?? [];
    for (const dep of deps) {
      if (!visited.has(dep)) {
        if (dfs(dep)) {
          return true;
        }
      } else if (recursionStack.has(dep)) {
        // Found a cycle
        const cycleStart = path.indexOf(dep);
        cycles.push([...path.slice(cycleStart), dep]);
      }
    }

    path.pop();
    recursionStack.delete(moduleId);
    return false;
  }

  for (const moduleId of plan.modules.keys()) {
    if (!visited.has(moduleId)) {
      dfs(moduleId);
    }
  }

  return cycles;
}

// Re-export types
export type { ParsedIndex } from '../parser/parse-index.js';
export type { ParsedDocument } from '../types/index.js';
