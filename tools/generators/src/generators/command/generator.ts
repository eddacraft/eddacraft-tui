import { Tree, formatFiles, joinPathFragments } from '@nx/devkit';

export interface CommandGeneratorSchema {
  name: string;
  description?: string;
}

interface NormalizedSchema extends CommandGeneratorSchema {
  pascalName: string;
  kebabName: string;
}

function kebabToPascalCase(kebab: string): string {
  return kebab
    .split('-')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join('');
}

function normalizeOptions(options: CommandGeneratorSchema): NormalizedSchema {
  const kebabName = options.name.toLowerCase().trim().replace(/\s+/g, '-');
  const pascalName = kebabToPascalCase(kebabName);

  return {
    ...options,
    kebabName,
    pascalName,
  };
}

function addFiles(tree: Tree, options: NormalizedSchema) {
  const commandPath = joinPathFragments('apps/anvil-cli/src/commands', `${options.kebabName}.ts`);
  const testPath = joinPathFragments('apps/anvil-cli/src/commands', `${options.kebabName}.test.ts`);

  const description = options.description ?? options.name;

  const commandContent = `import { Command } from 'commander';
import { createDebugger } from '@eddacraft/anvil-core';

const log = createDebugger('cli');

interface ${options.pascalName}Options {
  json?: boolean;
}

export function create${options.pascalName}Command(): Command {
  const command = new Command('${options.kebabName}')
    .description('${description}')
    .option('--json', 'Output as JSON')
    .action(async (options: ${options.pascalName}Options) => {
      log('${options.kebabName} command called with options: %O', options);
      // TODO: implement command logic
    });
  return command;
}
`;

  const testContent = `import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('@eddacraft/anvil-core', () => ({
  createDebugger: vi.fn(() => vi.fn()),
}));

describe('${options.kebabName} command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', async () => {
    const { create${options.pascalName}Command } = await import('./${options.kebabName}.js');
    const command = create${options.pascalName}Command();

    expect(command.name()).toBe('${options.kebabName}');
    expect(command.description()).toBeDefined();
  });

  it('should register json option', async () => {
    const { create${options.pascalName}Command } = await import('./${options.kebabName}.js');
    const command = create${options.pascalName}Command();

    expect(command.options.find((option) => option.long === '--json')).toBeDefined();
  });
});
`;

  tree.write(commandPath, commandContent);
  tree.write(testPath, testContent);
}

export default async function (tree: Tree, options: CommandGeneratorSchema) {
  const normalizedOptions = normalizeOptions(options);
  addFiles(tree, normalizedOptions);
  await formatFiles(tree);
}
