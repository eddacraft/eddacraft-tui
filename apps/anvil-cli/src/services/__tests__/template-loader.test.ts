import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  TemplateLoader,
  TemplateLoadError,
  TemplateMetadataSchema,
  TemplateVariableSchema,
  createTemplateLoader,
  getDefaultTemplatesDir,
  type TemplateMetadata,
} from '../template-loader.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

const TEST_DIR = join(process.cwd(), 'tmp-template-loader-test');

function createTestTemplate(
  id: string,
  options: {
    name?: string;
    description?: string;
    category?: TemplateMetadata['category'];
    tags?: string[];
    variables?: Array<{
      name: string;
      description: string;
      default?: string;
      required?: boolean;
    }>;
    content?: string;
  } = {}
): string {
  const {
    name = `Test ${id}`,
    description = `Description for ${id}`,
    category = 'api',
    tags = ['test'],
    variables = [],
    content = `# ${name}\n\nTemplate content for {{ project_name }}`,
  } = options;

  let frontmatter = `---
id: ${id}
name: ${name}
description: ${description}
category: ${category}
tags: [${tags.join(', ')}]`;

  if (variables.length > 0) {
    frontmatter += '\nvariables:';
    for (const v of variables) {
      frontmatter += `\n  - name: ${v.name}`;
      frontmatter += `\n    description: ${v.description}`;
      if (v.default !== undefined) {
        frontmatter += `\n    default: ${v.default}`;
      }
      if (v.required !== undefined) {
        frontmatter += `\n    required: ${v.required}`;
      }
    }
  }

  frontmatter += '\n---\n\n';

  return frontmatter + content;
}

