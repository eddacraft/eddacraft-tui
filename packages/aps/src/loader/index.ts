/**
 * Plan loader module
 * Loads and resolves APS planning documents into a graph structure
 */

import { promises as fs, realpathSync } from 'node:fs';
import { dirname, resolve, isAbsolute, sep, posix, win32 } from 'node:path';
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

  /** Maximum nesting depth for recursive index loading (default: 3) */
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
  const maxDepth = options.maxDepth ?? 3;

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
  maxDepth: number,
  depth = 0
): Promise<LoadedPlan> {
  if (depth > maxDepth) {
    throw new ParseError(
      `Maximum nesting depth (${maxDepth}) exceeded at "${indexPath}". ` +
        'Increase maxDepth or check for circular index references.',
      indexPath
    );
  }

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

      // Check if this module is itself an index file (nested plan).
      // Guard against false positives (e.g. a leaf spec with a heading like
      // "## Modules impacted"): probe with parseIndex first — only treat as
      // nested if it actually yields modules. Errors from the full recursive
      // load are NOT caught so depth violations, duplicate IDs, and malformed
      // nested metadata propagate correctly.
      if (detectIndexFile(moduleContent)) {
        let isRealIndex = false;
        try {
          const probe = await parseIndex(moduleContent, resolvedPath);
          isRealIndex = probe.modules.length > 0;
        } catch {
          // parseIndex failed — not a valid index, fall through to leaf
        }

        if (isRealIndex) {
          const nestedBaseDir = dirname(resolvedPath);
          const nestedPlan = await loadMultiModulePlan(
            resolvedPath,
            moduleContent,
            nestedBaseDir,
            recursive,
            maxDepth,
            depth + 1
          );

          // Merge nested modules into the parent plan, propagating parent deps
          const parentDeps = moduleMeta.dependencies ?? [];
          const nestedChildIds: string[] = [];
          for (const [nestedId, nestedModule] of nestedPlan.modules) {
            if (modules.has(nestedId)) {
              throw new ParseError(
                `Duplicate module ID "${nestedId}" when merging nested index "${resolvedPath}"`,
                indexPath
              );
            }
            const mergedDeps = [...new Set([...parentDeps, ...nestedModule.dependsOn])];
            modules.set(nestedId, { ...nestedModule, dependsOn: mergedDeps });
            allTasks.push(...nestedModule.tasks);
            dependencyGraph.set(nestedId, mergedDeps);
            nestedChildIds.push(nestedId);
          }

          // Insert the parent module ID as a virtual node that depends on all
          // its nested children. This preserves sibling references like
          // "frontend -> subsystem" — the parent ID stays in the graph and
          // transitively depends on all flattened children.
          if (modules.has(moduleId)) {
            throw new ParseError(
              `Duplicate module ID "${moduleId}" when inserting virtual parent for nested index "${resolvedPath}"`,
              indexPath
            );
          }
          const virtualDeps = [...new Set([...parentDeps, ...nestedChildIds])];
          modules.set(moduleId, {
            id: moduleId,
            metadata: moduleMeta,
            tasks: [],
            resolvedPath,
            dependsOn: virtualDeps,
          });
          dependencyGraph.set(moduleId, virtualDeps);

          continue;
        }
        // Fall through to leaf parsing — detected heading was a false positive
      }

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

      if (modules.has(moduleId)) {
        throw new ParseError(
          `Duplicate module ID "${moduleId}" in index "${indexPath}"`,
          indexPath
        );
      }
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

function isAbsoluteModulePath(relativePath: string): boolean {
  return (
    isAbsolute(relativePath) ||
    posix.isAbsolute(relativePath) ||
    win32.isAbsolute(relativePath) ||
    /^[A-Za-z]:/.test(relativePath)
  );
}

function containsDotDotSegment(relativePath: string): boolean {
  return relativePath.split(/[\\/]/).some((segment) => segment === '..');
}

function isContained(candidate: string, root: string): boolean {
  const left = process.platform === 'win32' ? candidate.toLowerCase() : candidate;
  const right = process.platform === 'win32' ? root.toLowerCase() : root;
  if (left === right) {
    return true;
  }
  const prefix = right.endsWith(sep) ? right : right + sep;
  return left.startsWith(prefix);
}

function tryRealpath(target: string): string | undefined {
  try {
    return realpathSync.native(target);
  } catch (err) {
    const code = (err as NodeJS.ErrnoException).code;
    if (code === 'ENOSYS') {
      try {
        return realpathSync(target);
      } catch (fallbackErr) {
        if ((fallbackErr as NodeJS.ErrnoException).code === 'ENOENT') {
          return undefined;
        }
        throw fallbackErr;
      }
    }
    if (code === 'ENOENT') {
      return undefined;
    }
    throw err;
  }
}

function wrapResolveError(relativePath: string, err: unknown): never {
  if (err instanceof ParseError) {
    throw err;
  }
  throw new ParseError(
    `Failed to resolve module path "${relativePath}": ${
      err instanceof Error ? err.message : String(err)
    }`,
    relativePath
  );
}

function assertResolvedInsideBase(
  resolved: string,
  resolvedBase: string,
  relativePath: string
): void {
  if (!isContained(resolved, resolvedBase)) {
    throw new ParseError(`Module path escapes base directory: ${relativePath}`, relativePath);
  }

  let realBase: string | undefined;
  try {
    realBase = tryRealpath(resolvedBase);
  } catch (err) {
    wrapResolveError(relativePath, err);
  }
  if (realBase === undefined) {
    return;
  }

  try {
    const realTarget = tryRealpath(resolved);
    if (realTarget !== undefined) {
      if (!isContained(realTarget, realBase)) {
        throw new ParseError(`Module path escapes base directory: ${relativePath}`, relativePath);
      }
      return;
    }

    let parent = dirname(resolved);
    while (parent !== resolved) {
      const realParent = tryRealpath(parent);
      if (realParent !== undefined) {
        if (!isContained(realParent, realBase)) {
          throw new ParseError(`Module path escapes base directory: ${relativePath}`, relativePath);
        }
        return;
      }
      const next = dirname(parent);
      if (next === parent) {
        return;
      }
      parent = next;
    }
  } catch (err) {
    wrapResolveError(relativePath, err);
  }
}

/**
 * Resolve a relative path against a base directory.
 * Rejects absolute paths and paths that escape the base directory after
 * separator normalisation and symlink resolution.
 */
export function resolvePath(relativePath: string, baseDir: string): string {
  if (relativePath.includes('\0')) {
    throw new ParseError(`Module path contains a null byte: ${relativePath}`, relativePath);
  }

  if (isAbsoluteModulePath(relativePath)) {
    throw new ParseError(`Absolute module paths are not allowed: ${relativePath}`, relativePath);
  }

  if (containsDotDotSegment(relativePath)) {
    throw new ParseError(`Module path escapes base directory: ${relativePath}`, relativePath);
  }

  const cleanPath = relativePath.replace(/^\.[\\/]/, '');
  const resolvedBase = resolve(baseDir);
  const resolved = resolve(resolvedBase, cleanPath);

  assertResolvedInsideBase(resolved, resolvedBase, relativePath);

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
