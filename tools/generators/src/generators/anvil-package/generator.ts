import { Tree, formatFiles, joinPathFragments, offsetFromRoot } from '@nx/devkit';

export interface AnvilPackageGeneratorSchema {
  name: 'contracts' | 'ports' | 'core' | 'runtime' | 'policy' | 'sdk';
  description?: string;
}

/**
 * Package dependency configuration
 * Defines the allowed dependencies for each anvil package
 */
const PACKAGE_CONFIG: Record<string, { description: string; deps: string[]; layer: number }> = {
  contracts: {
    description: 'Schemas, types, and events with zero dependencies',
    deps: [],
    layer: 0,
  },
  ports: {
    description: 'Interface definitions depending only on contracts',
    deps: ['@eddacraft/anvil-contracts'],
    layer: 1,
  },
  core: {
    description: 'Pure domain logic depending on ports and contracts',
    deps: ['@eddacraft/anvil-contracts', '@eddacraft/anvil-ports'],
    layer: 2,
  },
  policy: {
    description: 'OPA/Rego wrappers depending on contracts',
    deps: ['@eddacraft/anvil-contracts'],
    layer: 2,
  },
  runtime: {
    description: 'Orchestration and I/O depending on core, ports, contracts',
    deps: [
      '@eddacraft/anvil-contracts',
      '@eddacraft/anvil-ports',
      '@eddacraft/anvil-core',
      '@eddacraft/anvil-policy',
    ],
    layer: 3,
  },
  sdk: {
    description: 'Client SDK depending on contracts and ports',
    deps: ['@eddacraft/anvil-contracts', '@eddacraft/anvil-ports'],
    layer: 2,
  },
};

export default async function (tree: Tree, options: AnvilPackageGeneratorSchema) {
  const config = PACKAGE_CONFIG[options.name];
  const projectRoot = `packages/anvil/${options.name}`;
  const importPath = `@eddacraft/anvil-${options.name}`;
  const description = options.description || config.description;

  // Build dependencies object
  const dependencies: Record<string, string> = {};
  for (const dep of config.deps) {
    dependencies[dep] = 'workspace:*';
  }

  // Add zod for contracts package
  if (options.name === 'contracts') {
    dependencies['zod'] = '^4.3.5';
  }

  // Create package.json
  const packageJson = {
    name: importPath,
    version: '0.0.1',
    description,
    type: 'module',
    main: './dist/index.js',
    types: './dist/index.d.ts',
    exports: {
      '.': {
        types: './dist/index.d.ts',
        import: './dist/index.js',
      },
    },
    scripts: {
      build: 'tsc -p tsconfig.lib.json',
      test: 'vitest run',
      'test:watch': 'vitest',
      typecheck: 'tsc --noEmit',
    },
    dependencies,
    devDependencies: {
      vitest: '^4.0.17',
    },
  };

  tree.write(joinPathFragments(projectRoot, 'package.json'), JSON.stringify(packageJson, null, 2));

  // Create tsconfig.json
  const offset = offsetFromRoot(projectRoot);
  const tsconfig = {
    extends: `${offset}packages/tooling/tsconfig/base.json`,
    compilerOptions: {
      outDir: './dist',
      rootDir: './src',
    },
    include: ['src/**/*.ts'],
    exclude: ['node_modules', 'dist', '**/*.test.ts', '**/*.spec.ts'],
  };

  tree.write(joinPathFragments(projectRoot, 'tsconfig.json'), JSON.stringify(tsconfig, null, 2));

  // Create tsconfig.lib.json
  const tsconfigLib = {
    extends: './tsconfig.json',
    compilerOptions: {
      declaration: true,
      declarationMap: true,
    },
  };

  tree.write(
    joinPathFragments(projectRoot, 'tsconfig.lib.json'),
    JSON.stringify(tsconfigLib, null, 2)
  );

  // Create tsconfig.spec.json
  const tsconfigSpec = {
    extends: './tsconfig.json',
    compilerOptions: {
      types: ['vitest/globals', 'node'],
    },
    include: ['src/**/*.test.ts', 'src/**/*.spec.ts'],
  };

  tree.write(
    joinPathFragments(projectRoot, 'tsconfig.spec.json'),
    JSON.stringify(tsconfigSpec, null, 2)
  );

  // Create project.json for Nx
  const projectJson = {
    name: options.name,
    $schema: `${offset}node_modules/nx/schemas/project-schema.json`,
    sourceRoot: `${projectRoot}/src`,
    projectType: 'library',
    tags: ['scope:core', `layer:${config.layer}`],
    targets: {},
  };

  tree.write(joinPathFragments(projectRoot, 'project.json'), JSON.stringify(projectJson, null, 2));

  // Create src/index.ts with appropriate exports
  let indexContent = `/**
 * ${description}
 * @module ${importPath}
 */

`;

  if (options.name === 'contracts') {
    indexContent += `// Re-export schemas
export * from './schemas/index.js';

// Re-export types
export * from './types/index.js';

// Re-export events
export * from './events/index.js';
`;
  } else if (options.name === 'ports') {
    indexContent += `// Re-export interfaces
export * from './interfaces/index.js';
`;
  } else {
    indexContent += `export {};
`;
  }

  tree.write(joinPathFragments(projectRoot, 'src/index.ts'), indexContent);

  // Create directory structure based on package type
  if (options.name === 'contracts') {
    tree.write(
      joinPathFragments(projectRoot, 'src/schemas/index.ts'),
      '// Zod schemas\nexport {};\n'
    );
    tree.write(
      joinPathFragments(projectRoot, 'src/types/index.ts'),
      '// Type definitions\nexport {};\n'
    );
    tree.write(
      joinPathFragments(projectRoot, 'src/events/index.ts'),
      '// Event schemas\nexport {};\n'
    );
  } else if (options.name === 'ports') {
    tree.write(
      joinPathFragments(projectRoot, 'src/interfaces/index.ts'),
      '// Interface definitions\nexport {};\n'
    );
  }

  // Create src/index.test.ts
  tree.write(
    joinPathFragments(projectRoot, 'src/index.test.ts'),
    `import { describe, it, expect } from 'vitest';

describe('${importPath}', () => {
  it('should be defined', () => {
    expect(true).toBe(true);
  });
});
`
  );

  // Create vitest.config.ts
  tree.write(
    joinPathFragments(projectRoot, 'vitest.config.ts'),
    `import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
  },
});
`
  );

  await formatFiles(tree);
}