describe('TemplateLoader', () => {
  beforeEach(() => {
    mkdirSync(TEST_DIR, { recursive: true });
  });

  afterEach(async () => {
    await safeCleanup(TEST_DIR);
  });

  describe('loadTemplates', () => {
    it('loads all templates from directory', async () => {
      writeFileSync(join(TEST_DIR, 'template-a.md'), createTestTemplate('template-a'));
      writeFileSync(join(TEST_DIR, 'template-b.md'), createTestTemplate('template-b'));
      writeFileSync(join(TEST_DIR, 'template-c.md'), createTestTemplate('template-c'));

      const loader = new TemplateLoader(TEST_DIR);
      const templates = await loader.loadTemplates();

      expect(templates).toHaveLength(3);
      expect(loader.getTemplateCount()).toBe(3);
    });

    it('returns empty array for non-existent directory', async () => {
      const loader = new TemplateLoader('/non/existent/path');
      const templates = await loader.loadTemplates();

      expect(templates).toHaveLength(0);
    });

    it('ignores non-markdown files', async () => {
      writeFileSync(join(TEST_DIR, 'template.md'), createTestTemplate('template'));
      writeFileSync(join(TEST_DIR, 'readme.txt'), 'not a template');
      writeFileSync(join(TEST_DIR, 'config.json'), '{}');

      const loader = new TemplateLoader(TEST_DIR);
      const templates = await loader.loadTemplates();

      expect(templates).toHaveLength(1);
    });

    it('sets loaded flag after loading', async () => {
      writeFileSync(join(TEST_DIR, 'template.md'), createTestTemplate('template'));

      const loader = new TemplateLoader(TEST_DIR);
      expect(loader.isLoaded()).toBe(false);

      await loader.loadTemplates();
      expect(loader.isLoaded()).toBe(true);
    });
  });

  describe('loadTemplate', () => {
    it('parses template with frontmatter', async () => {
      const templateContent = createTestTemplate('test-template', {
        name: 'Test Template',
        description: 'A test template',
        category: 'authentication',
        tags: ['auth', 'security'],
      });
      const filePath = join(TEST_DIR, 'test.md');
      writeFileSync(filePath, templateContent);

      const loader = new TemplateLoader(TEST_DIR);
      const template = await loader.loadTemplate(filePath);

      expect(template.metadata.id).toBe('test-template');
      expect(template.metadata.name).toBe('Test Template');
      expect(template.metadata.description).toBe('A test template');
      expect(template.metadata.category).toBe('authentication');
      expect(template.metadata.tags).toContain('auth');
      expect(template.metadata.tags).toContain('security');
    });

    it('parses template variables', async () => {
      const templateContent = createTestTemplate('var-template', {
        variables: [
          { name: 'project_name', description: 'Project name', required: true },
          { name: 'author', description: 'Author name', default: 'Anonymous', required: false },
        ],
      });
      const filePath = join(TEST_DIR, 'var-test.md');
      writeFileSync(filePath, templateContent);

      const loader = new TemplateLoader(TEST_DIR);
      const template = await loader.loadTemplate(filePath);

      expect(template.metadata.variables).toHaveLength(2);
      expect(template.metadata.variables[0].name).toBe('project_name');
      expect(template.metadata.variables[0].required).toBe(true);
      expect(template.metadata.variables[1].name).toBe('author');
      expect(template.metadata.variables[1].default).toBe('Anonymous');
    });

    it('uses filename as id when id missing', async () => {
      const content = `---
name: No ID Template
description: Template without explicit ID
category: api
---

Content here`;
      const filePath = join(TEST_DIR, 'my-template.md');
      writeFileSync(filePath, content);

      const loader = new TemplateLoader(TEST_DIR);
      const template = await loader.loadTemplate(filePath);

      expect(template.metadata.id).toBe('my-template');
    });

    it('throws TemplateLoadError for missing frontmatter', async () => {
      const content = '# Just a heading\n\nNo frontmatter here';
      const filePath = join(TEST_DIR, 'bad.md');
      writeFileSync(filePath, content);

      const loader = new TemplateLoader(TEST_DIR);

      await expect(loader.loadTemplate(filePath)).rejects.toThrow(TemplateLoadError);
    });

    it('throws TemplateLoadError for invalid metadata', async () => {
      const content = `---
name: Invalid Template
category: invalid-category
---

Content`;
      const filePath = join(TEST_DIR, 'invalid.md');
      writeFileSync(filePath, content);

      const loader = new TemplateLoader(TEST_DIR);

      await expect(loader.loadTemplate(filePath)).rejects.toThrow(TemplateLoadError);
    });
  });

  describe('getTemplate', () => {
    it('returns template by id', async () => {
      writeFileSync(join(TEST_DIR, 'target.md'), createTestTemplate('target-id'));
      writeFileSync(join(TEST_DIR, 'other.md'), createTestTemplate('other-id'));

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const template = loader.getTemplate('target-id');
      expect(template).toBeDefined();
      expect(template?.metadata.id).toBe('target-id');
    });

    it('returns undefined for non-existent id', async () => {
      writeFileSync(join(TEST_DIR, 'template.md'), createTestTemplate('existing'));

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const template = loader.getTemplate('non-existent');
      expect(template).toBeUndefined();
    });
  });

  describe('getTemplatesByCategory', () => {
    it('filters templates by category', async () => {
      writeFileSync(
        join(TEST_DIR, 'auth1.md'),
        createTestTemplate('auth1', { category: 'authentication' })
      );
      writeFileSync(
        join(TEST_DIR, 'auth2.md'),
        createTestTemplate('auth2', { category: 'authentication' })
      );
      writeFileSync(join(TEST_DIR, 'api1.md'), createTestTemplate('api1', { category: 'api' }));

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const authTemplates = loader.getTemplatesByCategory('authentication');
      expect(authTemplates).toHaveLength(2);
      expect(authTemplates.every((t) => t.metadata.category === 'authentication')).toBe(true);
    });
  });

  describe('searchTemplates', () => {
    it('searches by name', async () => {
      writeFileSync(
        join(TEST_DIR, 'jwt.md'),
        createTestTemplate('jwt', { name: 'JWT Authentication' })
      );
      writeFileSync(
        join(TEST_DIR, 'oauth.md'),
        createTestTemplate('oauth', { name: 'OAuth Integration' })
      );

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const results = loader.searchTemplates('JWT');
      expect(results).toHaveLength(1);
      expect(results[0].metadata.id).toBe('jwt');
    });

    it('searches by description', async () => {
      writeFileSync(
        join(TEST_DIR, 'a.md'),
        createTestTemplate('a', { description: 'Implements secure login flow' })
      );
      writeFileSync(
        join(TEST_DIR, 'b.md'),
        createTestTemplate('b', { description: 'Database migrations' })
      );

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const results = loader.searchTemplates('secure');
      expect(results).toHaveLength(1);
      expect(results[0].metadata.id).toBe('a');
    });

    it('searches by tags', async () => {
      writeFileSync(
        join(TEST_DIR, 'tagged.md'),
        createTestTemplate('tagged', { tags: ['security', 'encryption'] })
      );
      writeFileSync(
        join(TEST_DIR, 'other.md'),
        createTestTemplate('other', { tags: ['database'] })
      );

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const results = loader.searchTemplates('encryption');
      expect(results).toHaveLength(1);
      expect(results[0].metadata.id).toBe('tagged');
    });

    it('is case-insensitive', async () => {
      writeFileSync(
        join(TEST_DIR, 'auth.md'),
        createTestTemplate('auth', { name: 'Authentication' })
      );

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      expect(loader.searchTemplates('AUTHENTICATION')).toHaveLength(1);
      expect(loader.searchTemplates('authentication')).toHaveLength(1);
    });
  });

  describe('renderTemplate', () => {
    it('substitutes variables in content', async () => {
      const templateContent = createTestTemplate('render-test', {
        variables: [{ name: 'project_name', description: 'Project name', required: true }],
        content: '# {{ project_name }}\n\nWelcome to {{ project_name }}!',
      });
      writeFileSync(join(TEST_DIR, 'render.md'), templateContent);

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const template = loader.getTemplate('render-test')!;
      const rendered = loader.renderTemplate(template, { project_name: 'MyApp' });

      expect(rendered.content).toContain('# MyApp');
      expect(rendered.content).toContain('Welcome to MyApp!');
      expect(rendered.content).not.toContain('{{ project_name }}');
    });

    it('uses default values for missing optional variables', async () => {
      const templateContent = createTestTemplate('defaults', {
        variables: [
          { name: 'name', description: 'Name', required: true },
          { name: 'version', description: 'Version', default: '1.0.0', required: false },
        ],
        content: '{{ name }} v{{ version }}',
      });
      writeFileSync(join(TEST_DIR, 'defaults.md'), templateContent);

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const template = loader.getTemplate('defaults')!;
      const rendered = loader.renderTemplate(template, { name: 'TestApp' });

      expect(rendered.content).toBe('TestApp v1.0.0');
    });

    it('throws error for missing required variables', async () => {
      const templateContent = createTestTemplate('required', {
        variables: [{ name: 'required_var', description: 'Required', required: true }],
      });
      writeFileSync(join(TEST_DIR, 'required.md'), templateContent);

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const template = loader.getTemplate('required')!;

      expect(() => loader.renderTemplate(template, {})).toThrow('Missing required variable');
    });

    it('returns variables in result', async () => {
      const templateContent = createTestTemplate('vars-result', {
        variables: [{ name: 'a', description: 'A', required: true }],
      });
      writeFileSync(join(TEST_DIR, 'vars.md'), templateContent);

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const template = loader.getTemplate('vars-result')!;
      const rendered = loader.renderTemplate(template, { a: 'value' });

      expect(rendered.variables).toEqual({ a: 'value' });
      expect(rendered.template).toBe(template);
    });
  });

  describe('getCategories', () => {
    it('returns unique categories', async () => {
      writeFileSync(join(TEST_DIR, 'a.md'), createTestTemplate('a', { category: 'api' }));
      writeFileSync(join(TEST_DIR, 'b.md'), createTestTemplate('b', { category: 'api' }));
      writeFileSync(
        join(TEST_DIR, 'c.md'),
        createTestTemplate('c', { category: 'authentication' })
      );
      writeFileSync(join(TEST_DIR, 'd.md'), createTestTemplate('d', { category: 'database' }));

      const loader = new TemplateLoader(TEST_DIR);
      await loader.loadTemplates();

      const categories = loader.getCategories();
      expect(categories).toHaveLength(3);
      expect(categories).toContain('api');
      expect(categories).toContain('authentication');
      expect(categories).toContain('database');
    });
  });
});

