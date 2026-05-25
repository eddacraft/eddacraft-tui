/**
 * Validator module - Validation rules for APS planning documents
 *
 * Provides validation for:
 * - Required sections (Index: ## Modules, Leaf: ## Tasks)
 * - Task format (ID, Intent required)
 * - Duplicate task IDs across plan graph
 * - Broken module links
 * - Scope mismatches (warning)
 * - Missing Confidence (warning)
 * - Missing Expected Outcome (warning)
 * - Missing Validation/Test (warning)
 * - Orphan leaf specs (warning)
 * - Circular module dependencies
 */

import { promises as fs, accessSync } from 'node:fs';
import { dirname, isAbsolute, resolve } from 'node:path';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import { visit } from 'unist-util-visit';
import type { Root, Heading, Paragraph, Link } from 'mdast';
import { TASK_ID_REGEX, ParseError } from '../types/index.js';
import { loadPlan, detectCycles, resolvePath, type LoadedPlan } from '../loader/index.js';

/**
 * Validation issue severity
 */
export type ValidationSeverity = 'error' | 'warning';

/**
 * A single validation issue
 */
export interface ValidationIssue {
  /** Severity level */
  severity: ValidationSeverity;

  /** Human-readable message */
  message: string;

  /** Rule that triggered this issue */
  rule: string;

  /** File path where the issue was found */
  path?: string;

  /** Line number in the file (1-based) */
  lineNumber?: number;

  /** Additional context */
  context?: string;
}

/**
 * Result of validating a planning document
 */
export interface ValidationResult {
  /** Whether the document is valid (no errors, warnings allowed) */
  valid: boolean;

  /** List of all issues found */
  issues: ValidationIssue[];

  /** Just the errors */
  errors: ValidationIssue[];

  /** Just the warnings */
  warnings: ValidationIssue[];
}

/**
 * Options for validation
 */
export interface ValidateOptions {
  /** Base directory for resolving relative paths */
  baseDir?: string;

  /** Whether to recursively validate linked modules (default: true) */
  recursive?: boolean;

  /** Rules to skip (by rule name) */
  skipRules?: string[];
}

/**
 * Validate an APS planning document
 *
 * @param filePath - Path to the planning document (index or leaf spec)
 * @param options - Validation options
 * @returns Validation result with errors and warnings
 *
 * @example
 * ```typescript
 * const result = await validatePlanningDoc('docs/planning/APS.md');
 * if (!result.valid) {
 *   for (const error of result.errors) {
 *     console.error(`${error.path}:${error.lineNumber}: ${error.message}`);
 *   }
 * }
 * ```
 */
export async function validatePlanningDoc(
  filePath: string,
  options: ValidateOptions = {}
): Promise<ValidationResult> {
  const absolutePath = isAbsolute(filePath) ? filePath : resolve(filePath);
  const baseDir = options.baseDir ?? dirname(absolutePath);
  const recursive = options.recursive ?? true;
  const skipRules = new Set(options.skipRules ?? []);

  const issues: ValidationIssue[] = [];

  // Read the file content
  let content: string;
  try {
    content = await fs.readFile(absolutePath, 'utf-8');
  } catch (error) {
    issues.push({
      severity: 'error',
      message: `Failed to read file: ${error instanceof Error ? error.message : String(error)}`,
      rule: 'file-readable',
      path: absolutePath,
    });
    return createResult(issues);
  }

  // Determine document type and validate structure
  const isIndex = detectIndexFile(content);

  if (isIndex) {
    // Validate index file structure
    if (!skipRules.has('required-sections')) {
      await validateIndexStructure(content, absolutePath, issues);
    }

    // Validate module links
    if (!skipRules.has('broken-links') && recursive) {
      await validateModuleLinks(content, absolutePath, baseDir, issues, skipRules);
    }
  } else {
    // Validate leaf spec structure
    if (!skipRules.has('required-sections')) {
      validateLeafStructure(content, absolutePath, issues);
    }

    // Validate tasks
    if (!skipRules.has('task-format')) {
      validateTaskFormat(content, absolutePath, issues, skipRules);
    }
  }

  // Load the full plan for cross-document validation
  if (recursive) {
    try {
      const plan = await loadPlan(absolutePath, { baseDir, recursive: true });

      // Check for duplicate task IDs
      if (!skipRules.has('duplicate-ids')) {
        validateDuplicateTaskIds(plan, issues);
      }

      // Check for circular dependencies
      if (!skipRules.has('circular-dependencies')) {
        validateCircularDependencies(plan, issues);
      }

      // Check scope mismatches
      if (!skipRules.has('scope-mismatch')) {
        validateScopeMismatches(plan, issues);
      }

      // Check for orphan modules (only if index file)
      if (isIndex && !skipRules.has('orphan-modules')) {
        await validateOrphanModules(absolutePath, baseDir, plan, issues);
      }
    } catch (error) {
      // If we can't load the plan, the earlier validation errors should explain why
      if (issues.length === 0) {
        issues.push({
          severity: 'error',
          message: `Failed to load plan: ${error instanceof Error ? error.message : String(error)}`,
          rule: 'plan-loadable',
          path: absolutePath,
        });
      }
    }
  }

  return createResult(issues);
}

