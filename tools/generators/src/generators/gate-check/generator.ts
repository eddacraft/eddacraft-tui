import { Tree, formatFiles, joinPathFragments } from '@nx/devkit';

export interface GateCheckGeneratorSchema {
  name: string;
  description?: string;
}

interface NormalizedSchema extends GateCheckGeneratorSchema {
  pascalName: string;
  kebabName: string;
}

function kebabToPascalCase(kebab: string): string {
  return kebab
    .split('-')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join('');
}

function normalizeOptions(options: GateCheckGeneratorSchema): NormalizedSchema {
  const kebabName = options.name.toLowerCase().replace(/\s+/g, '-');
  const pascalName = kebabToPascalCase(kebabName);

  return {
    ...options,
    pascalName,
    kebabName,
  };
}

function addFiles(tree: Tree, options: NormalizedSchema) {
  const checkPath = joinPathFragments(
    'packages/anvil/runtime/src/gate/checks',
    `${options.kebabName}.check.ts`
  );
  const testPath = joinPathFragments(
    'packages/anvil/runtime/src/gate/checks',
    `${options.kebabName}.check.test.ts`
  );

  const description = options.description || `Validates ${options.kebabName}`;

  const checkContent = `import { BaseCheck } from '../check.interface.js';
import type { CheckContext, GateResult } from '../../types/gate.types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const log = createDebugger('check');

// Register in gate-runner.ts: this.registerCheck(new ${options.pascalName}Check());
export class ${options.pascalName}Check extends BaseCheck {
  name = '${options.kebabName}';
  description = '${description}';

  async run(context: CheckContext): Promise<GateResult> {
    log('Running ${options.kebabName} check');
    // Implement check logic
    return this.createSuccess('Check passed', 100);
  }
}
`;

  tree.write(checkPath, checkContent);

  const testContent = `import { describe, it, expect, beforeEach } from 'vitest';
import { ${options.pascalName}Check } from './${options.kebabName}.check.js';
import type { CheckContext } from '../../types/gate.types.js';

describe('${options.pascalName}Check', () => {
  let check: ${options.pascalName}Check;
  let baseContext: CheckContext;

  beforeEach(() => {
    check = new ${options.pascalName}Check();
    baseContext = {
      plan: undefined,
      workspace_root: '/tmp/test',
      config: {
        version: 1,
        checks: [],
        thresholds: { overall_score: 80 },
      },
      check_config: {},
    };
  });

  it('should have correct name and description', () => {
    expect(check.name).toBe('${options.kebabName}');
    expect(check.description).toBeDefined();
  });

  it('should pass with default implementation', async () => {
    const result = await check.run(baseContext);
    expect(result.passed).toBe(true);
  });
});
`;

  tree.write(testPath, testContent);
}

export default async function (tree: Tree, options: GateCheckGeneratorSchema) {
  const normalizedOptions = normalizeOptions(options);
  addFiles(tree, normalizedOptions);
  await formatFiles(tree);
}
