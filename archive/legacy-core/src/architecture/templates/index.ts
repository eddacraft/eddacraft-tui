import { readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import YAML from 'yaml';
import { z } from 'zod';
import type { ArchitectureTemplate, LayerDefinition } from '../definition-schema.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const TemplateLayerSchema = z.object({
  patterns: z.array(z.string()),
  depends_on: z.array(z.string()).default([]),
  description: z.string().optional(),
});

const TemplateFileSchema = z.object({
  name: z.string(),
  description: z.string(),
  layers: z.record(z.string(), TemplateLayerSchema),
});

export type TemplateFile = z.infer<typeof TemplateFileSchema>;

export interface LoadedTemplate {
  name: ArchitectureTemplate;
  description: string;
  layers: Record<string, LayerDefinition>;
}

const TEMPLATE_FILES: Record<Exclude<ArchitectureTemplate, 'custom'>, string> = {
  starter: 'starter.yaml',
  layered: 'layered.yaml',
  hexagonal: 'hexagonal.yaml',
  clean: 'clean.yaml',
  ddd: 'ddd.yaml',
  monorepo: 'monorepo.yaml',
  serverless: 'serverless.yaml',
  'nx-workspace': 'nx-workspace.yaml',
};

export class TemplateLoader {
  private cache: Map<string, LoadedTemplate> = new Map();
  private templatesDir: string;

  constructor(templatesDir?: string) {
    this.templatesDir = templatesDir ?? __dirname;
  }

  async list(): Promise<ArchitectureTemplate[]> {
    return [
      'starter',
      'layered',
      'hexagonal',
      'clean',
      'ddd',
      'monorepo',
      'serverless',
      'nx-workspace',
      'custom',
    ];
  }

  async get(name: ArchitectureTemplate): Promise<LoadedTemplate> {
    if (name === 'custom') {
      return {
        name: 'custom',
        description: 'Empty template for custom architecture definitions',
        layers: {},
      };
    }

    const cached = this.cache.get(name);
    if (cached) {
      return cached;
    }

    const filename = TEMPLATE_FILES[name];
    if (!filename) {
      throw new Error(`Unknown template: ${name}`);
    }

    const filePath = join(this.templatesDir, filename);
    const content = await readFile(filePath, 'utf-8');
    const parsed = YAML.parse(content);

    const validated = TemplateFileSchema.safeParse(parsed);
    if (!validated.success) {
      throw new Error(`Invalid template file ${filename}: ${validated.error.message}`);
    }

    const template: LoadedTemplate = {
      name,
      description: validated.data.description,
      layers: validated.data.layers,
    };

    this.cache.set(name, template);
    return template;
  }

  async validate(name: ArchitectureTemplate): Promise<{ valid: boolean; errors: string[] }> {
    try {
      const template = await this.get(name);
      const errors: string[] = [];

      for (const [layerName, layer] of Object.entries(template.layers)) {
        for (const dep of layer.depends_on) {
          if (!template.layers[dep]) {
            errors.push(`Layer "${layerName}" depends on unknown layer "${dep}"`);
          }
        }

        if (layer.patterns.length === 0) {
          errors.push(`Layer "${layerName}" has no patterns defined`);
        }
      }

      return { valid: errors.length === 0, errors };
    } catch (err) {
      return {
        valid: false,
        errors: [err instanceof Error ? err.message : 'Unknown error'],
      };
    }
  }

  async getAll(): Promise<LoadedTemplate[]> {
    const names = await this.list();
    return Promise.all(names.map((name) => this.get(name)));
  }

  clearCache(): void {
    this.cache.clear();
  }
}

let defaultLoader: TemplateLoader | null = null;

export function getTemplateLoader(): TemplateLoader {
  if (!defaultLoader) {
    defaultLoader = new TemplateLoader();
  }
  return defaultLoader;
}

export async function listTemplates(): Promise<ArchitectureTemplate[]> {
  return getTemplateLoader().list();
}

export async function getTemplate(name: ArchitectureTemplate): Promise<LoadedTemplate> {
  return getTemplateLoader().get(name);
}

export async function validateTemplate(
  name: ArchitectureTemplate
): Promise<{ valid: boolean; errors: string[] }> {
  return getTemplateLoader().validate(name);
}
