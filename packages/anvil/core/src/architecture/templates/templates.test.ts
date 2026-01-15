import { describe, it, expect, beforeEach } from 'vitest';
import { TemplateLoader, listTemplates, getTemplate, validateTemplate } from './index.js';

describe('TemplateLoader', () => {
  let loader: TemplateLoader;

  beforeEach(() => {
    loader = new TemplateLoader();
    loader.clearCache();
  });

  describe('list', () => {
    it('returns all available templates', async () => {
      const templates = await loader.list();
      expect(templates).toContain('starter');
      expect(templates).toContain('layered');
      expect(templates).toContain('hexagonal');
      expect(templates).toContain('clean');
      expect(templates).toContain('ddd');
      expect(templates).toContain('monorepo');
      expect(templates).toContain('serverless');
      expect(templates).toContain('nx-workspace');
      expect(templates).toContain('custom');
      expect(templates).toHaveLength(9);
    });
  });

  describe('get', () => {
    it('loads layered template', async () => {
      const template = await loader.get('layered');
      expect(template.name).toBe('layered');
      expect(template.description).toContain('3-tier');
      expect(template.layers.presentation).toBeDefined();
      expect(template.layers.business).toBeDefined();
      expect(template.layers.data).toBeDefined();
      expect(template.layers.shared).toBeDefined();
    });

    it('loads hexagonal template', async () => {
      const template = await loader.get('hexagonal');
      expect(template.name).toBe('hexagonal');
      expect(template.layers.core).toBeDefined();
      expect(template.layers.ports).toBeDefined();
      expect(template.layers.adapters).toBeDefined();
    });

    it('loads clean template', async () => {
      const template = await loader.get('clean');
      expect(template.name).toBe('clean');
      expect(template.layers.entities).toBeDefined();
      expect(template.layers.use_cases).toBeDefined();
      expect(template.layers.interface_adapters).toBeDefined();
      expect(template.layers.frameworks).toBeDefined();
    });

    it('loads ddd template', async () => {
      const template = await loader.get('ddd');
      expect(template.name).toBe('ddd');
      expect(template.layers.domain).toBeDefined();
      expect(template.layers.application).toBeDefined();
      expect(template.layers.infrastructure).toBeDefined();
      expect(template.layers.interfaces).toBeDefined();
    });

    it('loads starter template', async () => {
      const template = await loader.get('starter');
      expect(template.name).toBe('starter');
      expect(template.layers.components).toBeDefined();
      expect(template.layers.lib).toBeDefined();
      expect(template.layers.services).toBeDefined();
    });

    it('loads monorepo template', async () => {
      const template = await loader.get('monorepo');
      expect(template.name).toBe('monorepo');
      expect(template.layers.packages).toBeDefined();
      expect(template.layers.shared).toBeDefined();
    });

    it('loads serverless template', async () => {
      const template = await loader.get('serverless');
      expect(template.name).toBe('serverless');
      expect(template.layers.functions).toBeDefined();
      expect(template.layers.services).toBeDefined();
      expect(template.layers.shared).toBeDefined();
    });

    it('loads nx-workspace template', async () => {
      const template = await loader.get('nx-workspace');
      expect(template.name).toBe('nx-workspace');
      expect(template.layers.apps).toBeDefined();
      expect(template.layers['feature-libs']).toBeDefined();
      expect(template.layers['data-access-libs']).toBeDefined();
      expect(template.layers['ui-libs']).toBeDefined();
      expect(template.layers['shared-libs']).toBeDefined();
    });

    it('returns empty layers for custom template', async () => {
      const template = await loader.get('custom');
      expect(template.name).toBe('custom');
      expect(template.layers).toEqual({});
    });

    it('caches loaded templates', async () => {
      const first = await loader.get('layered');
      const second = await loader.get('layered');
      expect(first).toBe(second);
    });

    it('layers have patterns and depends_on', async () => {
      const template = await loader.get('layered');

      expect(template.layers.presentation.patterns.length).toBeGreaterThan(0);
      expect(template.layers.presentation.depends_on).toContain('business');

      expect(template.layers.data.patterns.length).toBeGreaterThan(0);
      expect(template.layers.data.depends_on).toContain('shared');
    });
  });

  describe('validate', () => {
    it('validates correct templates', async () => {
      const result = await loader.validate('layered');
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('validates all built-in templates', async () => {
      const templates = [
        'starter',
        'layered',
        'hexagonal',
        'clean',
        'ddd',
        'monorepo',
        'serverless',
        'nx-workspace',
      ] as const;

      for (const name of templates) {
        const result = await loader.validate(name);
        expect(result.valid).toBe(true);
        expect(result.errors).toHaveLength(0);
      }
    });

    it('validates custom template', async () => {
      const result = await loader.validate('custom');
      expect(result.valid).toBe(true);
    });
  });

  describe('getAll', () => {
    it('returns all templates', async () => {
      const templates = await loader.getAll();
      expect(templates).toHaveLength(9);
      expect(templates.map((t) => t.name)).toContain('starter');
      expect(templates.map((t) => t.name)).toContain('layered');
      expect(templates.map((t) => t.name)).toContain('nx-workspace');
      expect(templates.map((t) => t.name)).toContain('custom');
    });
  });

  describe('clearCache', () => {
    it('clears cached templates', async () => {
      const first = await loader.get('layered');
      loader.clearCache();
      const second = await loader.get('layered');
      expect(first).not.toBe(second);
      expect(first).toEqual(second);
    });
  });
});

describe('module exports', () => {
  it('listTemplates returns all templates', async () => {
    const templates = await listTemplates();
    expect(templates).toContain('layered');
    expect(templates).toContain('custom');
  });

  it('getTemplate loads a template', async () => {
    const template = await getTemplate('hexagonal');
    expect(template.name).toBe('hexagonal');
  });

  it('validateTemplate validates a template', async () => {
    const result = await validateTemplate('clean');
    expect(result.valid).toBe(true);
  });
});
