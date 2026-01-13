import { describe, expect, it } from 'vitest';
import type { TemplateBundle, TemplateVariant } from './generator.js';
import {
  generateAllTemplates,
  generateIndexTemplate,
  generateLeafTemplate,
  generateSimplePlanTemplate,
  generateActionsTemplate,
} from './generator.js';

describe('Template Generator', () => {
  describe('generateIndexTemplate', () => {
    describe('standard variant (default)', () => {
      it('should generate a valid index template', () => {
        const template = generateIndexTemplate();

        expect(template).toContain('# [Plan Title]');
        expect(template).toContain('## Problem & Success Criteria');
        expect(template).toContain('## System Map');
        expect(template).toContain('## Milestones');
        expect(template).toContain('## Modules');
        expect(template).toContain('### [module-id]');
        expect(template).toContain('**Path:**');
        expect(template).toContain('**Scope:**');
        expect(template).toContain('**Owner:**');
        expect(template).toContain('**Status:**');
        expect(template).toContain('**Priority:**');
        expect(template).toContain('**Tags:**');
        expect(template).toContain('**Dependencies:**');
        expect(template).toContain('## Decisions');
        expect(template).toContain('## Open Questions');
      });

      it('should include module metadata fields', () => {
        const template = generateIndexTemplate();

        expect(template).toMatch(/\*\*Path:\*\*/);
        expect(template).toMatch(/\*\*Scope:\*\*/);
        expect(template).toMatch(/\*\*Owner:\*\*/);
        expect(template).toMatch(/\*\*Status:\*\*/);
        expect(template).toMatch(/\*\*Priority:\*\*/);
        expect(template).toMatch(/\*\*Tags:\*\*/);
        expect(template).toMatch(/\*\*Dependencies:\*\*/);
      });

      it('should use markdown link syntax for paths', () => {
        const template = generateIndexTemplate();

        expect(template).toMatch(/\[.*\]\(\.\/modules\/.*\.aps\.md\)/);
      });

      it('should include decision IDs', () => {
        const template = generateIndexTemplate();

        expect(template).toContain('**D-001:**');
        expect(template).toContain('**D-002:**');
      });
    });

    describe('minimal variant', () => {
      it('should generate a minimal index template', () => {
        const template = generateIndexTemplate({ variant: 'minimal' });

        expect(template).toContain('# [Plan Title]');
        expect(template).toContain('## Modules');
        expect(template).toContain('**Path:**');
        expect(template).toContain('**Scope:**');
        expect(template).toContain('**Owner:**');
        // Should NOT have extra sections
        expect(template).not.toContain('## Problem');
        expect(template).not.toContain('## Milestones');
        expect(template).not.toContain('## Decisions');
      });
    });

    describe('full variant', () => {
      it('should generate a comprehensive index template', () => {
        const template = generateIndexTemplate({ variant: 'full' });

        expect(template).toContain('# APS Index');
        expect(template).toContain('## Problem & Success Criteria');
        expect(template).toContain('## Scope');
        expect(template).toContain('**In Scope:**');
        expect(template).toContain('**Out of Scope:**');
        expect(template).toContain('## System Map');
        expect(template).toContain('## Milestones');
        expect(template).toContain('## Modules');
        expect(template).toContain('## Epics');
        expect(template).toContain('## Decisions');
        expect(template).toContain('## Risks');
        expect(template).toContain('## Open Questions');
      });
    });
  });

  describe('generateLeafTemplate', () => {
    describe('standard variant (default)', () => {
      it('should generate a valid leaf spec template', () => {
        const template = generateLeafTemplate();

        expect(template).toContain('# [Module Title]');
        expect(template).toContain('**Scope:**');
        expect(template).toContain('**Owner:**');
        expect(template).toContain('**Priority:**');
        expect(template).toContain('## Purpose');
        expect(template).toContain('## In Scope / Out of Scope');
        expect(template).toContain('## Interfaces');
        expect(template).toContain('**Depends on:**');
        expect(template).toContain('**Exposes:**');
        expect(template).toContain('## Tasks');
        expect(template).toContain('### [SCOPE]-001:');
        expect(template).toContain('**Intent:**');
        expect(template).toContain('**Expected Outcome:**');
        expect(template).toContain('**Confidence:**');
        expect(template).toContain('**Link:**');
        expect(template).toContain('**Scopes:**');
        expect(template).toContain('**Tags:**');
        expect(template).toContain('**Dependencies:**');
        expect(template).toContain('**Inputs:**');
        expect(template).toContain('## Decisions');
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
        expect(template).toMatch(/\*\*Link:\*\*/);
        expect(template).toMatch(/\*\*Scopes:\*\*/);
        expect(template).toMatch(/\*\*Tags:\*\*/);
        expect(template).toMatch(/\*\*Dependencies:\*\*/);
        expect(template).toMatch(/\*\*Inputs:\*\*/);
      });

      it('should include external link placeholder', () => {
        const template = generateLeafTemplate();

        expect(template).toContain('**Link:**');
        expect(template).toContain('jira.example.com');
      });
    });

    describe('minimal variant', () => {
      it('should generate a minimal leaf template', () => {
        const template = generateLeafTemplate({ variant: 'minimal' });

        expect(template).toContain('# [Module Title]');
        expect(template).toContain('## Tasks');
        expect(template).toContain('**Intent:**');
        expect(template).toContain('**Confidence:**');
        // Should NOT have extra sections
        expect(template).not.toContain('## Purpose');
        expect(template).not.toContain('## Interfaces');
        expect(template).not.toContain('**Link:**');
      });
    });

    describe('full variant', () => {
      it('should generate a comprehensive leaf template', () => {
        const template = generateLeafTemplate({ variant: 'full' });

        expect(template).toContain('# Module APS');
        expect(template).toContain('## Purpose');
        expect(template).toContain('## In Scope / Out of Scope');
        expect(template).toContain('## Assumptions');
        expect(template).toContain('## Interfaces');
        expect(template).toContain('## Tasks');
        expect(template).toContain('## Decisions');
        expect(template).toContain('## Risks');
        expect(template).toContain('## Open Questions');
        expect(template).toContain('## Notes');
      });
    });
  });

  describe('generateSimplePlanTemplate', () => {
    describe('standard variant (default)', () => {
      it('should generate a valid single-file plan template', () => {
        const template = generateSimplePlanTemplate();

        expect(template).toContain('# Feature:');
        expect(template).toContain('**Scope:**');
        expect(template).toContain('**Owner:**');
        expect(template).toContain('**Priority:**');
        expect(template).toContain('## Purpose');
        expect(template).toContain('## Success Criteria');
        expect(template).toContain('## Tasks');
        expect(template).toContain('### [SCOPE]-001:');
        expect(template).toContain('**Intent:**');
        expect(template).toContain('**Link:**');
        expect(template).toContain('## Notes');
      });

      it('should include task dependencies example', () => {
        const template = generateSimplePlanTemplate();

        expect(template).toMatch(/\*\*Dependencies:\*\* \[SCOPE\]-001/);
      });
    });

    describe('minimal variant', () => {
      it('should generate a minimal simple template', () => {
        const template = generateSimplePlanTemplate({ variant: 'minimal' });

        expect(template).toContain('# [Feature Name]');
        expect(template).toContain('## Tasks');
        // Should NOT have extra sections
        expect(template).not.toContain('## Purpose');
        expect(template).not.toContain('## Success Criteria');
      });
    });

    describe('full variant', () => {
      it('should generate a comprehensive simple template', () => {
        const template = generateSimplePlanTemplate({ variant: 'full' });

        expect(template).toContain('# Feature:');
        expect(template).toContain('## Purpose');
        expect(template).toContain('## Success Criteria');
        expect(template).toContain('## In Scope / Out of Scope');
        expect(template).toContain('## Assumptions');
        expect(template).toContain('## Tasks');
        expect(template).toContain('## Decisions');
        expect(template).toContain('## Open Questions');
        expect(template).toContain('## Notes');
      });
    });
  });

  describe('generateActionsTemplate', () => {
    describe('standard variant (default)', () => {
      it('should generate a valid action plan template', () => {
        const template = generateActionsTemplate();

        expect(template).toContain('# Actions: [SCOPE-NNN]');
        expect(template).toContain('| Source | Work Item | Created by | Status |');
        expect(template).toContain('## Prerequisites');
        expect(template).toContain('## Actions');
        expect(template).toContain('### 1.');
        expect(template).toContain('**Purpose:**');
        expect(template).toContain('**Produces:**');
        expect(template).toContain('**Checkpoint:**');
        expect(template).toContain('**Validate:**');
        expect(template).toContain('## Completion');
      });

      it('should include action structure fields', () => {
        const template = generateActionsTemplate();

        expect(template).toMatch(/\*\*Purpose:\*\*/);
        expect(template).toMatch(/\*\*Produces:\*\*/);
        expect(template).toMatch(/\*\*Checkpoint:\*\*/);
        expect(template).toMatch(/\*\*Validate:\*\*/);
      });
    });

    describe('minimal variant', () => {
      it('should generate a minimal action plan template', () => {
        const template = generateActionsTemplate({ variant: 'minimal' });

        expect(template).toContain('# Actions: [SCOPE-NNN]');
        expect(template).toContain('## Actions');
        expect(template).toContain('**Checkpoint:**');
        expect(template).toContain('**Validate:**');
        expect(template).toContain('## Completion');
        // Should NOT have extra sections
        expect(template).not.toContain('## Prerequisites');
        expect(template).not.toContain('**Purpose:**');
      });
    });

    describe('full variant', () => {
      it('should generate a comprehensive action plan template', () => {
        const template = generateActionsTemplate({ variant: 'full' });

        expect(template).toContain('# Actions: [SCOPE-NNN]');
        expect(template).toContain('## Overview');
        expect(template).toContain('**Intent:**');
        expect(template).toContain('**Expected Outcome:**');
        expect(template).toContain('## Prerequisites');
        expect(template).toContain('## Actions');
        expect(template).toContain('## Blocked/Deferred');
        expect(template).toContain('## Notes');
        expect(template).toContain('## Completion');
      });
    });
  });

  describe('generateAllTemplates', () => {
    it('should generate all template types', () => {
      const templates = generateAllTemplates();

      expect(templates).toHaveProperty('index');
      expect(templates).toHaveProperty('leaf');
      expect(templates).toHaveProperty('simple');
      expect(templates).toHaveProperty('actions');
    });

    it('should return a properly typed TemplateBundle', () => {
      const templates: TemplateBundle = generateAllTemplates();

      // Type-safe access to known keys
      expect(typeof templates.index).toBe('string');
      expect(typeof templates.leaf).toBe('string');
      expect(typeof templates.simple).toBe('string');
      expect(typeof templates.actions).toBe('string');

      // TypeScript will error if trying to access unknown keys
      // @ts-expect-error - 'unknown' does not exist on type 'TemplateBundle'
      const _ = templates.unknown;
    });

    it('should return non-empty templates', () => {
      const templates = generateAllTemplates();

      expect(templates.index).toBeTruthy();
      expect(templates.leaf).toBeTruthy();
      expect(templates.simple).toBeTruthy();
      expect(templates.actions).toBeTruthy();

      expect(templates.index.length).toBeGreaterThan(0);
      expect(templates.leaf.length).toBeGreaterThan(0);
      expect(templates.simple.length).toBeGreaterThan(0);
      expect(templates.actions.length).toBeGreaterThan(0);
    });

    it('should return distinct templates', () => {
      const templates = generateAllTemplates();

      expect(templates.index).not.toEqual(templates.leaf);
      expect(templates.index).not.toEqual(templates.simple);
      expect(templates.index).not.toEqual(templates.actions);
      expect(templates.leaf).not.toEqual(templates.simple);
      expect(templates.leaf).not.toEqual(templates.actions);
      expect(templates.simple).not.toEqual(templates.actions);
    });

    it('should respect variant option', () => {
      const variants: TemplateVariant[] = ['minimal', 'standard', 'full'];

      for (const variant of variants) {
        const templates = generateAllTemplates({ variant });
        expect(templates.index.length).toBeGreaterThan(0);
        expect(templates.leaf.length).toBeGreaterThan(0);
        expect(templates.simple.length).toBeGreaterThan(0);
        expect(templates.actions.length).toBeGreaterThan(0);
      }
    });

    it('should generate different sizes for different variants', () => {
      const minimal = generateAllTemplates({ variant: 'minimal' });
      const standard = generateAllTemplates({ variant: 'standard' });
      const full = generateAllTemplates({ variant: 'full' });

      // Full should be larger than standard, standard larger than minimal
      expect(full.index.length).toBeGreaterThan(standard.index.length);
      expect(standard.index.length).toBeGreaterThan(minimal.index.length);

      expect(full.leaf.length).toBeGreaterThan(standard.leaf.length);
      expect(standard.leaf.length).toBeGreaterThan(minimal.leaf.length);

      expect(full.simple.length).toBeGreaterThan(standard.simple.length);
      expect(standard.simple.length).toBeGreaterThan(minimal.simple.length);

      expect(full.actions.length).toBeGreaterThan(standard.actions.length);
      expect(standard.actions.length).toBeGreaterThan(minimal.actions.length);
    });
  });
});
