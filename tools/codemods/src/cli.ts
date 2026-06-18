#!/usr/bin/env node
/**
 * Anvil Codemod CLI
 *
 * Usage:
 *   pnpm codemod:imports          # Run import path rewrite
 *   pnpm codemod:imports --dry-run  # Preview changes without applying
 */

import { program } from 'commander';
import chalk from 'chalk';
import { glob } from 'glob';
import { resolve, relative } from 'node:path';
import {
  createProject,
  rewriteImportsInFile,
  type TransformResult,
} from './transforms/rewrite-imports.js';
import { printImportsCompletion } from './imports-completion.js';

const ROOT_DIR = resolve(import.meta.dirname, '../../..');
const DEFAULT_EXCLUDE_PATTERNS = [
  '**/node_modules/**',
  '**/dist/**',
  '**/.nx/**',
  '**/tools/codemods/**',
];

program.name('anvil-codemod').description('Codemods for Anvil monorepo migration').version('0.0.1');

program
  .command('imports')
  .description('Rewrite @eddacraft/anvil-* import paths for monorepo migration')
  .option('-d, --dry-run', 'Preview changes without applying', false)
  .option('-v, --verbose', 'Show detailed output', false)
  .option('-p, --path <glob>', 'Glob pattern for files to process', '**/*.ts')
  .option('--exclude <patterns...>', 'Patterns to exclude')
  .action(async (options) => {
    console.log(chalk.blue('\n  Anvil Import Path Codemod\n'));

    if (options.dryRun) {
      console.log(chalk.yellow('  [DRY RUN] No files will be modified.\n'));
    }

    const project = createProject();

    // Find all TypeScript files
    const pattern = resolve(ROOT_DIR, options.path);
    const excludePatterns = (options.exclude ?? DEFAULT_EXCLUDE_PATTERNS).map((p: string) =>
      resolve(ROOT_DIR, p)
    );

    console.log(chalk.gray(`  Searching: ${relative(ROOT_DIR, pattern)}`));

    const files = await glob(pattern, {
      ignore: excludePatterns,
      absolute: true,
    });

    console.log(chalk.gray(`  Found ${files.length} TypeScript files\n`));

    // Add files to project
    for (const file of files) {
      project.addSourceFileAtPath(file);
    }

    const sourceFiles = project.getSourceFiles();
    const results: TransformResult[] = [];
    let totalChanges = 0;
    let totalErrors = 0;

    // Process each file
    for (const sourceFile of sourceFiles) {
      const result = rewriteImportsInFile(sourceFile, {
        dryRun: options.dryRun,
        verbose: options.verbose,
      });

      if (result.changes.length > 0 || result.errors.length > 0) {
        results.push(result);
        totalChanges += result.changes.length;
        totalErrors += result.errors.length;
      }
    }

    // Print results
    if (results.length === 0) {
      console.log(chalk.green('  No import changes needed.\n'));
      return;
    }

    console.log(chalk.blue('  Changes:\n'));

    for (const result of results) {
      const relPath = relative(ROOT_DIR, result.file);

      if (result.changes.length > 0) {
        console.log(chalk.white(`  ${relPath}`));

        for (const change of result.changes) {
          const symbolInfo = change.symbols ? ` {${change.symbols.join(', ')}}` : '';
          console.log(
            chalk.gray(`    L${change.line}: `) +
              chalk.red(change.original) +
              chalk.gray(' -> ') +
              chalk.green(change.rewritten) +
              chalk.cyan(symbolInfo)
          );
        }
        console.log();
      }

      if (result.errors.length > 0) {
        for (const error of result.errors) {
          console.log(chalk.red(`    Error: ${error}`));
        }
      }
    }

    // Summary
    console.log(chalk.blue('  Summary:\n'));
    console.log(chalk.white(`    Files with changes: ${results.length}`));
    console.log(chalk.white(`    Total imports rewritten: ${totalChanges}`));

    if (totalErrors > 0) {
      console.log(chalk.red(`    Errors: ${totalErrors}`));
    }

    printImportsCompletion({ dryRun: options.dryRun, totalErrors });
  });

await program.parseAsync(process.argv);
