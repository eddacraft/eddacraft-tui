import { Command } from 'commander';
import inquirer from 'inquirer';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync } from 'fs';
import { mkdir, writeFile } from 'fs/promises';
import { dirname } from 'path';
import YAML from 'yaml';
import {
  type ArchitectureTemplate,
  type ArchitectureDefinition,
  type LayerDefinition,
  getAvailableTemplates,
  getTemplateDefaults,
  mergeWithTemplate,
  architectureYamlExists,
  getArchitectureYamlPath,
  parseArchitectureDefinition,
  writeDCConfig,
  needsRegeneration,
  dcConfigExists,
  getDCConfigPath,
  ARCHITECTURE_YAML_FILENAME,
  getDefaultOptions,
} from '@anvil/core';

export function createArchitectureCommand(): Command {
  const command = new Command('architecture')
    .alias('arch')
    .description('Manage architecture definition and dependency rules');

  command.addCommand(createInitSubcommand());
  command.addCommand(createGenerateSubcommand());
  command.addCommand(createValidateSubcommand());
  command.addCommand(createShowSubcommand());

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
      const projectRoot = process.cwd();
      const yamlPath = getArchitectureYamlPath(projectRoot);

      if (architectureYamlExists(projectRoot) && !options.force) {
        console.log(chalk.yellow(`\n${ARCHITECTURE_YAML_FILENAME} already exists.`));
        console.log(chalk.dim('Use --force to overwrite, or edit the existing file.'));
        process.exit(1);
      }

      let template: ArchitectureTemplate;
      let layerPaths: Record<string, string[]> | undefined;

      if (options.nonInteractive) {
        template = validateTemplate(options.template) || 'layered';
      } else if (options.template) {
        const validated = validateTemplate(options.template);
        if (!validated) {
          console.log(chalk.red(`\nInvalid template: ${options.template}`));
          console.log(chalk.dim(`Available templates: ${getAvailableTemplates().join(', ')}`));
          process.exit(1);
        }
        template = validated;

        const customise = await inquirer.prompt([
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
      } else {
        const answers = await inquirer.prompt([
          {
            type: 'list',
            name: 'template',
            message: 'Select an architecture template:',
            choices: [
              { name: 'Layered (presentation → business → data)', value: 'layered' },
              { name: 'Hexagonal (ports & adapters)', value: 'hexagonal' },
              { name: 'Clean Architecture (entities → use cases → adapters)', value: 'clean' },
              { name: 'DDD (domain-driven design)', value: 'ddd' },
              { name: 'Custom (empty, define your own)', value: 'custom' },
            ],
            default: 'layered',
          },
        ]);
        template = answers.template;

        if (template !== 'custom') {
          const customise = await inquirer.prompt([
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
        }
      }

      const spinner = ora('Creating architecture.yaml...').start();

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
          schema_version: '0.1.0',
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

        spinner.succeed(chalk.green(`Created ${ARCHITECTURE_YAML_FILENAME}`));

        console.log(chalk.dim(`\nFile: ${yamlPath}`));
        console.log(chalk.dim(`Template: ${template}`));
        console.log(chalk.dim(`Layers: ${Object.keys(layers).join(', ')}`));

        console.log(chalk.cyan('\nNext steps:'));
        console.log(chalk.dim('  1. Review and customise layer paths in architecture.yaml'));
        console.log(chalk.dim('  2. Generate DC config: anvil architecture generate'));
        console.log(
          chalk.dim('  3. Run architecture check: anvil gate --only-checks architecture')
        );
      } catch (err) {
        spinner.fail('Failed to create architecture.yaml');
        console.log(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
        process.exit(1);
      }
    });
}

function createGenerateSubcommand(): Command {
  return new Command('generate')
    .alias('gen')
    .description('Generate dependency-cruiser config from architecture.yaml')
    .option('--force', 'Regenerate even if up to date')
    .action(async (options: { force?: boolean }) => {
      const projectRoot = process.cwd();

      if (!architectureYamlExists(projectRoot)) {
        console.log(chalk.red('\nNo architecture.yaml found.'));
        console.log(chalk.dim('Run: anvil architecture init'));
        process.exit(1);
      }

      const spinner = ora('Loading architecture definition...').start();

      try {
        const definition = await parseArchitectureDefinition(projectRoot);
        const merged = mergeWithTemplate(definition);

        spinner.text = 'Checking if regeneration needed...';

        if (!options.force && dcConfigExists(projectRoot)) {
          const needsRegen = await needsRegeneration(projectRoot, merged);
          if (!needsRegen) {
            spinner.succeed(chalk.green('DC config is up to date'));
            console.log(chalk.dim(`\nFile: ${getDCConfigPath(projectRoot)}`));
            return;
          }
        }

        spinner.text = 'Generating dependency-cruiser config...';
        const configPath = await writeDCConfig(projectRoot, merged);

        spinner.succeed(chalk.green('Generated dependency-cruiser config'));
        console.log(chalk.dim(`\nFile: ${configPath}`));

        console.log(chalk.cyan('\nNext steps:'));
        console.log(chalk.dim('  Run architecture check: anvil gate --only-checks architecture'));
        console.log(
          chalk.dim('  Or directly: npx depcruise --config .anvil/dependency-cruiser.js src')
        );
      } catch (err) {
        spinner.fail('Failed to generate config');
        console.log(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
        process.exit(1);
      }
    });
}

function createValidateSubcommand(): Command {
  return new Command('validate')
    .description('Validate architecture.yaml syntax and references')
    .action(async () => {
      const projectRoot = process.cwd();

      if (!architectureYamlExists(projectRoot)) {
        console.log(chalk.red('\nNo architecture.yaml found.'));
        console.log(chalk.dim('Run: anvil architecture init'));
        process.exit(1);
      }

      const spinner = ora('Validating architecture.yaml...').start();

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
          console.log('');
          for (const issue of issues) {
            console.log(chalk.yellow(`  • ${issue}`));
          }
          process.exit(1);
        }

        spinner.succeed(chalk.green('architecture.yaml is valid'));
        console.log(chalk.dim(`\nTemplate: ${definition.template}`));
        console.log(chalk.dim(`Layers: ${Object.keys(definition.layers).length}`));
        console.log(chalk.dim(`Rules: ${definition.rules.length}`));
      } catch (err) {
        spinner.fail('Validation failed');
        console.log(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
        process.exit(1);
      }
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
        console.log(chalk.red('\nNo architecture.yaml found.'));
        console.log(chalk.dim('Run: anvil architecture init'));
        process.exit(1);
      }

      try {
        const definition = await parseArchitectureDefinition(projectRoot);

        if (options.json) {
          console.log(JSON.stringify(definition, null, 2));
          return;
        }

        if (options.yaml) {
          console.log(YAML.stringify(definition, { indent: 2 }));
          return;
        }

        console.log(chalk.bold('\nArchitecture Definition'));
        console.log(chalk.dim('─'.repeat(40)));
        console.log(chalk.cyan('Template:'), definition.template);
        console.log(chalk.cyan('Schema:'), definition.schema_version);

        console.log(chalk.bold('\nLayers:'));
        for (const [name, layer] of Object.entries(definition.layers)) {
          console.log(chalk.cyan(`  ${name}:`));
          console.log(chalk.dim(`    Patterns: ${layer.patterns.join(', ')}`));
          console.log(
            chalk.dim(
              `    Depends on: ${layer.depends_on.length > 0 ? layer.depends_on.join(', ') : '(none)'}`
            )
          );
          if (layer.description) {
            console.log(chalk.dim(`    Description: ${layer.description}`));
          }
        }

        if (definition.rules.length > 0) {
          console.log(chalk.bold('\nRules:'));
          for (const rule of definition.rules) {
            const arrow = rule.allowed ? chalk.green('→') : chalk.red('✗');
            console.log(`  ${arrow} ${rule.name}: ${rule.from} → ${rule.to} [${rule.severity}]`);
          }
        }

        if (definition.options) {
          console.log(chalk.bold('\nOptions:'));
          console.log(chalk.dim(`  Detect circular: ${definition.options.detect_circular}`));
          console.log(chalk.dim(`  Detect orphans: ${definition.options.detect_orphans}`));
          console.log(chalk.dim(`  Default severity: ${definition.options.default_severity}`));
        }

        console.log('');
      } catch (err) {
        console.log(chalk.red(err instanceof Error ? err.message : 'Unknown error'));
        process.exit(1);
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

  console.log(chalk.dim('\nEnter glob patterns for each layer (comma-separated):'));

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