/**
 * Detect if content is an index file (has ## Modules section)
 */
function detectIndexFile(content: string): boolean {
  const processor = unified().use(remarkParse);
  const ast = processor.parse(content) as Root;

  let hasModulesSection = false;

  visit(ast, 'heading', (node: Heading) => {
    if (node.depth === 2) {
      let text = '';
      visit(node, 'text', (textNode: { value: string }) => {
        text += textNode.value;
      });

      const normalizedText = text.trim().toLowerCase();
      if (normalizedText === 'modules' || normalizedText.startsWith('modules')) {
        hasModulesSection = true;
      }
    }
  });

  return hasModulesSection;
}

/**
 * Validate index file structure
 */
async function validateIndexStructure(
  content: string,
  filePath: string,
  issues: ValidationIssue[]
): Promise<void> {
  const processor = unified().use(remarkParse);
  const ast = processor.parse(content) as Root;

  let hasH1 = false;
  let hasModulesSection = false;
  let modulesLineNumber = 0;
  let hasModuleEntries = false;

  visit(ast, 'heading', (node: Heading) => {
    if (node.depth === 1) {
      hasH1 = true;
    }
    if (node.depth === 2) {
      let text = '';
      visit(node, 'text', (textNode: { value: string }) => {
        text += textNode.value;
      });

      const normalizedText = text.trim().toLowerCase();
      if (normalizedText === 'modules' || normalizedText.startsWith('modules')) {
        hasModulesSection = true;
        modulesLineNumber = node.position?.start.line ?? 0;
      }
    }
    if (node.depth === 3 && hasModulesSection) {
      hasModuleEntries = true;
    }
  });

  if (!hasH1) {
    issues.push({
      severity: 'error',
      message: 'Index file must have an H1 title',
      rule: 'required-sections',
      path: filePath,
      lineNumber: 1,
    });
  }

  if (!hasModulesSection) {
    issues.push({
      severity: 'error',
      message: 'Index file must have a "## Modules" section',
      rule: 'required-sections',
      path: filePath,
    });
  } else if (!hasModuleEntries) {
    issues.push({
      severity: 'warning',
      message: '"## Modules" section has no module entries (H3 headings)',
      rule: 'required-sections',
      path: filePath,
      lineNumber: modulesLineNumber,
    });
  }
}

/**
 * Validate leaf spec structure
 */
function validateLeafStructure(content: string, filePath: string, issues: ValidationIssue[]): void {
  const processor = unified().use(remarkParse);
  const ast = processor.parse(content) as Root;

  let hasH1 = false;
  let hasTasksSection = false;
  let inTasksSection = false;
  let tasksLineNumber = 0;
  let hasTaskEntries = false;

  visit(ast, 'heading', (node: Heading) => {
    if (node.depth === 1) {
      hasH1 = true;
    }
    if (node.depth === 2) {
      let text = '';
      visit(node, 'text', (textNode: { value: string }) => {
        text += textNode.value;
      });

      inTasksSection = isTaskSectionTitle(text);
      if (inTasksSection) {
        hasTasksSection = true;
        tasksLineNumber = node.position?.start.line ?? 0;
      }
    }
    if (node.depth === 3 && inTasksSection) {
      hasTaskEntries = true;
    }
  });

  if (!hasH1) {
    issues.push({
      severity: 'error',
      message: 'Leaf spec must have an H1 title',
      rule: 'required-sections',
      path: filePath,
      lineNumber: 1,
    });
  }

  if (!hasTasksSection) {
    issues.push({
      severity: 'error',
      message: 'Leaf spec must have a "## Tasks" or "## Work Items" section',
      rule: 'required-sections',
      path: filePath,
    });
  } else if (!hasTaskEntries) {
    issues.push({
      severity: 'warning',
      message: 'Task section has no task entries (H3 headings)',
      rule: 'required-sections',
      path: filePath,
      lineNumber: tasksLineNumber,
    });
  }
}

