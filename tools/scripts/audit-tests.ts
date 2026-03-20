#!/usr/bin/env npx tsx
/**
 * Test Quality Audit Script
 *
 * AST-based analysis of test files to detect violations of testing best practices
 * as documented in docs/guides/testing.md.
 *
 * Usage:
 *   npx tsx scripts/audit-tests.ts          # Console output
 *   npx tsx scripts/audit-tests.ts --json   # JSON output
 *
 * Detects:
 *   - `as any` type assertions (should use vi.mocked())
 *   - Missing afterEach cleanup when mocks are used
 *   - process.chdir() without restoration
 *   - Temp directory creation without cleanup
 *   - vi.mock() without corresponding reset
 */

import * as ts from 'typescript';
import * as fs from 'node:fs';
import { glob } from 'glob';

interface Violation {
  file: string;
  line: number;
  column: number;
  rule: string;
  message: string;
  suggestion?: string;
}

interface AuditResult {
  totalFiles: number;
  filesWithViolations: number;
  violations: Violation[];
  summary: Record<string, number>;
}

/**
 * Find all test files in the project
 */
async function findTestFiles(): Promise<string[]> {
  const patterns = ['core/src/**/*.test.ts', 'cli/src/**/*.test.ts', 'packages/*/src/**/*.test.ts'];

  const files: string[] = [];
  for (const pattern of patterns) {
    const matches = await glob(pattern, {
      cwd: process.cwd(),
      ignore: ['**/node_modules/**', '**/dist/**'],
    });
    files.push(...matches);
  }

  return files;
}

/**
 * Analyse a single test file for violations
 */
function analyseFile(filePath: string): Violation[] {
  const violations: Violation[] = [];
  const content = fs.readFileSync(filePath, 'utf-8');
  const sourceFile = ts.createSourceFile(filePath, content, ts.ScriptTarget.Latest, true);

  // Track state for context-aware analysis
  let hasMockCalls = false;
  let hasSpyOnCalls = false;
  let hasAfterEach = false;
  let hasRestoreAllMocks = false;
  let hasChdirCall = false;
  let hasCwdRestoration = false;
  let hasTempDirCreation = false;
  let hasTempDirCleanup = false;

  function getLineAndColumn(node: ts.Node): { line: number; column: number } {
    const { line, character } = sourceFile.getLineAndCharacterOfPosition(node.getStart());
    return { line: line + 1, column: character + 1 };
  }

  function visit(node: ts.Node): void {
    // Detect `as any` type assertions
    if (ts.isAsExpression(node)) {
      const typeNode = node.type;
      if (ts.isTypeReferenceNode(typeNode) || typeNode.kind === ts.SyntaxKind.AnyKeyword) {
        if (typeNode.kind === ts.SyntaxKind.AnyKeyword) {
          const { line, column } = getLineAndColumn(node);
          violations.push({
            file: filePath,
            line,
            column,
            rule: 'no-any-in-tests',
            message: 'Avoid using `as any` in tests',
            suggestion: 'Use vi.mocked() for typed mock access',
          });
        }
      }
    }

    // Detect vi.mock() calls
    if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)) {
      const expr = node.expression;
      if (ts.isIdentifier(expr.expression) && expr.expression.text === 'vi') {
        const methodName = expr.name.text;

        if (methodName === 'mock') {
          hasMockCalls = true;
        }
        if (methodName === 'spyOn') {
          hasSpyOnCalls = true;
        }
        if (methodName === 'restoreAllMocks') {
          hasRestoreAllMocks = true;
        }
        // Note: resetAllMocks tracking could be added here if needed
      }
    }

    // Detect afterEach calls
    if (
      ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      node.expression.text === 'afterEach'
    ) {
      hasAfterEach = true;
    }

    // Detect process.chdir() calls
    if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)) {
      const expr = node.expression;
      if (
        ts.isIdentifier(expr.expression) &&
        expr.expression.text === 'process' &&
        expr.name.text === 'chdir'
      ) {
        hasChdirCall = true;

        // Check if it's restoring (argument is a variable like originalCwd)
        if (node.arguments.length > 0) {
          const arg = node.arguments[0];
          if (ts.isIdentifier(arg)) {
            const name = arg.text.toLowerCase();
            if (name.includes('original') || name.includes('prev') || name.includes('saved')) {
              hasCwdRestoration = true;
            }
          }
        }
      }
    }

    // Detect temp directory patterns (mkdirSync/mkdir with tmpdir)
    if (ts.isCallExpression(node)) {
      const callText = node.getText(sourceFile);
      if (callText.includes('mkdirSync') || callText.includes('mkdir')) {
        if (callText.includes('tmpdir') || callText.includes('tmp')) {
          hasTempDirCreation = true;
        }
      }
      if (
        callText.includes('rmSync') ||
        callText.includes('rm(') ||
        callText.includes('cleanup()')
      ) {
        hasTempDirCleanup = true;
      }
    }

    ts.forEachChild(node, visit);
  }

  visit(sourceFile);

  // Check for missing afterEach cleanup when mocks are used
  if ((hasMockCalls || hasSpyOnCalls) && !hasAfterEach) {
    violations.push({
      file: filePath,
      line: 1,
      column: 1,
      rule: 'require-mock-cleanup',
      message: 'Test file uses mocks but has no afterEach hook',
      suggestion: 'Add afterEach(() => { vi.restoreAllMocks(); }) to clean up mocks',
    });
  }

  // Check for missing restoreAllMocks when spies are used
  if (hasSpyOnCalls && hasAfterEach && !hasRestoreAllMocks) {
    violations.push({
      file: filePath,
      line: 1,
      column: 1,
      rule: 'require-mock-cleanup',
      message: 'Test file uses vi.spyOn() but afterEach does not call vi.restoreAllMocks()',
      suggestion: 'Add vi.restoreAllMocks() to afterEach hook',
    });
  }

  // Check for process.chdir without restoration
  if (hasChdirCall && !hasCwdRestoration) {
    violations.push({
      file: filePath,
      line: 1,
      column: 1,
      rule: 'require-cwd-restoration',
      message:
        'Test file calls process.chdir() but does not appear to restore the original directory',
      suggestion: 'Save process.cwd() before changing and restore in afterEach',
    });
  }

  // Check for temp directory creation without cleanup
  if (hasTempDirCreation && !hasTempDirCleanup) {
    violations.push({
      file: filePath,
      line: 1,
      column: 1,
      rule: 'require-temp-cleanup',
      message: 'Test file creates temp directories but does not appear to clean them up',
      suggestion: 'Add rmSync/rm cleanup in afterEach hook',
    });
  }

  return violations;
}

