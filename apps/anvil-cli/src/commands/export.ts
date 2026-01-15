/**
 * Export Command - Convert between formats (SpecKit ↔ APS) or export constraints
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync, mkdirSync, writeFileSync } from 'fs';
import { dirname, extname, basename, join } from 'path';
import { PlanLoader } from '../services/plan-loader.js';
import { AdapterRegistry } from '@anvil/adapters';
import {
  collectConstraints,
  formatAsLlmsTxt,
  formatAsMcpResourceJson,
  formatAsPrompt,
} from '@anvil/runtime';
import type { ExportOptions } from '../types/command-options.js';

export function createExportCommand(): Command {
  return new Command('export')
    .description('Export/convert plans between formats or export constraints')
    .argument('[source]', 'Source file path (optional for constraint export)')
    .option('--to <format>', 'Target format for plan conversion (aps, json, yaml, speckit)')
    .option(
      '--format <format>',
      'Output format for constraint export (llms.txt, mcp-resource, prompt-fragment)'
    )
    .option('--output <path>', 'Output file path or directory')
    .option('--from <format>', 'Source format (auto-detected if not specified)')
    .option('--compact', 'Compact JSON output (no pretty-printing)', false)
    .action(
      async (sourcePath: string | undefined, options: ExportOptions & { format?: string }) => {
        // Handle constraint export (--format llms.txt, mcp-resource, prompt-fragment)
        if (options.format) {
          return await exportConstraints(options.format, options.output, options.compact);
        }

        // Handle plan conversion (--to aps, json, yaml, speckit)
        if (!options.to) {
          console.error(chalk.red('Error: Either --format or --to must be specified'));
          console.log(chalk.gray('\nExamples:'));
          console.log(chalk.gray('  Constraint export:'), 'anvil export --format llms.txt');
          console.log(chalk.gray('  Plan conversion:  '), 'anvil export source.md --to json');
          process.exit(1);
        }

        if (!sourcePath) {
          console.error(chalk.red('Error: Source file path is required for plan conversion'));
          process.exit(1);
        }

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
          console.error(
            chalk.red('Error:'),
            error instanceof Error ? error.message : String(error)
          );
          process.exit(1);
        }
      }
    );
}

/**
 * Export constraints in specified format
 */
async function exportConstraints(
  format: string,
  outputPath?: string,
  compact = false
): Promise<void> {
  const spinner = ora('Collecting constraints...').start();

  try {
    // Get workspace root (current directory)
    const workspaceRoot = process.cwd();

    // Collect constraints
    const constraints = await collectConstraints(workspaceRoot);

    spinner.text = `Formatting as ${format}...`;

    // Format based on requested format
    let content: string;
    let defaultFilename: string;
    let mimeType: string;

    const normalizedFormat = normalizeConstraintFormat(format);

    switch (normalizedFormat) {
      case 'llms.txt':
        content = formatAsLlmsTxt(constraints);
        defaultFilename = '.llms.txt';
        mimeType = 'text/markdown';
        break;

      case 'mcp-resource':
        content = formatAsMcpResourceJson(constraints, !compact);
        defaultFilename = 'anvil-constraints.mcp.json';
        mimeType = 'application/json';
        break;

      case 'prompt-fragment':
        content = formatAsPrompt(constraints);
        defaultFilename = 'anvil-constraints-prompt.txt';
        mimeType = 'text/plain';
        break;

      default:
        throw new Error(
          `Unsupported format: ${format}. Supported formats: llms.txt, mcp-resource, prompt-fragment`
        );
    }

    // Determine output path
    const finalOutputPath = outputPath || join(workspaceRoot, defaultFilename);

    // Ensure output directory exists
    const outputDir = dirname(finalOutputPath);
    if (!existsSync(outputDir)) {
      mkdirSync(outputDir, { recursive: true });
    }

    // Write output file
    writeFileSync(finalOutputPath, content, 'utf-8');

    spinner.succeed(chalk.green(`✓ Exported constraints as ${normalizedFormat}`));
    console.log(chalk.gray('  Output:'), chalk.cyan(finalOutputPath));
    console.log(chalk.gray('  Format:'), chalk.cyan(mimeType));
    console.log(chalk.gray('  Size:  '), chalk.cyan(`${content.length} bytes`));

    // Show constraint counts
    console.log(chalk.gray('\n  Constraints exported:'));
    if (constraints.boundaries.length > 0) {
      console.log(chalk.gray('    - Boundaries:   '), chalk.cyan(constraints.boundaries.length));
    }
    if (constraints.layers.length > 0) {
      console.log(chalk.gray('    - Layers:       '), chalk.cyan(constraints.layers.length));
    }
    if (constraints.antiPatterns.length > 0) {
      console.log(chalk.gray('    - Anti-patterns:'), chalk.cyan(constraints.antiPatterns.length));
    }
    if (constraints.conventions.length > 0) {
      console.log(chalk.gray('    - Conventions:  '), chalk.cyan(constraints.conventions.length));
    }

    console.log(chalk.green('\n✓ Export complete'));
  } catch (error) {
    spinner.fail(chalk.red('Export failed'));
    console.error(chalk.red('Error:'), error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

/**
 * Normalize constraint format string
 */
function normalizeConstraintFormat(format: string): string {
  const normalized = format.toLowerCase().trim();

  const formatMap: Record<string, string> = {
    'llms.txt': 'llms.txt',
    llmstxt: 'llms.txt',
    llms: 'llms.txt',
    'mcp-resource': 'mcp-resource',
    mcp: 'mcp-resource',
    'prompt-fragment': 'prompt-fragment',
    prompt: 'prompt-fragment',
  };

  return formatMap[normalized] || normalized;
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

async function formatAsYaml(_plan: unknown): Promise<string> {
  throw new Error(
    'YAML export is not yet implemented. Use --format json instead, or install the yaml package.'
  );
}
