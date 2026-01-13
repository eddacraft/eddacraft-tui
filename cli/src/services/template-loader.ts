/**
 * Template loader service for plan templates
 * @module cli/services/template-loader
 */

import { readFile, readdir } from 'node:fs/promises';
import { join, basename } from 'node:path';
import { existsSync } from 'node:fs';
import { z } from 'zod';

// Handle YAML multi-line strings (parsed as arrays) and single-line strings
// Rejects empty arrays to catch parser issues with indented block scalars
const YamlStringSchema = z
  .union([z.string(), z.array(z.string()).min(1)])
  .transform((val) => (Array.isArray(val) ? val.join(' ') : val));

// Handle YAML numbers and strings for default values
const YamlStringOrNumberSchema = z
  .union([z.string(), z.number()])
  .transform((val) => String(val))
  .optional();

export const TemplateVariableSchema = z.object({
  name: z.string(),
  description: YamlStringSchema,
  default: YamlStringOrNumberSchema,
  required: z.boolean().default(true),
  type: z.enum(['string', 'boolean', 'choice']).default('string'),
  choices: z.array(z.string()).optional(),
});

export type TemplateVariable = z.infer<typeof TemplateVariableSchema>;

export const TemplateMetadataSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: YamlStringSchema,
  category: z.enum([
    'authentication',
    'api',
    'database',
    'frontend',
    'infrastructure',
    'testing',
    'integration',
  ]),
  tags: z.array(z.string()).default([]),
  variables: z.array(TemplateVariableSchema).default([]),
});

export type TemplateMetadata = z.infer<typeof TemplateMetadataSchema>;

export interface Template {
  metadata: TemplateMetadata;
  content: string;
  filePath: string;
}

export interface RenderedTemplate {
  template: Template;
  content: string;
  variables: Record<string, string>;
}

export class TemplateLoadError extends Error {
  constructor(
    message: string,
    public readonly filePath?: string,
    public readonly cause?: Error
  ) {
    super(message);
    this.name = 'TemplateLoadError';
  }
}

function parseFrontmatter(content: string): { metadata: Record<string, unknown>; body: string } {
  const frontmatterRegex = /^---\n([\s\S]*?)\n---\n([\s\S]*)$/;
  const match = content.match(frontmatterRegex);

  if (!match) {
    throw new TemplateLoadError('Template missing frontmatter (---...---)');
  }

  const [, frontmatter, body] = match;

  const metadata: Record<string, unknown> = {};
  let currentArray: unknown[] | null = null;
  let inVariables = false;
  let currentVariable: Record<string, unknown> | null = null;
  const variables: Record<string, unknown>[] = [];

  for (const line of frontmatter.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;

    const keyMatch = line.match(/^(\w+):\s*(.*)$/);
    if (keyMatch) {
      const [, key, value] = keyMatch;

      if (currentVariable && inVariables) {
        variables.push(currentVariable);
        currentVariable = null;
      }

      if (key === 'variables') {
        inVariables = true;
        currentArray = null;
        continue;
      }

      inVariables = false;

      if (value) {
        metadata[key] = parseValue(value);
      } else {
        currentArray = [];
        metadata[key] = currentArray;
      }
      continue;
    }

    if (inVariables && line.match(/^\s+-\s+name:/)) {
      if (currentVariable) {
        variables.push(currentVariable);
      }
      const nameMatch = line.match(/name:\s*(.+)$/);
      currentVariable = { name: nameMatch?.[1]?.trim() || '' };
      continue;
    }

    if (inVariables && currentVariable && line.match(/^\s+\w+:/)) {
      const propMatch = line.match(/^\s+(\w+):\s*(.*)$/);
      if (propMatch) {
        const [, propKey, propValue] = propMatch;
        currentVariable[propKey] = parseValue(propValue);
      }
      continue;
    }

    if (line.match(/^\s+-\s+/)) {
      const itemValue = line.replace(/^\s+-\s+/, '').trim();
      if (currentArray) {
        currentArray.push(parseValue(itemValue));
      }
    } else if (currentArray && line.match(/^\s+\S/)) {
      // Handle indented block scalar (text without leading -)
      currentArray.push(trimmed);
    }
  }

  if (currentVariable) {
    variables.push(currentVariable);
  }

  if (variables.length > 0) {
    metadata.variables = variables;
  }

  return { metadata, body: body.trim() };
}