describe('Schema validation', () => {
  describe('TemplateVariableSchema', () => {
    it('validates valid variable', () => {
      const result = TemplateVariableSchema.safeParse({
        name: 'test_var',
        description: 'A test variable',
        default: 'default_value',
        required: true,
        type: 'string',
      });

      expect(result.success).toBe(true);
    });

    it('requires name and description', () => {
      const result = TemplateVariableSchema.safeParse({});
      expect(result.success).toBe(false);
    });

    it('defaults type to string', () => {
      const result = TemplateVariableSchema.parse({
        name: 'test',
        description: 'Test',
      });

      expect(result.type).toBe('string');
    });

    it('defaults required to true', () => {
      const result = TemplateVariableSchema.parse({
        name: 'test',
        description: 'Test',
      });

      expect(result.required).toBe(true);
    });
  });

  describe('TemplateMetadataSchema', () => {
    it('validates valid metadata', () => {
      const result = TemplateMetadataSchema.safeParse({
        id: 'test-template',
        name: 'Test Template',
        description: 'A test template',
        category: 'api',
      });

      expect(result.success).toBe(true);
    });

    it('rejects invalid category', () => {
      const result = TemplateMetadataSchema.safeParse({
        id: 'test',
        name: 'Test',
        description: 'Desc',
        category: 'invalid-category',
      });

      expect(result.success).toBe(false);
    });

    it('defaults tags to empty array', () => {
      const result = TemplateMetadataSchema.parse({
        id: 'test',
        name: 'Test',
        description: 'Desc',
        category: 'api',
      });

      expect(result.tags).toEqual([]);
    });

    it('defaults variables to empty array', () => {
      const result = TemplateMetadataSchema.parse({
        id: 'test',
        name: 'Test',
        description: 'Desc',
        category: 'api',
      });

      expect(result.variables).toEqual([]);
    });

    it('transforms array description to joined string', () => {
      const result = TemplateMetadataSchema.parse({
        id: 'test',
        name: 'Test',
        description: ['Line one', 'line two', 'line three'],
        category: 'api',
      });

      expect(result.description).toBe('Line one line two line three');
    });

    it('rejects empty array descriptions', () => {
      const result = TemplateMetadataSchema.safeParse({
        id: 'test',
        name: 'Test',
        description: [],
        category: 'api',
      });

      expect(result.success).toBe(false);
    });
  });

  describe('TemplateVariableSchema numeric defaults', () => {
    it('transforms numeric default to string', () => {
      const result = TemplateVariableSchema.parse({
        name: 'port',
        description: 'Port number',
        default: 3000,
      });

      expect(result.default).toBe('3000');
      expect(typeof result.default).toBe('string');
    });

    it('transforms array description to joined string', () => {
      const result = TemplateVariableSchema.parse({
        name: 'test_var',
        description: ['A variable that does', 'something important'],
      });

      expect(result.description).toBe('A variable that does something important');
    });

    it('keeps string default as string', () => {
      const result = TemplateVariableSchema.parse({
        name: 'host',
        description: 'Hostname',
        default: 'localhost',
      });

      expect(result.default).toBe('localhost');
    });

    it('allows undefined default when field is omitted', () => {
      const result = TemplateVariableSchema.parse({
        name: 'required_field',
        description: 'A required field',
      });

      expect(result.default).toBeUndefined();
    });
  });
});

