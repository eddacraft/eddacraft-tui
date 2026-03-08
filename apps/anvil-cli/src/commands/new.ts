import { Command } from 'commander';
import { writeFileSync, existsSync } from 'node:fs';
import { validatePathWithinRoot } from '@eddacraft/anvil-core';
import {
  createTemplateLoader,
  type Template,
  type TemplateMetadata,
} from '../services/template-loader.js';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { theme } from '../tui/utils/theme.js';
import { CliError } from '../utils/cli-error.js';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { print } from '../utils/output.js';

interface NewCommandOptions {
  list?: boolean;
  output?: string;
  force?: boolean;
  // Commander.js --no-tui sets options.tui = false (not options.noTui = true)
  tui?: boolean;
  category?: string;
  var?: string[];
}

export function createNewCommand(): Command {
  const command = new Command('new')
    .argument('[template-id]', 'Template ID to use')
    .description('Create a new plan from a template')
    .option('-l, --list', 'List all available templates')
    .option('-o, --output <path>', 'Output file path')
    .option('-f, --force', 'Overwrite existing file')
    .option('--tui', 'Force TUI mode')
    .option('--no-tui', 'Force plain text mode')
    .option('-c, --category <category>', 'Filter templates by category')
    .option('--var <key=value...>', 'Set template variable (can be used multiple times)')
    .action(async (templateId: string | undefined, options: NewCommandOptions) => {
      try {
        await runNewCommand(templateId, options);
      } catch (error) {
        if (error instanceof CliError) throw error;
        print(`${theme.icons.error} ${error instanceof Error ? error.message : 'Unknown error'}`);
        throw new CliError('Command failed');
      }
    });

  return command;
}

async function runNewCommand(
  templateId: string | undefined,
  options: NewCommandOptions
): Promise<void> {
  const loader = createTemplateLoader();
  await loader.loadTemplates();

  const templates = loader.getAllTemplates();

  if (templates.length === 0) {
    print(`${theme.icons.error} No templates found`);
    throw new CliError('No templates found');
  }

  if (options.list) {
    printTemplateList(templates, options.category as TemplateMetadata['category'] | undefined);
    return;
  }

  const useTUI = isTUIAvailable({ tui: options.tui });

  if (!templateId) {
    if (useTUI) {
      const { showTemplateBrowser } = await import('../tui/commands/new/index.js');
      const result = await showTemplateBrowser(templates);

      if (!result) {
        print('Template selection cancelled');
        return;
      }

      const variables = result.variables;
      await generateFromTemplate(loader, result.templateId, variables, options);
    } else {
      print('Available templates:');
      printTemplateList(templates, options.category as TemplateMetadata['category'] | undefined);
      print('\nUsage: anvil new <template-id> [--var key=value]');
      return;
    }
  } else {
    const variables = parseVariables(options.var || []);
    await generateFromTemplate(loader, templateId, variables, options);
  }
}

async function generateFromTemplate(
  loader: ReturnType<typeof createTemplateLoader>,
  templateId: string,
  variables: Record<string, string>,
  options: NewCommandOptions
): Promise<void> {
  const template = loader.getTemplate(templateId);

  if (!template) {
    print(`${theme.icons.error} Template not found: ${templateId}`);
    print('\nAvailable templates:');
    loader.getAllTemplates().forEach((t) => {
      print(`  ${t.metadata.id} - ${t.metadata.name}`);
    });
    throw new CliError('Template not found');
  }

  const missingVars = template.metadata.variables
    .filter((v) => v.required && !variables[v.name] && !v.default)
    .map((v) => v.name);

  if (missingVars.length > 0) {
    print(`${theme.icons.error} Missing required variables: ${missingVars.join(', ')}`);
    print('\nRequired variables for this template:');
    template.metadata.variables
      .filter((v) => v.required)
      .forEach((v) => {
        const defaultStr = v.default ? ` (default: ${v.default})` : '';
        print(`  --var ${v.name}=<value>  ${v.description}${defaultStr}`);
      });
    throw new CliError('Missing required variables');
  }

  const rendered = loader.renderTemplate(template, variables);

  const outputPath = validatePathWithinRoot(
    options.output || `${templateId}.md`,
    getWorkspaceRoot()
  );

  if (existsSync(outputPath) && !options.force) {
    print(`${theme.icons.error} File already exists: ${outputPath}`);
    print('Use --force to overwrite');
    throw new CliError('File already exists');
  }

  writeFileSync(outputPath, rendered.content, 'utf-8');

  print(`${theme.icons.success} Created ${outputPath} from template "${template.metadata.name}"`);

  if (Object.keys(variables).length > 0) {
    print('\nVariables applied:');
    Object.entries(variables).forEach(([key, value]) => {
      print(`  ${key}: ${value}`);
    });
  }

  print(`\nNext steps:`);
  print(`  anvil validate ${outputPath}`);
}

function printTemplateList(
  templates: Template[],
  filterCategory?: TemplateMetadata['category']
): void {
  const filtered = filterCategory
    ? templates.filter((t) => t.metadata.category === filterCategory)
    : templates;

  if (filtered.length === 0) {
    print('No templates found');
    return;
  }

  const byCategory = new Map<string, Template[]>();
  for (const template of filtered) {
    const cat = template.metadata.category;
    if (!byCategory.has(cat)) {
      byCategory.set(cat, []);
    }
    byCategory.get(cat)!.push(template);
  }

  for (const [category, categoryTemplates] of byCategory) {
    print(`\n${theme.icons.arrow} ${category.toUpperCase()}`);
    for (const template of categoryTemplates) {
      print(`  ${template.metadata.id.padEnd(25)} ${template.metadata.description}`);
    }
  }

  print(`\nTotal: ${filtered.length} templates`);
}

function parseVariables(vars: string[]): Record<string, string> {
  const result: Record<string, string> = {};

  for (const v of vars) {
    const eqIndex = v.indexOf('=');
    if (eqIndex === -1) {
      throw new Error(`Invalid variable format: ${v} (expected key=value)`);
    }
    const key = v.slice(0, eqIndex);
    const value = v.slice(eqIndex + 1);
    result[key] = value;
  }

  return result;
}