/**
 * Validate module links in an index file
 */
async function validateModuleLinks(
  content: string,
  filePath: string,
  baseDir: string,
  issues: ValidationIssue[],
  skipRules: Set<string> = new Set()
): Promise<void> {
  const processor = unified().use(remarkParse);
  const ast = processor.parse(content) as Root;

  let inModulesSection = false;
  let currentModuleId: string | null = null;

  visit(ast, (node) => {
    if (node.type === 'heading') {
      const heading = node as Heading;
      if (heading.depth === 2) {
        let text = '';
        visit(heading, 'text', (textNode: { value: string }) => {
          text += textNode.value;
        });
        const normalizedText = text.trim().toLowerCase();
        inModulesSection = normalizedText === 'modules' || normalizedText.startsWith('modules');
      }
      if (heading.depth === 3 && inModulesSection) {
        let text = '';
        visit(heading, 'text', (textNode: { value: string }) => {
          text += textNode.value;
        });
        currentModuleId = text.trim();
      }
    }

    // Check for Path links in list items
    if (node.type === 'paragraph' && inModulesSection && currentModuleId) {
      const para = node as Paragraph;
      let hasPathField = false;
      let linkUrl: string | null = null;
      const linkLine = para.position?.start.line ?? 0;

      for (const child of para.children) {
        if (child.type === 'strong') {
          let strongText = '';
          visit(child, 'text', (textNode: { value: string }) => {
            strongText += textNode.value;
          });
          if (strongText === 'Path:') {
            hasPathField = true;
          }
        }
        if (child.type === 'link' && hasPathField) {
          linkUrl = (child as Link).url;
        }
      }

      if (hasPathField && linkUrl) {
        // Validate the link is within the project and exists
        try {
          const resolvedPath = resolvePath(linkUrl, baseDir);
          validateFileExists(resolvedPath, filePath, linkLine, currentModuleId, issues);
        } catch (err) {
          if (err instanceof ParseError) {
            // resolvePath rejects absolute paths and paths escaping baseDir —
            // report as a validation issue rather than crashing the run
            if (!skipRules.has('path-containment')) {
              issues.push({
                severity: 'error',
                message: `Unsafe link path in module "${currentModuleId}": "${linkUrl}" escapes project directory`,
                rule: 'path-containment',
                path: filePath,
                lineNumber: linkLine,
                context: `Module: ${currentModuleId}`,
              });
            }
          } else {
            throw err;
          }
        }
      }
    }
  });
}

/**
 * Validate that a file exists (async check queued for later)
 */
function validateFileExists(
  targetPath: string,
  sourcePath: string,
  lineNumber: number,
  moduleId: string,
  issues: ValidationIssue[]
): void {
  // Use sync check for simplicity (file system is fast for existence checks)
  try {
    accessSync(targetPath);
  } catch {
    issues.push({
      severity: 'error',
      message: `Broken link: module "${moduleId}" links to non-existent file "${targetPath}"`,
      rule: 'broken-links',
      path: sourcePath,
      lineNumber,
      context: `Module: ${moduleId}`,
    });
  }
}

/**
 * Validate task format in a leaf spec
 */