describe('YAML formatting variations', () => {
  beforeEach(() => {
    mkdirSync(TEST_DIR, { recursive: true });
  });

  afterEach(async () => {
    await safeCleanup(TEST_DIR);
  });

  it('parses multi-line YAML description (indented block scalar)', async () => {
    const content = `---
id: multiline-test
name: Multiline Test
description:
  This is a multi-line description that spans
  multiple lines in the YAML frontmatter
category: api
---

Content here`;
    const filePath = join(TEST_DIR, 'multiline.md');
    writeFileSync(filePath, content);

    const loader = new TemplateLoader(TEST_DIR);
    const template = await loader.loadTemplate(filePath);

    expect(template.metadata.description).toBe(
      'This is a multi-line description that spans multiple lines in the YAML frontmatter'
    );
  });

  it('parses numeric default values in variables', async () => {
    const content = `---
id: numeric-defaults
name: Numeric Defaults Test
description: Test template with numeric defaults
category: api
variables:
  - name: port
    description: Port number
    default: 8080
    required: false
  - name: timeout
    description: Timeout in ms
    default: 5000
    required: false
---

Port: {{ port }}, Timeout: {{ timeout }}`;
    const filePath = join(TEST_DIR, 'numeric.md');
    writeFileSync(filePath, content);

    const loader = new TemplateLoader(TEST_DIR);
    const template = await loader.loadTemplate(filePath);

    expect(template.metadata.variables[0].default).toBe('8080');
    expect(template.metadata.variables[1].default).toBe('5000');
    expect(typeof template.metadata.variables[0].default).toBe('string');
  });

  it('handles variables with and without defaults', async () => {
    const content = `---
id: mixed-defaults
name: Mixed Defaults Test
description: Test with mixed default presence
category: api
variables:
  - name: required_var
    description: A required variable
    required: true
  - name: optional_with_default
    description: Optional with default
    default: fallback
    required: false
---

Content`;
    const filePath = join(TEST_DIR, 'mixed.md');
    writeFileSync(filePath, content);

    const loader = new TemplateLoader(TEST_DIR);
    const template = await loader.loadTemplate(filePath);

    expect(template.metadata.variables[0].default).toBeUndefined();
    expect(template.metadata.variables[1].default).toBe('fallback');
  });
});

describe('createTemplateLoader', () => {
  it('creates loader with default templates directory', () => {
    const loader = createTemplateLoader();
    expect(loader).toBeInstanceOf(TemplateLoader);
  });
});

describe('getDefaultTemplatesDir', () => {
  it('returns path to templates directory', () => {
    const dir = getDefaultTemplatesDir();
    expect(dir).toContain('templates');
  });
});
