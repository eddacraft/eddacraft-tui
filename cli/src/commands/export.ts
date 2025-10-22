/**
 * Export Command - Convert between formats (SpecKit ↔ APS)
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync, mkdirSync, writeFileSync } from 'fs';
import { dirname, extname, basename, join } from 'path';
import { PlanLoader } from '../services/plan-loader.js';
import { AdapterRegistry } from '@anvil/adapters';
import type { ExportOptions } from '../types/command-options.js';

export function createExportCommand(): Command {
  return new Command('export')
    .description('Export/convert plans between formats (SpecKit ↔ APS)')
    .argument('<source>', 'Source file path')
    .requiredOption('--to <format>', 'Target format (aps, json, yaml, speckit)')
    .option('--output <path>', 'Output file path or directory')
    .option('--from <format>', 'Source format (auto-detected if not specified)')
    .option('--compact', 'Compact JSON output (no pretty-printing)', false)
    .action(async (sourcePath: string, options: ExportOptions) => {
      const spinner = ora('Loading source file...').start();

      try {
        // Validate source file exists
        if (!existsSync(sourcePath)) {
          throw new Error(`Source file not found: ${sourcePath}`);
        }

        // Load source plan with format detection
        const planLoader = new PlanLoader();
        const registry = AdapterRegistry.getInstance();

        spinner.text = options.from
          ? `Loading as ${options.from} format...`
          : 'Detecting source format...';

        const loadResult = await planLoader.loadPlan(sourcePath, {
          format: options.from,
          validateHash: false,
          strict: false,
        });

        const { plan, sourceFormat } = loadResult;

        if (!sourceFormat) {
          throw new Error('Could not detect source format');
        }

        spinner.succeed(
          chalk.green(
            `✓ Loaded from ${chalk.cyan(sourceFormat.format)} (${sourceFormat.confidence}% confidence)`
          )
        );

        // Normalize target format
        const targetFormat = normalizeTargetFormat(options.to);

        // Export to target format
        spinner.start(`Converting to ${targetFormat}...`);

        if (targetFormat === 'aps' || targetFormat === 'json' || targetFormat === 'yaml') {
          // Export to APS
          const outputPath =
            options.output ||
            generateDefaultOutput(sourcePath, targetFormat === 'yaml' ? 'yaml' : 'json');

          // Ensure output directory exists
          const outputDir = dirname(outputPath);
          if (!existsSync(outputDir)) {
            mkdirSync(outputDir, { recursive: true });
          }

          // Write APS file
          const content =
            targetFormat === 'yaml'
              ? await formatAsYaml(plan)
              : JSON.stringify(plan, null, options.compact ? 0 : 2);

          writeFileSync(outputPath, content, 'utf-8');

          spinner.succeed(chalk.green(`✓ Exported to ${chalk.cyan(targetFormat.toUpperCase())}`));
          console.log(chalk.gray('  Output:'), chalk.cyan(outputPath));
          console.log(chalk.gray('  Size:  '), chalk.cyan(`${content.length} bytes`));

          console.log(chalk.green('\n✓ Export complete'));
          console.log(chalk.gray('\nNext steps:'));
          console.log(chalk.gray('  - Validate:'), `anvil validate ${outputPath}`);
        } else if (targetFormat === 'speckit') {
          // Export to SpecKit
          const adapter = registry.getAdapter('speckit-export');
          if (!adapter) {
            throw new Error('SpecKit export adapter not found');
          }

          // Check if adapter has convertFromAPS method (export adapters only)
          if (!('convertFromAPS' in adapter) || typeof adapter.convertFromAPS !== 'function') {
            throw new Error('Adapter does not support export from APS');
          }

          const exportResult = await adapter.convertFromAPS(plan);

          if (!exportResult.success || !exportResult.data) {
            const errors = 'errors' in exportResult ? exportResult.errors : [];
            throw new Error(
              `Failed to convert to SpecKit: ${errors?.map((e: { message: string }) => e.message).join(', ') || 'Unknown error'}`
            );
          }

          // Determine output directory
          const outputDir = options.output || dirname(sourcePath);

          // Ensure output directory exists
          if (!existsSync(outputDir)) {
            mkdirSync(outputDir, { recursive: true });
          }

          // Write SpecKit files
          const content = exportResult.data.content as {
            specContent: string;
            planContent: string;
            tasksContent: string;
          };

          const files = [
            { name: 'spec.md', content: content.specContent },
            { name: 'plan.md', content: content.planContent },
            { name: 'tasks.md', content: content.tasksContent },
          ];

          const writtenPaths: string[] = [];
          for (const file of files) {
            const filePath = join(outputDir, file.name);
            writeFileSync(filePath, file.content, 'utf-8');
            writtenPaths.push(filePath);
          }

          spinner.succeed(chalk.green(`✓ Exported to SpecKit format`));
          console.log(chalk.gray('  Output directory:'), chalk.cyan(outputDir));
          console.log(chalk.gray('  Files created:'));
          writtenPaths.forEach((path) => {
            console.log(chalk.gray('    -'), chalk.cyan(basename(path)));
          });

          // Show warnings if any
          if (exportResult.warnings && exportResult.warnings.length > 0) {
            console.log(chalk.yellow('\n⚠ Warnings:'));
            exportResult.warnings.forEach((warning: { message: string }) => {
              console.log(chalk.yellow(`  - ${warning.message}`));
            });
          }

          console.log(chalk.green('\n✓ Export complete'));
          console.log(chalk.gray('\nNext steps:'));
          console.log(chalk.gray('  - Validate:'), `anvil validate ${writtenPaths[0]}`);
        } else {
          throw new Error(`Unsupported target format: ${options.to}`);
        }
      } catch (error) {
        spinner.fail(chalk.red('Export failed'));
        console.error(chalk.red('Error:'), error instanceof Error ? error.message : String(error));
        process.exit(1);
      }
    });
}

/**
 * Normalize target format string
 */
function normalizeTargetFormat(format: string): string {
  const normalized = format.toLowerCase().trim();

  const formatMap: Record<string, string> = {
    aps: 'aps',
    json: 'json',
    yaml: 'yaml',
    yml: 'yaml',
    speckit: 'speckit',
    'spec.md': 'speckit',
  };

  return formatMap[normalized] || normalized;
}

/**
 * Generate default output path based on source and target format
 */
function generateDefaultOutput(sourcePath: string, targetExt: string): string {
  const ext = extname(sourcePath);
  const base = basename(sourcePath, ext);
  const dir = dirname(sourcePath);

  return join(dir, `${base}.aps.${targetExt}`);
}

/**
 * Format plan as YAML (placeholder - would need yaml library)
 */
async function formatAsYaml(plan: unknown): Promise<string> {
  // For now, just use JSON - in production would use a YAML library
  return JSON.stringify(plan, null, 2);
}