function validateTaskFormat(
  content: string,
  filePath: string,
  issues: ValidationIssue[],
  skipRules: Set<string>
): void {
  const processor = unified().use(remarkParse);
  const ast = processor.parse(content) as Root;

  let inTasksSection = false;
  let currentTaskHeading: { id: string; title: string; line: number } | null = null;
  let currentTaskContent: string[] = [];

  visit(ast, (node) => {
    if (node.type === 'heading') {
      const heading = node as Heading;

      if (heading.depth === 2) {
        // Check for section change
        if (currentTaskHeading) {
          validateTaskContent(currentTaskHeading, currentTaskContent, filePath, issues, skipRules);
          currentTaskHeading = null;
          currentTaskContent = [];
        }

        let text = '';
        visit(heading, 'text', (textNode: { value: string }) => {
          text += textNode.value;
        });
        inTasksSection = isTaskSectionTitle(text);
      }

      if (heading.depth === 3 && inTasksSection) {
        // Save previous task
        if (currentTaskHeading) {
          validateTaskContent(currentTaskHeading, currentTaskContent, filePath, issues, skipRules);
        }

        // Parse task heading
        let text = '';
        visit(heading, 'text', (textNode: { value: string }) => {
          text += textNode.value;
        });

        const lineNumber = heading.position?.start.line ?? 0;
        const match = text.match(/^([A-Z0-9]+-\d+):\s*(.*)$/);

        if (!match) {
          issues.push({
            severity: 'error',
            message: `Invalid task heading format: "${text}". Expected "ID: Title" (e.g., "AUTH-001: Implement login")`,
            rule: 'task-format',
            path: filePath,
            lineNumber,
          });
          currentTaskHeading = null;
        } else {
          const [, id, title] = match;

          // Validate task ID format
          if (!TASK_ID_REGEX.test(id)) {
            issues.push({
              severity: 'error',
              message: `Invalid task ID format: "${id}". Expected 1-10 alphanumeric scope + hyphen + 3-digit number (e.g., AUTH-001)`,
              rule: 'task-format',
              path: filePath,
              lineNumber,
            });
          }

          currentTaskHeading = { id, title, line: lineNumber };
          currentTaskContent = [];
        }
      }
    }

    // Collect task content
    if (currentTaskHeading && node.type === 'paragraph') {
      let text = '';
      visit(node, 'text', (textNode: { value: string }) => {
        text += textNode.value;
      });
      visit(node, 'strong', (strongNode) => {
        visit(strongNode, 'text', (textNode: { value: string }) => {
          text += textNode.value;
        });
      });
      currentTaskContent.push(text);
    }
  });

  // Validate last task
  if (currentTaskHeading) {
    validateTaskContent(currentTaskHeading, currentTaskContent, filePath, issues, skipRules);
  }
}

/**
 * Validate task content (Intent required, Confidence/Validation/ExpectedOutcome warnings)
 */
function validateTaskContent(
  task: { id: string; title: string; line: number },
  content: string[],
  filePath: string,
  issues: ValidationIssue[],
  skipRules: Set<string>
): void {
  const fullContent = content.join(' ');

  // Check for Intent (required)
  if (!skipRules.has('task-intent') && !fullContent.includes('Intent:')) {
    issues.push({
      severity: 'error',
      message: `Task "${task.id}" is missing required **Intent:** field`,
      rule: 'task-intent',
      path: filePath,
      lineNumber: task.line,
    });
  }

  // Check for Expected Outcome (warning per APS spec)
  if (
    !skipRules.has('missing-expected-outcome') &&
    !fullContent.includes('Expected Outcome:') &&
    !fullContent.includes('ExpectedOutcome:') &&
    !fullContent.includes('Outcome:')
  ) {
    issues.push({
      severity: 'warning',
      message: `Task "${task.id}" is missing **Expected Outcome:** field (recommended for testability)`,
      rule: 'missing-expected-outcome',
      path: filePath,
      lineNumber: task.line,
    });
  }

  // Check for Validation/Test (warning per APS spec)
  if (
    !skipRules.has('missing-validation') &&
    !fullContent.includes('Validation:') &&
    !fullContent.includes('Test:')
  ) {
    issues.push({
      severity: 'warning',
      message: `Task "${task.id}" is missing **Validation:** or **Test:** field (recommended for verification)`,
      rule: 'missing-validation',
      path: filePath,
      lineNumber: task.line,
    });
  }

  // Check for Confidence (warning)
  if (!skipRules.has('missing-confidence') && !fullContent.includes('Confidence:')) {
    issues.push({
      severity: 'warning',
      message: `Task "${task.id}" is missing **Confidence:** field (defaults to "medium")`,
      rule: 'missing-confidence',
      path: filePath,
      lineNumber: task.line,
    });
  }
}

function isTaskSectionTitle(text: string): boolean {
  const normalized = text.trim().toLowerCase();
  return normalized === 'tasks' || normalized === 'work items';
}

/**
 * Validate duplicate task IDs across the plan
 */
function validateDuplicateTaskIds(plan: LoadedPlan, issues: ValidationIssue[]): void {
  const taskLocations = new Map<string, Array<{ path: string; line?: number }>>();

  for (const task of plan.allTasks) {
    const locations = taskLocations.get(task.id) ?? [];
    locations.push({
      path: task.sourcePath ?? 'unknown',
      line: task.sourceLineNumber,
    });
    taskLocations.set(task.id, locations);
  }

  for (const [taskId, locations] of taskLocations) {
    if (locations.length > 1) {
      const locationStrings = locations
        .map((loc) => `${loc.path}${loc.line ? `:${loc.line}` : ''}`)
        .join(', ');

      issues.push({
        severity: 'error',
        message: `Duplicate task ID "${taskId}" found in: ${locationStrings}`,
        rule: 'duplicate-ids',
        path: locations[0].path,
        lineNumber: locations[0].line,
        context: `Also found at: ${locations
          .slice(1)
          .map((l) => `${l.path}:${l.line}`)
          .join(', ')}`,
      });
    }
  }
}

