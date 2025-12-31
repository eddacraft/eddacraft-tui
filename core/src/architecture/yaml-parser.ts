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

const TEMPLATE_DEFAULTS: Record<ArchitectureTemplate, LayersRecord> = {
  layered: LAYERED_TEMPLATE,
  hexagonal: HEXAGONAL_TEMPLATE,
  clean: CLEAN_TEMPLATE,
  ddd: DDD_TEMPLATE,
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
