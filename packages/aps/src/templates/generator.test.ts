import { describe, expect, it } from 'vitest';
import type { TemplateBundle } from './generator.js';
import {
  generateAllTemplates,
  generateIndexTemplate,
  generateLeafTemplate,
  generateSimplePlanTemplate,
} from './generator.js';

describe('Template Generator', () => {
  describe('generateIndexTemplate', () => {
    it('should generate a valid index template', () => {
      const template = generateIndexTemplate();

      expect(template).toContain('# [Plan Title]');
      expect(template).toContain('## Overview');
      expect(template).toContain('## Modules');
      expect(template).toContain('### [module-id]');
      expect(template).toContain('**Path:**');
      expect(template).toContain('**Scope:**');
      expect(template).toContain('**Owner:**');
      expect(template).toContain('**Priority:**');
      expect(template).toContain('**Tags:**');
      expect(template).toContain('**Dependencies:**');
      expect(template).toContain('## Open Questions');
      expect(template).toContain('## Decisions');
    });

    it('should include module metadata fields', () => {
      const template = generateIndexTemplate();

      expect(template).toMatch(/\*\*Path:\*\*/);
      expect(template).toMatch(/\*\*Scope:\*\*/);
      expect(template).toMatch(/\*\*Owner:\*\*/);
      expect(template).toMatch(/\*\*Priority:\*\*/);
      expect(template).toMatch(/\*\*Tags:\*\*/);
      expect(template).toMatch(/\*\*Dependencies:\*\*/);
    });

    it('should use markdown link syntax for paths', () => {
      const template = generateIndexTemplate();

      expect(template).toMatch(/\[.*\]\(\.\/modules\/.*\.aps\.md\)/);
    });
  });

  describe('generateLeafTemplate', () => {
    it('should generate a valid leaf spec template', () => {
      const template = generateLeafTemplate();

      expect(template).toContain('# [Module Title]');
      expect(template).toContain('**Scope:**');
      expect(template).toContain('**Owner:**');
      expect(template).toContain('**Priority:**');
      expect(template).toContain('## Tasks');
      expect(template).toContain('### [SCOPE]-001:');
      expect(template).toContain('**Intent:**');
      expect(template).toContain('**Expected Outcome:**');
      expect(template).toContain('**Confidence:**');
      expect(template).toContain('**Scopes:**');
      expect(template).toContain('**Tags:**');
      expect(template).toContain('**Dependencies:**');
      expect(template).toContain('**Inputs:**');
      expect(template).toContain('## Dependencies');
      expect(template).toContain('## Notes');
    });

    it('should use task ID format SCOPE-NUMBER', () => {
      const template = generateLeafTemplate();

      expect(template).toMatch(/### \[SCOPE\]-001:/);
      expect(template).toMatch(/### \[SCOPE\]-002:/);
    });

    it('should include all task fields', () => {
      const template = generateLeafTemplate();

      expect(template).toMatch(/\*\*Intent:\*\*/);
      expect(template).toMatch(/\*\*Expected Outcome:\*\*/);
      expect(template).toMatch(/\*\*Confidence:\*\*/);
      expect(template).toMatch(/\*\*Scopes:\*\*/);
      expect(template).toMatch(/\*\*Tags:\*\*/);
      expect(template).toMatch(/\*\*Dependencies:\*\*/);
      expect(template).toMatch(/\*\*Inputs:\*\*/);
    });
  });

  describe('generateSimplePlanTemplate', () => {
    it('should generate a valid single-file plan template', () => {
      const template = generateSimplePlanTemplate();

      expect(template).toContain('# Feature:');
      expect(template).toContain('**Scope:**');
      expect(template).toContain('**Owner:**');
      expect(template).toContain('**Priority:**');
      expect(template).toContain('## Tasks');
      expect(template).toContain('### [SCOPE]-001:');
      expect(template).toContain('**Intent:**');
      expect(template).toContain('## Notes');
    });

    it('should include task dependencies example', () => {
      const template = generateSimplePlanTemplate();

      expect(template).toMatch(/\*\*Dependencies:\*\* \[SCOPE\]-001/);
    });
  });

  describe('generateAllTemplates', () => {
    it('should generate all template types', () => {
      const templates = generateAllTemplates();

      expect(templates).toHaveProperty('index');
      expect(templates).toHaveProperty('leaf');
      expect(templates).toHaveProperty('simple');
    });

    it('should return a properly typed TemplateBundle', () => {
      const templates: TemplateBundle = generateAllTemplates();

      // Type-safe access to known keys
      expect(typeof templates.index).toBe('string');
      expect(typeof templates.leaf).toBe('string');
      expect(typeof templates.simple).toBe('string');

      // TypeScript will error if trying to access unknown keys
      // @ts-expect-error - 'unknown' does not exist on type 'TemplateBundle'
      const _ = templates.unknown;
    });

    it('should return non-empty templates', () => {
      const templates = generateAllTemplates();

      expect(templates.index).toBeTruthy();
      expect(templates.leaf).toBeTruthy();
      expect(templates.simple).toBeTruthy();

      expect(templates.index.length).toBeGreaterThan(0);
      expect(templates.leaf.length).toBeGreaterThan(0);
      expect(templates.simple.length).toBeGreaterThan(0);
    });

    it('should return distinct templates', () => {
      const templates = generateAllTemplates();

      expect(templates.index).not.toEqual(templates.leaf);
      expect(templates.index).not.toEqual(templates.simple);
      expect(templates.leaf).not.toEqual(templates.simple);
    });
  });
});