/**
 * Validate circular module dependencies
 */
function validateCircularDependencies(plan: LoadedPlan, issues: ValidationIssue[]): void {
  const cycles = detectCycles(plan);

  for (const cycle of cycles) {
    const cycleStr = cycle.join(' -> ');
    issues.push({
      severity: 'error',
      message: `Circular dependency detected: ${cycleStr}`,
      rule: 'circular-dependencies',
      path: plan.rootPath,
      context: `Cycle: ${cycleStr}`,
    });
  }
}

/**
 * Validate scope mismatches (task ID prefix vs module scope)
 */
function validateScopeMismatches(plan: LoadedPlan, issues: ValidationIssue[]): void {
  for (const module of plan.modules.values()) {
    const moduleScope = module.metadata.scope?.toUpperCase();

    for (const task of module.tasks) {
      const taskScope = task.id.split('-')[0];

      if (moduleScope && taskScope !== moduleScope) {
        issues.push({
          severity: 'warning',
          message: `Task "${task.id}" has scope prefix "${taskScope}" but belongs to module with scope "${moduleScope}"`,
          rule: 'scope-mismatch',
          path: task.sourcePath,
          lineNumber: task.sourceLineNumber,
          context: `Module scope: ${moduleScope}`,
        });
      }
    }
  }
}

/**
 * Validate for orphan modules (leaf specs in directory not linked from index)
 */
async function validateOrphanModules(
  indexPath: string,
  baseDir: string,
  plan: LoadedPlan,
  issues: ValidationIssue[]
): Promise<void> {
  // Get all linked module paths
  const linkedPaths = new Set<string>();
  for (const module of plan.modules.values()) {
    linkedPaths.add(module.resolvedPath);
  }

  // Scan the directory for .aps.md files
  try {
    const { readdir, stat } = await import('node:fs/promises');
    const { join } = await import('node:path');

    const MAX_SCAN_DEPTH = 10;

    async function scanDir(dir: string, depth = 0): Promise<string[]> {
      if (depth >= MAX_SCAN_DEPTH) {
        issues.push({
          severity: 'warning',
          rule: 'orphan-scan-depth',
          message: `Orphan module scan: depth limit (${MAX_SCAN_DEPTH}) reached at ${dir}, subtree skipped`,
          path: dir,
        });
        return [];
      }
      const files: string[] = [];
      try {
        const entries = await readdir(dir);
        for (const entry of entries) {
          const fullPath = join(dir, entry);
          const stats = await stat(fullPath);
          if (stats.isDirectory()) {
            files.push(...(await scanDir(fullPath, depth + 1)));
          } else if (entry.endsWith('.aps.md') && fullPath !== indexPath) {
            files.push(fullPath);
          }
        }
      } catch {
        // Ignore errors (permission denied, etc.)
      }
      return files;
    }

    const allApsFiles = await scanDir(baseDir);

    for (const file of allApsFiles) {
      if (!linkedPaths.has(file)) {
        issues.push({
          severity: 'warning',
          message: `Orphan leaf spec found: "${file}" is not linked from the index file`,
          rule: 'orphan-modules',
          path: file,
        });
      }
    }
  } catch {
    // Ignore directory scanning errors
  }
}

/**
 * Create a ValidationResult from issues
 */
function createResult(issues: ValidationIssue[]): ValidationResult {
  const errors = issues.filter((i) => i.severity === 'error');
  const warnings = issues.filter((i) => i.severity === 'warning');

  return {
    valid: errors.length === 0,
    issues,
    errors,
    warnings,
  };
}

/**
 * Format validation issues for display
 */
export function formatValidationIssues(result: ValidationResult): string {
  if (result.issues.length === 0) {
    return 'No issues found.';
  }

  const lines: string[] = [];

  for (const issue of result.issues) {
    const severity = issue.severity === 'error' ? 'ERROR' : 'WARN';
    const location = issue.path
      ? issue.lineNumber
        ? `${issue.path}:${issue.lineNumber}`
        : issue.path
      : '';
    const prefix = location ? `${location}: ` : '';
    lines.push(`[${severity}] ${prefix}${issue.message}`);
    if (issue.context) {
      lines.push(`         ${issue.context}`);
    }
  }

  const summary = `\n${result.errors.length} error(s), ${result.warnings.length} warning(s)`;
  lines.push(summary);

  return lines.join('\n');
}
