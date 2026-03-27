import { Command } from 'commander';
import inquirer from 'inquirer';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import YAML from 'yaml';
import { createDebugger, validatePathWithinRoot } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../utils/cli-error.js';
import { print, blank, data, json, debug } from '../utils/output.js';
import {
  type ArchitectureTemplate,
  type ArchitectureDefinition,
  type LayerDefinition,
  getAvailableTemplates,
  getTemplateDefaults,
  architectureYamlExists,
  getArchitectureYamlPath,
  parseArchitectureDefinition,
  compileArchitecture,
  needsCompilation,
  getDCConfigPath,
  getRegoPath,
  ARCHITECTURE_YAML_FILENAME,
  ARCHITECTURE_DEFINITION_VERSION,
  getDefaultOptions,
} from '@eddacraft/anvil-core';
import { renderMermaidAscii, renderMermaid } from 'beautiful-mermaid';

const log = createDebugger('cli');

/** Template metadata for display */
const TEMPLATE_INFO: Record<
  ArchitectureTemplate,
  { title: string; description: string; layers: string[] }
> = {
  starter: {
    title: 'Starter',
    description: 'Simple and flexible structure for new projects, MVPs, and learning',
    layers: ['components', 'lib', 'services'],
  },
  layered: {
    title: 'Layered Architecture',
    description: 'Classic 3-tier architecture - ideal for APIs and web backends',
    layers: ['presentation', 'business', 'data', 'shared'],
  },
  hexagonal: {
    title: 'Hexagonal (Ports & Adapters)',
    description: 'Core domain isolated from external concerns via ports and adapters',
    layers: ['core', 'ports', 'adapters', 'application'],
  },
  clean: {
    title: 'Clean Architecture',
    description: "Uncle Bob's architecture with strict dependency rules",
    layers: ['entities', 'use_cases', 'interface_adapters', 'frameworks'],
  },
  ddd: {
    title: 'Domain-Driven Design',
    description: 'Organise code around business domains and bounded contexts',
    layers: ['domain', 'application', 'infrastructure', 'interfaces'],
  },
  monorepo: {
    title: 'Monorepo',
    description: 'Multi-package workspace with shared libraries and clear boundaries',
    layers: ['apps', 'packages', 'shared'],
  },
  serverless: {
    title: 'Serverless',
    description: 'Functions-as-a-Service for AWS Lambda, Azure Functions, etc.',
    layers: ['functions', 'services', 'shared'],
  },
  'nx-workspace': {
    title: 'Nx Workspace',
    description: 'Nx monorepo with apps, feature libs, and shared libs',
    layers: ['apps', 'feature-libs', 'data-access-libs', 'ui-libs', 'shared-libs'],
  },
  custom: {
    title: 'Custom Architecture',
    description: 'Start with an empty template and define your own layers',
    layers: [],
  },
};

/** Print the welcome banner */
function printWelcomeBanner(): void {
  blank();
  print(chalk.bold.cyan('  Anvil Architecture Setup'));
  print(chalk.dim('  ─────────────────────────────────────────'));
  blank();
  print(chalk.dim('  Define your project structure to enforce dependency rules'));
  print(chalk.dim('  and prevent architectural violations.'));
  blank();
}

/** Build a mermaid definition from template layer list */
function templateToMermaid(layers: string[]): string {
  if (layers.length === 0) return '';
  return `graph TD\n  ${layers.join(' --> ')}`;
}

/** Print template details */
function printTemplatePreview(template: ArchitectureTemplate): void {
  const info = TEMPLATE_INFO[template];
  blank();
  print(chalk.dim('  ─────────────────────────────────────────'));
  print(`  ${chalk.bold(info.title)}`);
  print(`  ${chalk.dim(info.description)}`);
  if (info.layers.length > 0) {
    blank();
    try {
      const mermaidDef = templateToMermaid(info.layers);
      const ascii = renderMermaidAscii(mermaidDef, { paddingX: 2, paddingY: 1 });
      ascii.split('\n').forEach((line) => print(chalk.dim('  ' + line)));
    } catch {
      debug('architecture: mermaid rendering failed, falling back to arrow notation');
      // Fall back to simple arrow notation
      print(`  ${chalk.cyan('Layers:')} ${info.layers.join(' → ')}`);
    }
  }
  print(chalk.dim('  ─────────────────────────────────────────'));
  blank();
}

