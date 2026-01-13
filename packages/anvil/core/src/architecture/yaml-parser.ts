import { readFile, writeFile } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import YAML from 'yaml';
import {
  ArchitectureDefinitionSchema,
  type ArchitectureDefinition,
  type ArchitectureTemplate,
  type LayerDefinition,
  getDefaultOptions,
} from './definition-schema.js';

export const ARCHITECTURE_YAML_FILENAME = 'architecture.yaml';
export const ANVIL_DIR = '.anvil';

export function getArchitectureYamlPath(workspaceRoot: string): string {
  return join(workspaceRoot, ANVIL_DIR, ARCHITECTURE_YAML_FILENAME);
}

export function architectureYamlExists(workspaceRoot: string): boolean {
  return existsSync(getArchitectureYamlPath(workspaceRoot));
}

export async function parseArchitectureDefinition(
  workspaceRoot: string
): Promise<ArchitectureDefinition> {
  const yamlPath = getArchitectureYamlPath(workspaceRoot);
  if (!existsSync(yamlPath)) {
    throw new Error(`Architecture YAML not found: ${yamlPath}`);
  }
  const content = await readFile(yamlPath, 'utf-8');
  const raw = YAML.parse(content);
  const result = ArchitectureDefinitionSchema.safeParse(raw);
  if (!result.success) {
    throw new Error(`Invalid architecture.yaml: ${result.error.message}`);
  }
  return applyDefaults(result.data);
}

export async function writeArchitectureYaml(
  workspaceRoot: string,
  definition: ArchitectureDefinition
): Promise<void> {
  const yamlPath = getArchitectureYamlPath(workspaceRoot);
  const content = YAML.stringify(definition, { indent: 2 });
  await writeFile(yamlPath, content, 'utf-8');
}

function applyDefaults(definition: ArchitectureDefinition): ArchitectureDefinition {
  return {
    ...definition,
    options: definition.options ?? getDefaultOptions(),
  };
}

type LayersRecord = Record<string, LayerDefinition>;

const LAYERED_TEMPLATE: LayersRecord = {
  presentation: {
    patterns: ['src/controllers/**', 'src/routes/**', 'src/api/**'],
    depends_on: ['business', 'shared'],
  },
  business: {
    patterns: ['src/services/**', 'src/use-cases/**'],
    depends_on: ['data', 'shared'],
  },
  data: {
    patterns: ['src/repositories/**', 'src/db/**', 'src/data/**'],
    depends_on: ['shared'],
  },
  shared: {
    patterns: ['src/utils/**', 'src/lib/**', 'src/common/**'],
    depends_on: [],
  },
};

const HEXAGONAL_TEMPLATE: LayersRecord = {
  core: {
    patterns: ['src/domain/**', 'src/core/**'],
    depends_on: [],
    description: 'Domain logic - no external dependencies',
  },
  ports: {
    patterns: ['src/ports/**', 'src/interfaces/**'],
    depends_on: ['core'],
    description: 'Port interfaces',
  },
  adapters: {
    patterns: ['src/adapters/**', 'src/infrastructure/**'],
    depends_on: ['ports', 'core'],
    description: 'Adapter implementations',
  },
  application: {
    patterns: ['src/application/**', 'src/services/**'],
    depends_on: ['core', 'ports'],
    description: 'Application services',
  },
};

const CLEAN_TEMPLATE: LayersRecord = {
  entities: {
    patterns: ['src/entities/**', 'src/domain/entities/**'],
    depends_on: [],
    description: 'Enterprise business rules',
  },
  use_cases: {
    patterns: ['src/use-cases/**', 'src/application/**'],
    depends_on: ['entities'],
    description: 'Application business rules',
  },
  interface_adapters: {
    patterns: ['src/adapters/**', 'src/controllers/**', 'src/presenters/**'],
    depends_on: ['use_cases', 'entities'],
    description: 'Interface adapters',
  },
  frameworks: {
    patterns: ['src/frameworks/**', 'src/infrastructure/**', 'src/db/**'],
    depends_on: ['interface_adapters', 'use_cases', 'entities'],
    description: 'Frameworks and drivers',
  },
};