/**
 * Main audit function
 */
async function audit(): Promise<AuditResult> {
  const files = await findTestFiles();
  const allViolations: Violation[] = [];
  const filesWithViolations = new Set<string>();

  for (const file of files) {
    const violations = analyseFile(file);
    if (violations.length > 0) {
      filesWithViolations.add(file);
      allViolations.push(...violations);
    }
  }

  // Build summary
  const summary: Record<string, number> = {};
  for (const v of allViolations) {
    summary[v.rule] = (summary[v.rule] || 0) + 1;
  }

  return {
    totalFiles: files.length,
    filesWithViolations: filesWithViolations.size,
    violations: allViolations,
    summary,
  };
}

/**
 * Format output for console
 */
function formatConsole(result: AuditResult): void {
  console.log('\n📊 Test Quality Audit Report\n');
  console.log(`Files scanned: ${result.totalFiles}`);
  console.log(`Files with violations: ${result.filesWithViolations}`);
  console.log(`Total violations: ${result.violations.length}\n`);

  if (result.violations.length === 0) {
    console.log('✅ No violations found!\n');
    return;
  }

  console.log('📋 Summary by rule:');
  for (const [rule, count] of Object.entries(result.summary)) {
    console.log(`   ${rule}: ${count}`);
  }
  console.log('');

  // Group violations by file
  const byFile = new Map<string, Violation[]>();
  for (const v of result.violations) {
    const existing = byFile.get(v.file) || [];
    existing.push(v);
    byFile.set(v.file, existing);
  }

  for (const [file, violations] of byFile) {
    console.log(`\n📁 ${file}`);
    for (const v of violations) {
      console.log(`   ${v.line}:${v.column} - ${v.rule}`);
      console.log(`      ${v.message}`);
      if (v.suggestion) {
        console.log(`      💡 ${v.suggestion}`);
      }
    }
  }

  console.log('\n');
}

// Run the audit
const args = process.argv.slice(2);
const jsonOutput = args.includes('--json');

audit()
  .then((result) => {
    if (jsonOutput) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      formatConsole(result);
    }

    // Exit with error code if violations found
    process.exit(result.violations.length > 0 ? 1 : 0);
  })
  .catch((error) => {
    console.error('Audit failed:', error);
    process.exit(2);
  });