/** Run the interactive architecture wizard */
async function runInteractiveWizard(options: { force?: boolean }): Promise<void> {
  log(`architecture: interactive wizard started force=${options.force}`);
  const projectRoot = process.cwd();
  const yamlPath = getArchitectureYamlPath(projectRoot);
  const exists = architectureYamlExists(projectRoot);

  printWelcomeBanner();

  // Check if architecture already exists
  if (exists && !options.force) {
    print(chalk.yellow('  An architecture definition already exists.'));
    blank();

    const { action } = await inquirer.prompt<{
      action: 'show' | 'validate' | 'generate' | 'replace' | 'exit';
    }>([
      {
        type: 'select',
        name: 'action',
        message: 'What would you like to do?',
        choices: [
          { name: 'View current architecture', value: 'show' },
          { name: 'Validate configuration', value: 'validate' },
          { name: 'Regenerate configs', value: 'generate' },
          { name: 'Replace with new template (overwrites)', value: 'replace' },
          new inquirer.Separator(),
          { name: 'Exit', value: 'exit' },
        ],
      },
    ]);

    switch (action) {
      case 'show':
        await showArchitectureDefinition(projectRoot, {});
        return;
      case 'validate':
        await validateArchitectureDefinition(projectRoot);
        return;
      case 'generate':
        await generateArchitectureConfigs(projectRoot, {});
        return;
      case 'replace':
        // Continue to template selection
        break;
      case 'exit':
      default:
        return;
    }
  }

  // Template selection
  print(chalk.bold('  Choose an Architecture Pattern'));
  blank();

  const templates = getAvailableTemplates();
  const choices = templates.map((t) => {
    const info = TEMPLATE_INFO[t];
    return {
      name: `${info.title}\n     ${chalk.dim(info.description)}`,
      value: t,
      short: info.title,
    };
  });

  const { template } = await inquirer.prompt<{ template: ArchitectureTemplate }>([
    {
      type: 'select',
      name: 'template',
      message: 'Select a template:',
      choices,
      pageSize: 10,
    },
  ]);

  // Show template preview
  printTemplatePreview(template);

  // Confirm selection
  const { confirmed } = await inquirer.prompt<{ confirmed: boolean }>([
    {
      type: 'confirm',
      name: 'confirmed',
      message: `Create ${TEMPLATE_INFO[template].title} configuration?`,
      default: true,
    },
  ]);

  if (!confirmed) {
    print(chalk.dim('\n  Setup cancelled.\n'));
    return;
  }

  // Create the architecture file
  await createArchitectureFile(projectRoot, yamlPath, template);
}

