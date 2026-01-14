/**
 * Import path rewrite transform
 *
 * Uses ts-morph to analyze and rewrite @anvil/* imports
 * based on the monorepo migration mapping.
 */

import { Project, SourceFile, ImportDeclaration, SyntaxKind } from 'ts-morph';
import { getRewrittenPath, getPackageForSymbol } from '../import-mappings.js';

export interface TransformOptions {
  dryRun?: boolean;
  verbose?: boolean;
}

export interface TransformResult {
  file: string;
  changes: ImportChange[];
  errors: string[];
}

export interface ImportChange {
  line: number;
  original: string;
  rewritten: string;
  symbols?: string[];
}

/**
 * Rewrites imports in a source file
 */
export function rewriteImportsInFile(
  sourceFile: SourceFile,
  options: TransformOptions = {}
): TransformResult {
  const result: TransformResult = {
    file: sourceFile.getFilePath(),
    changes: [],
    errors: [],
  };

  const imports = sourceFile.getImportDeclarations();

  for (const importDecl of imports) {
    try {
      const change = processImportDeclaration(importDecl);
      if (change) {
        result.changes.push(change);
        if (!options.dryRun) {
          importDecl.setModuleSpecifier(change.rewritten);
        }
      }
    } catch (error) {
      result.errors.push(
        `Error processing import at line ${importDecl.getStartLineNumber()}: ${error}`
      );
    }
  }

  // Handle complex cases where symbols need to be split across packages
  const splitChanges = processSplitImports(sourceFile, options);
  result.changes.push(...splitChanges);

  if (!options.dryRun && result.changes.length > 0) {
    sourceFile.saveSync();
  }

  return result;
}

/**
 * Processes a single import declaration
 */
function processImportDeclaration(importDecl: ImportDeclaration): ImportChange | null {
  const moduleSpecifier = importDecl.getModuleSpecifierValue();

  // Only process @anvil/* imports
  if (!moduleSpecifier.startsWith('@anvil/')) {
    return null;
  }

  // Skip if not a core import (adapters, aps, etc. don't need rewriting yet)
  if (!moduleSpecifier.startsWith('@anvil/core')) {
    return null;
  }

  const rewritten = getRewrittenPath(moduleSpecifier);
  if (!rewritten || rewritten === moduleSpecifier) {
    return null;
  }

  const namedImports = importDecl.getNamedImports();
  const symbols = namedImports.map((ni) => ni.getName());

  return {
    line: importDecl.getStartLineNumber(),
    original: moduleSpecifier,
    rewritten,
    symbols: symbols.length > 0 ? symbols : undefined,
  };
}

/**
 * Handles cases where a single import needs to be split into multiple packages
 *
 * For example:
 *   import { APSPlanSchema, GateRunner, OPAExecutor } from '@anvil/core';
 *
 * Becomes:
 *   import { APSPlanSchema } from '@anvil/contracts';
 *   import { GateRunner } from '@anvil/runtime';
 *   import { OPAExecutor } from '@anvil/policy';
 */
function processSplitImports(
  sourceFile: SourceFile,
  options: TransformOptions
): ImportChange[] {
  const changes: ImportChange[] = [];
  const imports = sourceFile.getImportDeclarations();

  for (const importDecl of imports) {
    const moduleSpecifier = importDecl.getModuleSpecifierValue();

    // Only handle bare @anvil/core imports that might need splitting
    if (moduleSpecifier !== '@anvil/core') {
      continue;
    }

    const namedImports = importDecl.getNamedImports();
    if (namedImports.length === 0) {
      continue;
    }

    // Group symbols by their target package
    const symbolsByPackage = new Map<string, string[]>();

    for (const namedImport of namedImports) {
      const symbol = namedImport.getName();
      const targetPackage = getPackageForSymbol(symbol);

      if (targetPackage) {
        if (!symbolsByPackage.has(targetPackage)) {
          symbolsByPackage.set(targetPackage, []);
        }
        symbolsByPackage.get(targetPackage)!.push(symbol);
      } else {
        // Unknown symbol, keep in @anvil/contracts as default
        if (!symbolsByPackage.has('@anvil/contracts')) {
          symbolsByPackage.set('@anvil/contracts', []);
        }
        symbolsByPackage.get('@anvil/contracts')!.push(symbol);
      }
    }

    // If all symbols go to the same package, simple rewrite
    if (symbolsByPackage.size === 1) {
      const [targetPackage, symbols] = [...symbolsByPackage.entries()][0];
      if (targetPackage !== '@anvil/core') {
        changes.push({
          line: importDecl.getStartLineNumber(),
          original: moduleSpecifier,
          rewritten: targetPackage,
          symbols,
        });

        if (!options.dryRun) {
          importDecl.setModuleSpecifier(targetPackage);
        }
      }
      continue;
    }

    // Multiple packages - need to split the import
    if (!options.dryRun) {
      const line = importDecl.getStartLineNumber();
      const insertIndex = sourceFile.getImportDeclarations().indexOf(importDecl);

      // Remove the original import
      importDecl.remove();

      // Add new imports for each package
      let idx = insertIndex;
      for (const [pkg, symbols] of symbolsByPackage) {
        sourceFile.insertImportDeclaration(idx, {
          moduleSpecifier: pkg,
          namedImports: symbols,
        });
        idx++;

        changes.push({
          line,
          original: moduleSpecifier,
          rewritten: pkg,
          symbols,
        });
      }
    } else {
      // Dry run - just record the changes
      for (const [pkg, symbols] of symbolsByPackage) {
        changes.push({
          line: importDecl.getStartLineNumber(),
          original: moduleSpecifier,
          rewritten: pkg,
          symbols,
        });
      }
    }
  }

  return changes;
}

/**
 * Creates a ts-morph project and processes all TypeScript files
 */
export function createProject(tsConfigPath?: string): Project {
  return new Project({
    tsConfigFilePath: tsConfigPath,
    skipAddingFilesFromTsConfig: true,
  });
}

/**
 * Adds files to the project using glob patterns
 */
export function addFilesToProject(
  project: Project,
  patterns: string[],
  excludePatterns: string[] = []
): SourceFile[] {
  const sourceFiles: SourceFile[] = [];

  for (const pattern of patterns) {
    const files = project.addSourceFilesAtPaths(pattern);
    sourceFiles.push(...files);
  }

  // Remove excluded files
  for (const exclude of excludePatterns) {
    const excluded = project.getSourceFiles(exclude);
    for (const file of excluded) {
      project.removeSourceFile(file);
    }
  }

  return sourceFiles;
}