function parseValue(value: string): unknown {
  const trimmed = value.trim();

  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;

  const num = Number(trimmed);
  if (!isNaN(num) && trimmed !== '') return num;

  if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
    return trimmed
      .slice(1, -1)
      .split(',')
      .map((s) => s.trim().replace(/^['"]|['"]$/g, ''));
  }

  return trimmed.replace(/^['"]|['"]$/g, '');
}

export class TemplateLoader {
  private templates: Map<string, Template> = new Map();
  private loaded = false;

  constructor(private readonly templatesDir: string) {}

  async loadTemplates(): Promise<Template[]> {
    if (!existsSync(this.templatesDir)) {
      return [];
    }

    const files = await readdir(this.templatesDir);
    const templateFiles = files.filter((f) => f.endsWith('.md'));

    const templates: Template[] = [];

    for (const file of templateFiles) {
      try {
        const template = await this.loadTemplate(join(this.templatesDir, file));
        templates.push(template);
        this.templates.set(template.metadata.id, template);
      } catch (error) {
        console.error(`Warning: Failed to load template ${file}:`, error);
      }
    }

    this.loaded = true;
    return templates;
  }

  async loadTemplate(filePath: string): Promise<Template> {
    const content = await readFile(filePath, 'utf-8');

    try {
      const { metadata: rawMetadata, body } = parseFrontmatter(content);

      if (!rawMetadata.id) {
        rawMetadata.id = basename(filePath, '.md');
      }

      const metadata = TemplateMetadataSchema.parse(rawMetadata);

      return {
        metadata,
        content: body,
        filePath,
      };
    } catch (error) {
      if (error instanceof z.ZodError) {
        throw new TemplateLoadError(
          `Invalid template metadata: ${error.issues.map((issue) => issue.message).join(', ')}`,
          filePath,
          error
        );
      }
      throw new TemplateLoadError(
        `Failed to parse template: ${error instanceof Error ? error.message : String(error)}`,
        filePath,
        error instanceof Error ? error : undefined
      );
    }
  }

  getTemplate(id: string): Template | undefined {
    return this.templates.get(id);
  }

  getAllTemplates(): Template[] {
    return Array.from(this.templates.values());
  }

  getTemplatesByCategory(category: TemplateMetadata['category']): Template[] {
    return this.getAllTemplates().filter((t) => t.metadata.category === category);
  }

  searchTemplates(query: string): Template[] {
    const lowerQuery = query.toLowerCase();
    return this.getAllTemplates().filter(
      (t) =>
        t.metadata.name.toLowerCase().includes(lowerQuery) ||
        t.metadata.description.toLowerCase().includes(lowerQuery) ||
        t.metadata.tags.some((tag) => tag.toLowerCase().includes(lowerQuery))
    );
  }

  renderTemplate(template: Template, variables: Record<string, string>): RenderedTemplate {
    let content = template.content;

    for (const variable of template.metadata.variables) {
      const value = variables[variable.name] ?? variable.default;

      if (variable.required && !value) {
        throw new Error(`Missing required variable: ${variable.name}`);
      }
    }

    for (const [key, value] of Object.entries(variables)) {
      const placeholder = new RegExp(`\\{\\{\\s*${key}\\s*\\}\\}`, 'g');
      content = content.replace(placeholder, value);
    }

    for (const variable of template.metadata.variables) {
      if (variable.default) {
        const placeholder = new RegExp(`\\{\\{\\s*${variable.name}\\s*\\}\\}`, 'g');
        content = content.replace(placeholder, variable.default);
      }
    }

    return {
      template,
      content,
      variables,
    };
  }

  isLoaded(): boolean {
    return this.loaded;
  }

  getTemplateCount(): number {
    return this.templates.size;
  }

  getCategories(): TemplateMetadata['category'][] {
    const categories = new Set<TemplateMetadata['category']>();
    for (const template of this.templates.values()) {
      categories.add(template.metadata.category);
    }
    return Array.from(categories);
  }
}

export function getDefaultTemplatesDir(): string {
  return join(import.meta.dirname, '..', '..', 'templates');
}

export function createTemplateLoader(): TemplateLoader {
  return new TemplateLoader(getDefaultTemplatesDir());
}
