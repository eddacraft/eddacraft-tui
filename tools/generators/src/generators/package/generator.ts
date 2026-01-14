import {
  Tree,
  formatFiles,
  generateFiles,
  names,
  offsetFromRoot,
  joinPathFragments,
} from '@nx/devkit';
import * as path from 'path';

export interface PackageGeneratorSchema {
  name: string;
  directory: string;
  description?: string;
  publishable?: boolean;
}

interface NormalizedSchema extends PackageGeneratorSchema {
  projectName: string;
  projectRoot: string;
  projectDirectory: string;
  parsedTags: string[];
  npmScope: string;
  importPath: string;
}

function normalizeOptions(
  tree: Tree,
  options: PackageGeneratorSchema
): NormalizedSchema {
  const projectDirectory = options.name;
  const projectRoot = joinPathFragments(options.directory, projectDirectory);
  const projectName = options.name.replace(/\//g, '-');

  // Determine npm scope based on directory
  let npmScope = '@anvil';
  let importPath = `${npmScope}/${options.name}`;

  // For platform/shared packages, use the directory as part of the import
  if (options.directory === 'packages/platform') {
    importPath = `${npmScope}/platform/${options.name}`;
  } else if (options.directory === 'packages/shared') {
    importPath = `${npmScope}/shared/${options.name}`;
  }

  // Determine tags based on directory
  const parsedTags: string[] = [];
  if (options.directory.includes('anvil')) {
    parsedTags.push('scope:core');
  } else if (options.directory.includes('platform')) {
    parsedTags.push('scope:platform');
  } else if (options.directory.includes('shared')) {
    parsedTags.push('scope:shared');
  } else if (options.directory.includes('adapters')) {
    parsedTags.push('scope:adapters');
  }

  return {
    ...options,
    projectName,
    projectRoot,
    projectDirectory,
    parsedTags,
    npmScope,
    importPath,
  };
}

function addFiles(tree: Tree, options: NormalizedSchema) {
  const templateOptions = {
    ...options,
    ...names(options.name),
    offsetFromRoot: offsetFromRoot(options.projectRoot),
    template: '',
  };

  // Create package.json
  const packageJson = {
    name: options.importPath,
    version: '0.0.1',
    description: options.description || `Anvil ${options.name} package`,
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
    ...(options.publishable
      ? {}
      : { private: true }),
    dependencies: {},
    devDependencies: {
      vitest: '^4.0.17',
    },
  };

  tree.write(
    joinPathFragments(options.projectRoot, 'package.json'),
    JSON.stringify(packageJson, null, 2)
  );

  // Create tsconfig.json
  const tsconfig = {
    extends: `${offsetFromRoot(options.projectRoot)}packages/tooling/tsconfig/base.json`,
    compilerOptions: {
      outDir: './dist',
      rootDir: './src',
    },
    include: ['src/**/*.ts'],
    exclude: ['node_modules', 'dist', '**/*.test.ts', '**/*.spec.ts'],
  };

  tree.write(
    joinPathFragments(options.projectRoot, 'tsconfig.json'),
    JSON.stringify(tsconfig, null, 2)
  );

  // Create tsconfig.lib.json
  const tsconfigLib = {
    extends: './tsconfig.json',
    compilerOptions: {
      declaration: true,
      declarationMap: true,
    },
  };

  tree.write(
    joinPathFragments(options.projectRoot, 'tsconfig.lib.json'),
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
    joinPathFragments(options.projectRoot, 'tsconfig.spec.json'),
    JSON.stringify(tsconfigSpec, null, 2)
  );

  // Create project.json for Nx
  const projectJson = {
    name: options.projectName,
    $schema: `${offsetFromRoot(options.projectRoot)}node_modules/nx/schemas/project-schema.json`,
    sourceRoot: `${options.projectRoot}/src`,
    projectType: 'library',
    tags: options.parsedTags,
    targets: {},
  };

  tree.write(
    joinPathFragments(options.projectRoot, 'project.json'),
    JSON.stringify(projectJson, null, 2)
  );

  // Create src/index.ts
  tree.write(
    joinPathFragments(options.projectRoot, 'src/index.ts'),
    `/**
 * ${options.description || `Anvil ${options.name} package`}
 * @module ${options.importPath}
 */

export {};
`
  );

  // Create src/index.test.ts
  tree.write(
    joinPathFragments(options.projectRoot, 'src/index.test.ts'),
    `import { describe, it, expect } from 'vitest';

describe('${options.name}', () => {
  it('should be defined', () => {
    expect(true).toBe(true);
  });
});
`
  );

  // Create vitest.config.ts
  tree.write(
    joinPathFragments(options.projectRoot, 'vitest.config.ts'),
    `import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
  },
});
`
  );
}

export default async function (tree: Tree, options: PackageGeneratorSchema) {
  const normalizedOptions = normalizeOptions(tree, options);
  addFiles(tree, normalizedOptions);
  await formatFiles(tree);
}