/** Create the architecture.yaml file */
async function createArchitectureFile(
  projectRoot: string,
  yamlPath: string,
  template: ArchitectureTemplate,
  layerPaths?: Record<string, string[]>
): Promise<void> {
  blank();
  const spinner = ora({
    text: 'Creating architecture configuration...',
    indent: 2,
  }).start();

  try {
    const templateLayers = getTemplateDefaults(template);
    const layers: Record<string, LayerDefinition> = {};

    for (const [name, def] of Object.entries(templateLayers)) {
      layers[name] = {
        patterns: layerPaths?.[name] ?? def.patterns,
        depends_on: def.depends_on,
        description: def.description,
      };
    }

    const definition: ArchitectureDefinition = {
      schema_version: ARCHITECTURE_DEFINITION_VERSION,
      template,
      layers,
      rules: [],
      options: getDefaultOptions(),
    };

    const yamlDir = dirname(yamlPath);
    if (!existsSync(yamlDir)) {
      await mkdir(yamlDir, { recursive: true });
    }

    const content = YAML.stringify(definition, { indent: 2 });
    await writeFile(yamlPath, content, 'utf-8');

    spinner.succeed(chalk.green('Architecture configuration created'));

    // Success output - simple and clean
    blank();
    print(chalk.bold.green('  Configuration created successfully'));
    blank();
    print(`  ${chalk.cyan('File:')}     .anvil/${ARCHITECTURE_YAML_FILENAME}`);
    print(`  ${chalk.cyan('Template:')} ${template}`);
    print(`  ${chalk.cyan('Layers:')}   ${Object.keys(layers).length}`);

    // Next steps
    blank();
    print(chalk.bold('  Next Steps'));
    blank();
    print(chalk.white('  1.') + chalk.dim(' Review layer paths in .anvil/architecture.yaml'));
    print(chalk.white('  2.') + chalk.dim(' Generate enforcement configs:'));
    print(chalk.cyan('     anvil arch generate'));
    print(chalk.white('  3.') + chalk.dim(' Run architecture checks:'));
    print(chalk.cyan('     anvil gate --only-checks architecture'));
    blank();
  } catch (err) {
    if (err instanceof CliError || err instanceof CliExit) throw err;
    spinner.fail('Failed to create architecture configuration');
    print(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
    throw new CliError('Failed to create architecture configuration');
  }
}

/** Show architecture definition (extracted for reuse) */
async function showArchitectureDefinition(
  projectRoot: string,
  options: { json?: boolean; yaml?: boolean }
): Promise<void> {
  const definition = await parseArchitectureDefinition(projectRoot);

  if (options.json) {
    json(definition);
    return;
  }

  if (options.yaml) {
    data(YAML.stringify(definition, { indent: 2 }));
    return;
  }

  blank();
  print(chalk.bold.cyan('  Architecture Definition'));
  print(chalk.dim('  ─────────────────────────────────────────'));
  blank();
  print(`  ${chalk.cyan('Template:')}  ${definition.template}`);
  print(`  ${chalk.cyan('Schema:')}    ${definition.schema_version}`);

  // Show dependency graph
  blank();
  try {
    const lines = ['graph TD'];
    for (const [name, layer] of Object.entries(definition.layers)) {
      for (const dep of layer.depends_on) {
        if (definition.layers[dep]) {
          lines.push(`  ${name} --> ${dep}`);
        }
      }
    }
    if (lines.length > 1) {
      const ascii = renderMermaidAscii(lines.join('\n'), { paddingX: 2, paddingY: 1 });
      ascii.split('\n').forEach((line) => print(chalk.dim('  ' + line)));
      blank();
    }
  } catch {
    debug('architecture: diagram rendering failed, skipping');
    // Diagram rendering failed — skip silently, details below are sufficient
  }

  print(chalk.bold('  Layers'));
  blank();
  for (const [name, layer] of Object.entries(definition.layers)) {
    print(`  ${chalk.cyan(name)}`);
    print(chalk.dim(`    Patterns:   ${layer.patterns.join(', ')}`));
    print(
      chalk.dim(
        `    Depends on: ${layer.depends_on.length > 0 ? layer.depends_on.join(', ') : '(none)'}`
      )
    );
    if (layer.description) {
      print(chalk.dim(`    ${layer.description}`));
    }
    blank();
  }

  if (definition.rules.length > 0) {
    print(chalk.bold('  Custom Rules'));
    blank();
    for (const rule of definition.rules) {
      const arrow = rule.allowed ? chalk.green('→') : chalk.red('✗');
      print(`  ${arrow} ${rule.name}: ${rule.from} → ${rule.to} [${rule.severity}]`);
    }
    blank();
  }

  if (definition.options) {
    print(chalk.bold('  Options'));
    blank();
    print(chalk.dim(`  Detect circular: ${definition.options.detect_circular}`));
    print(chalk.dim(`  Detect orphans:  ${definition.options.detect_orphans}`));
    print(chalk.dim(`  Default severity: ${definition.options.default_severity}`));
    blank();
  }
}

/** Validate architecture definition (extracted for reuse) */
async function validateArchitectureDefinition(projectRoot: string): Promise<void> {
  blank();
  const spinner = ora({ text: 'Validating architecture.yaml...', indent: 2 }).start();

  try {
    const definition = await parseArchitectureDefinition(projectRoot);
    const issues: string[] = [];

    for (const [name, layer] of Object.entries(definition.layers)) {
      for (const dep of layer.depends_on) {
        if (!definition.layers[dep]) {
          issues.push(`Layer "${name}" depends on unknown layer "${dep}"`);
        }
      }
    }

    for (const rule of definition.rules) {
      if (!definition.layers[rule.from]) {
        issues.push(`Rule "${rule.name}" references unknown source layer "${rule.from}"`);
      }
      if (!definition.layers[rule.to]) {
        issues.push(`Rule "${rule.name}" references unknown target layer "${rule.to}"`);
      }
    }

    if (issues.length > 0) {
      spinner.fail(chalk.red('Validation failed'));
      blank();
      for (const issue of issues) {
        print(chalk.yellow(`  • ${issue}`));
      }
      blank();
      throw new CliError('Architecture definition has validation errors');
    }

    spinner.succeed(chalk.green('Architecture configuration is valid'));
    blank();
    print(chalk.dim(`  Template: ${definition.template}`));
    print(chalk.dim(`  Layers:   ${Object.keys(definition.layers).length}`));
    print(chalk.dim(`  Rules:    ${definition.rules.length}`));
    blank();
  } catch (err) {
    if (err instanceof CliError || err instanceof CliExit) throw err;
    spinner.fail('Validation failed');
    print(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
    throw new CliError('Architecture validation failed');
  }
}

/** Generate architecture configs (extracted for reuse) */
async function generateArchitectureConfigs(
  projectRoot: string,
  options: { force?: boolean; skipDc?: boolean; skipRego?: boolean }
): Promise<void> {
  blank();
  const spinner = ora({ text: 'Checking configuration status...', indent: 2 }).start();

  try {
    if (!options.force) {
      const needs = await needsCompilation(projectRoot);
      const skipDC = options.skipDc || !needs.dc;
      const skipRego = options.skipRego || !needs.rego;

      if (skipDC && skipRego) {
        spinner.succeed(chalk.green('All configs are up to date'));
        blank();
        print(chalk.dim(`  DC config:   ${getDCConfigPath(projectRoot)}`));
        print(chalk.dim(`  Rego policy: ${getRegoPath(projectRoot)}`));
        blank();
        return;
      }
    }

    spinner.text = 'Generating enforcement configs...';
    const result = await compileArchitecture(projectRoot, {
      force: options.force,
      skipDC: options.skipDc,
      skipRego: options.skipRego,
    });

    spinner.succeed(chalk.green('Architecture configs generated'));

    blank();
    if (result.dcConfig.regenerated) {
      print(chalk.dim(`  DC config:   ${result.dcConfig.path}`) + chalk.green(' (updated)'));
    } else if (!options.skipDc) {
      print(chalk.dim(`  DC config:   ${result.dcConfig.path} (unchanged)`));
    }

    if (result.regoPolicy.regenerated) {
      print(chalk.dim(`  Rego policy: ${result.regoPolicy.path}`) + chalk.green(' (updated)'));
    } else if (!options.skipRego) {
      print(chalk.dim(`  Rego policy: ${result.regoPolicy.path} (unchanged)`));
    }

    blank();
    print(chalk.bold('  Next Steps'));
    blank();
    print(chalk.dim('  Run architecture checks:'));
    print(chalk.cyan('  anvil gate --only-checks architecture'));
    blank();
  } catch (err) {
    if (err instanceof CliError || err instanceof CliExit) throw err;
    spinner.fail('Failed to generate configs');
    print(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
    throw new CliError('Failed to generate architecture configs');
  }
}

export function createArchitectureCommand(): Command {
  const command = new Command('architecture')
    .alias('arch')
    .description('Manage architecture definition and dependency rules')
    .action(async () => {
      // When called without subcommand, run interactive wizard
      await runInteractiveWizard({});
    });

  command.addCommand(createInitSubcommand());
  command.addCommand(createGenerateSubcommand());
  command.addCommand(createValidateSubcommand());
  command.addCommand(createShowSubcommand());
  command.addCommand(createVisualiseSubcommand());

  return command;
}

function createInitSubcommand(): Command {
  return new Command('init')
    .description('Create architecture.yaml from a template')
    .option(
      '-t, --template <template>',
      'Architecture template (layered, hexagonal, clean, ddd, custom)'
    )
    .option('--force', 'Overwrite existing architecture.yaml')
    .option('--non-interactive', 'Skip prompts, use defaults or --template')
    .action(async (options: { template?: string; force?: boolean; nonInteractive?: boolean }) => {
      log(
        `architecture init: template=${options.template} force=${options.force} nonInteractive=${options.nonInteractive}`
      );
      const projectRoot = process.cwd();
      const yamlPath = getArchitectureYamlPath(projectRoot);

      if (architectureYamlExists(projectRoot) && !options.force) {
        blank();
        print(chalk.yellow(`  ${ARCHITECTURE_YAML_FILENAME} already exists.`));
        print(chalk.dim('  Use --force to overwrite, or edit the existing file.'));
        blank();
        throw new CliError('Architecture definition already exists');
      }

      let template: ArchitectureTemplate;
      let layerPaths: Record<string, string[]> | undefined;

      if (options.nonInteractive) {
        // Non-interactive mode: just create with defaults
        template = validateTemplate(options.template) || 'layered';
        await createArchitectureFile(projectRoot, yamlPath, template, layerPaths);
        return;
      }

      if (options.template) {
        // Template specified: validate and optionally customise
        const validated = validateTemplate(options.template);
        if (!validated) {
          blank();
          print(chalk.red(`  Invalid template: ${options.template}`));
          print(chalk.dim(`  Available templates: ${getAvailableTemplates().join(', ')}`));
          blank();
          throw new CliError(`Invalid architecture template: ${options.template}`);
        }
        template = validated;

        // Show what they're getting
        printTemplatePreview(template);

        const customise = await inquirer.prompt<{ customiseLayers: boolean }>([
          {
            type: 'confirm',
            name: 'customiseLayers',
            message: 'Would you like to customise layer paths?',
            default: false,
          },
        ]);

        if (customise.customiseLayers) {
          layerPaths = await promptLayerPaths(template);
        }

        await createArchitectureFile(projectRoot, yamlPath, template, layerPaths);
        return;
      }

      // Interactive mode: full wizard experience
      await runInteractiveWizard({ force: options.force });
    });
}

function createGenerateSubcommand(): Command {
  return new Command('generate')
    .alias('gen')
    .description('Generate DC config and Rego policies from architecture.yaml')
    .option('--force', 'Regenerate even if up to date')
    .option('--skip-dc', 'Skip dependency-cruiser config generation')
    .option('--skip-rego', 'Skip Rego policy generation')
    .action(async (options: { force?: boolean; skipDc?: boolean; skipRego?: boolean }) => {
      log(
        `architecture generate: force=${options.force} skipDc=${options.skipDc} skipRego=${options.skipRego}`
      );
      const projectRoot = process.cwd();

      if (!architectureYamlExists(projectRoot)) {
        blank();
        print(chalk.red('  No architecture.yaml found.'));
        print(chalk.dim('  Run: anvil arch init'));
        blank();
        throw new CliError('Architecture definition not found for generate');
      }

      await generateArchitectureConfigs(projectRoot, options);
    });
}

function createValidateSubcommand(): Command {
  return new Command('validate')
    .description('Validate architecture.yaml syntax and references')
    .action(async () => {
      const projectRoot = process.cwd();

      if (!architectureYamlExists(projectRoot)) {
        blank();
        print(chalk.red('  No architecture.yaml found.'));
        print(chalk.dim('  Run: anvil arch init'));
        blank();
        throw new CliError('Architecture definition not found for validate');
      }

      await validateArchitectureDefinition(projectRoot);
    });
}

function createShowSubcommand(): Command {
  return new Command('show')
    .description('Display current architecture definition')
    .option('--json', 'Output as JSON')
    .option('--yaml', 'Output as YAML')
    .action(async (options: { json?: boolean; yaml?: boolean }) => {
      const projectRoot = process.cwd();

      if (!architectureYamlExists(projectRoot)) {
        blank();
        print(chalk.red('  No architecture.yaml found.'));
        print(chalk.dim('  Run: anvil arch init'));
        blank();
        throw new CliError('Architecture definition not found for show');
      }

      try {
        await showArchitectureDefinition(projectRoot, options);
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        print(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
        throw new CliError('Failed to show architecture definition');
      }
    });
}

function createVisualiseSubcommand(): Command {
  return new Command('visualise')
    .alias('visualize')
    .alias('viz')
    .description('Visualise architecture dependency graph')
    .option('-f, --format <format>', 'Output format: ascii (default), svg, mermaid', 'ascii')
    .option('-o, --output <path>', 'Write output to file instead of stdout')
    .action(async (options: { format?: string; output?: string }) => {
      const projectRoot = process.cwd();

      if (!architectureYamlExists(projectRoot)) {
        blank();
        print(chalk.red('  No architecture.yaml found.'));
        print(chalk.dim('  Run: anvil arch init'));
        blank();
        throw new CliError('Architecture definition not found for visualise');
      }

      try {
        const definition = await parseArchitectureDefinition(projectRoot);

        // Build mermaid definition from layers
        const mermaidLines = ['graph TD'];
        // Declare all layer nodes so isolated layers still appear
        for (const name of Object.keys(definition.layers)) {
          mermaidLines.push(`  ${name}`);
        }
        for (const [name, layer] of Object.entries(definition.layers)) {
          for (const dep of layer.depends_on) {
            if (definition.layers[dep]) {
              mermaidLines.push(`  ${name} --> ${dep}`);
            }
          }
        }
        const mermaidDef = mermaidLines.join('\n');

        const format = options.format ?? 'ascii';
        let output: string;

        switch (format) {
          case 'mermaid':
            output = mermaidDef;
            break;
          case 'svg': {
            try {
              output = await renderMermaid(mermaidDef);
            } catch {
              print(chalk.yellow('  SVG rendering failed — falling back to raw Mermaid'));
              output = mermaidDef;
            }
            break;
          }
          case 'ascii':
          default:
            try {
              output = renderMermaidAscii(mermaidDef, { paddingX: 2, paddingY: 1 });
            } catch {
              print(chalk.yellow('  ASCII rendering failed — falling back to raw Mermaid'));
              output = mermaidDef;
            }
            break;
        }

        if (options.output) {
          const validatedOutput = validatePathWithinRoot(options.output, projectRoot);
          const outDir = dirname(validatedOutput);
          if (!existsSync(outDir)) {
            await mkdir(outDir, { recursive: true });
          }
          await writeFile(validatedOutput, output, 'utf-8');
          print(chalk.green(`  Written to ${validatedOutput}`));
        } else {
          blank();
          print(chalk.bold.cyan('  Architecture Dependency Graph'));
          print(chalk.dim(`  Template: ${definition.template}`));
          blank();
          data(output);
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        print(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
        throw new CliError('Architecture visualisation failed');
      }
    });
}

function validateTemplate(template: string | undefined): ArchitectureTemplate | null {
  if (!template) return null;
  const available = getAvailableTemplates();
  return available.includes(template as ArchitectureTemplate)
    ? (template as ArchitectureTemplate)
    : null;
}

async function promptLayerPaths(template: ArchitectureTemplate): Promise<Record<string, string[]>> {
  const defaults = getTemplateDefaults(template);
  const result: Record<string, string[]> = {};

  print(chalk.dim('\nEnter glob patterns for each layer (comma-separated):'));

  for (const [name, def] of Object.entries(defaults)) {
    const answer = await inquirer.prompt([
      {
        type: 'input',
        name: 'patterns',
        message: `${name}:`,
        default: def.patterns.join(', '),
      },
    ]);
    result[name] = answer.patterns
      .split(',')
      .map((p: string) => p.trim())
      .filter(Boolean);
  }

  return result;
}