const DDD_TEMPLATE: LayersRecord = {
  domain: {
    patterns: ['src/domain/**'],
    depends_on: [],
    description: 'Domain model and logic',
  },
  application: {
    patterns: ['src/application/**'],
    depends_on: ['domain'],
    description: 'Application services',
  },
  infrastructure: {
    patterns: ['src/infrastructure/**'],
    depends_on: ['domain', 'application'],
    description: 'Infrastructure implementations',
  },
  interfaces: {
    patterns: ['src/interfaces/**', 'src/api/**'],
    depends_on: ['application', 'domain'],
    description: 'User interfaces and API',
  },
};

const STARTER_TEMPLATE: LayersRecord = {
  components: {
    patterns: ['src/components/**', 'src/ui/**'],
    depends_on: ['lib'],
    description: 'UI components and visual elements',
  },
  lib: {
    patterns: ['src/lib/**', 'src/utils/**', 'src/helpers/**'],
    depends_on: [],
    description: 'Shared utilities and helper functions',
  },
  services: {
    patterns: ['src/services/**', 'src/api/**'],
    depends_on: ['lib'],
    description: 'API calls and external service integrations',
  },
};

const MONOREPO_TEMPLATE: LayersRecord = {
  apps: {
    patterns: ['apps/**', 'packages/app-*/**'],
    depends_on: ['packages', 'shared'],
    description: 'Application packages',
  },
  packages: {
    patterns: ['packages/**', 'libs/**'],
    depends_on: ['shared'],
    description: 'Reusable library packages',
  },
  shared: {
    patterns: ['shared/**', 'packages/shared/**', 'packages/common/**'],
    depends_on: [],
    description: 'Shared utilities and configurations',
  },
};

const SERVERLESS_TEMPLATE: LayersRecord = {
  functions: {
    patterns: ['src/functions/**', 'src/handlers/**', 'src/lambdas/**'],
    depends_on: ['services', 'shared'],
    description: 'Serverless function handlers',
  },
  services: {
    patterns: ['src/services/**', 'src/business/**'],
    depends_on: ['shared'],
    description: 'Business logic shared across functions',
  },
  shared: {
    patterns: ['src/shared/**', 'src/utils/**', 'src/lib/**'],
    depends_on: [],
    description: 'Shared utilities and configurations',
  },
};

const NX_WORKSPACE_TEMPLATE: LayersRecord = {
  apps: {
    patterns: ['apps/**'],
    depends_on: ['feature-libs', 'shared-libs'],
    description: 'Deployable applications',
  },
  'feature-libs': {
    patterns: ['libs/feature-*/**', 'libs/*/feature-*/**'],
    depends_on: ['data-access-libs', 'ui-libs', 'shared-libs'],
    description: 'Feature libraries',
  },
  'data-access-libs': {
    patterns: ['libs/data-access-*/**', 'libs/*/data-access-*/**'],
    depends_on: ['shared-libs'],
    description: 'Data access libraries',
  },
  'ui-libs': {
    patterns: ['libs/ui-*/**', 'libs/*/ui-*/**'],
    depends_on: ['shared-libs'],
    description: 'UI component libraries',
  },
  'shared-libs': {
    patterns: ['libs/shared/**', 'libs/util-*/**', 'libs/*/util-*/**'],
    depends_on: [],
    description: 'Shared utilities and configurations',
  },
};

const TEMPLATE_DEFAULTS: Record<ArchitectureTemplate, LayersRecord> = {
  starter: STARTER_TEMPLATE,
  layered: LAYERED_TEMPLATE,
  hexagonal: HEXAGONAL_TEMPLATE,
  clean: CLEAN_TEMPLATE,
  ddd: DDD_TEMPLATE,
  monorepo: MONOREPO_TEMPLATE,
  serverless: SERVERLESS_TEMPLATE,
  'nx-workspace': NX_WORKSPACE_TEMPLATE,
  custom: {},
};

export function getTemplateDefaults(template: ArchitectureTemplate): LayersRecord {
  return { ...TEMPLATE_DEFAULTS[template] };
}

export function mergeWithTemplate(definition: ArchitectureDefinition): ArchitectureDefinition {
  const templateLayers = getTemplateDefaults(definition.template);
  const hasUserLayers = Object.keys(definition.layers).length > 0;

  return {
    ...definition,
    layers: hasUserLayers ? definition.layers : templateLayers,
    options: definition.options ?? getDefaultOptions(),
  };
}

export function createDefinitionFromTemplate(
  template: ArchitectureTemplate
): ArchitectureDefinition {
  return {
    schema_version: '0.1.0',
    template,
    layers: getTemplateDefaults(template),
    rules: [],
    options: getDefaultOptions(),
  };
}
