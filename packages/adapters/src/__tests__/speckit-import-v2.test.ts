import { describe, it, expect } from 'vitest';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SpecKitImportAdapterV2 } from '../speckit/import-v2.js';
import type { ExternalSpec } from '../common/types.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, 'fixtures/speckit-official/auth-feature');

describe('SpecKitImportAdapterV2 - Official Format', () => {
  let adapter: SpecKitImportAdapterV2;

  beforeEach(() => {
    adapter = new SpecKitImportAdapterV2();
  });

  it('should have correct name and version', () => {
    expect(adapter.name).toBe('speckit-import-v2');
    expect(adapter.version).toBe('2.0.0');
  });

  it('should support spec-kit formats', () => {
    expect(adapter.canImport('speckit')).toBe(true);
    expect(adapter.canImport('spec-kit')).toBe(true);
    expect(adapter.canImport('spec.md')).toBe(true);
    expect(adapter.canImport('plan.md')).toBe(true);
    expect(adapter.canImport('tasks.md')).toBe(true);
    expect(adapter.canImport('unknown')).toBe(false);
  });

  it('should convert spec.md only to APS', async () => {
    const specContent = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');

    const externalSpec: ExternalSpec = {
      format: 'spec.md',
      version: '2.0.0',
      content: {
        spec: { content: specContent },
        metadata: {
          timestamp: '2025-01-15T10:00:00Z',
          author: 'test@example.com',
        },
      },
    };

    const result = await adapter.convertToAPS(externalSpec);

    expect(result.success).toBe(true);
    expect(result.data).toBeDefined();

    if (result.success && result.data) {
      // Check intent was built from user scenarios
      expect(result.data.intent).toContain('User Authentication System');
      expect(result.data.intent).toContain('new user wants to');

      // Check metadata preservation
      expect(result.data.metadata?.feature).toBe('User Authentication System');
      expect(result.data.metadata?.userScenarios).toBeDefined();
      expect(result.data.metadata?.requirements).toBeDefined();
      expect(result.data.metadata?.successCriteria).toBeDefined();
      expect(result.data.metadata?.clarifications).toBeDefined();

      // Check proposed changes generated from scenarios
      expect(result.data.proposed_changes.length).toBeGreaterThan(0);

      // P1 and P2 scenarios should become changes
      const p1Changes = result.data.proposed_changes.filter((c) => c.metadata?.priority === 'P1');
      expect(p1Changes.length).toBeGreaterThan(0);
    }
  });

  it('should convert spec.md + plan.md to APS', async () => {
    const specContent = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const planContent = await readFile(join(fixturesDir, 'plan.md'), 'utf-8');

    const externalSpec: ExternalSpec = {
      format: 'speckit',
      version: '2.0.0',
      content: {
        spec: { content: specContent },
        plan: { content: planContent },
        metadata: {
          timestamp: '2025-01-15T10:00:00Z',
          author: 'test@example.com',
        },
      },
    };

    const result = await adapter.convertToAPS(externalSpec);

    expect(result.success).toBe(true);
    expect(result.data).toBeDefined();

    if (result.success && result.data) {
      // Check technical context from plan.md
      expect(result.data.metadata?.technicalContext).toBeDefined();
      expect(result.data.metadata?.technicalContext?.language).toContain('TypeScript');
      expect(result.data.metadata?.technicalContext?.dependencies).toBeDefined();
      expect(result.data.metadata?.technicalContext?.dependencies?.length).toBeGreaterThan(0);

      // Check constitution check
      expect(result.data.metadata?.constitutionCheck).toBeDefined();

      // Check project structure
      expect(result.data.metadata?.projectStructure).toBeDefined();

      // Check implementation details
      expect(result.data.metadata?.implementationDetails).toBeDefined();

      // Should have more changes with plan details
      expect(result.data.proposed_changes.length).toBeGreaterThan(2);

      // Should have dependency installation change
      const depChange = result.data.proposed_changes.find((c) => c.type === 'dependency_add');
      expect(depChange).toBeDefined();
      expect(depChange?.description).toContain('dependencies');
    }
  });

  it('should convert spec.md + plan.md + tasks.md to APS', async () => {
    const specContent = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const planContent = await readFile(join(fixturesDir, 'plan.md'), 'utf-8');
    const tasksContent = await readFile(join(fixturesDir, 'tasks.md'), 'utf-8');

    const externalSpec: ExternalSpec = {
      format: 'speckit',
      version: '2.0.0',
      content: {
        spec: { content: specContent },
        plan: { content: planContent },
        tasks: { content: tasksContent },
        metadata: {
          timestamp: '2025-01-15T10:00:00Z',
          author: 'test@example.com',
        },
      },
    };

    const result = await adapter.convertToAPS(externalSpec);

    expect(result.success).toBe(true);
    expect(result.data).toBeDefined();

    if (result.success && result.data) {
      // Check phases from tasks.md
      expect(result.data.metadata?.phases).toBeDefined();
      expect(result.data.metadata?.phases?.length).toBeGreaterThan(0);

      // Check task dependencies
      expect(result.data.metadata?.taskDependencies).toBeDefined();

      // Check implementation strategies
      expect(result.data.metadata?.implementationStrategies).toBeDefined();

      // Verify complete metadata
      expect(result.data.metadata?.source_format).toBe('speckit-v2');
    }
  });

  it('should handle missing spec.md', async () => {
    const externalSpec: ExternalSpec = {
      format: 'speckit',
      version: '2.0.0',
      content: {
        plan: { content: '# Plan\nSome plan' },
      },
    };

    const result = await adapter.convertToAPS(externalSpec);

    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors?.[0].code).toBe('MISSING_SPEC');
  });

  it('should preserve clarifications in metadata', async () => {
    const specContent = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');

    const externalSpec: ExternalSpec = {
      format: 'spec.md',
      version: '2.0.0',
      content: {
        spec: { content: specContent },
      },
    };

    const result = await adapter.convertToAPS(externalSpec);

    expect(result.success).toBe(true);
    if (result.success && result.data) {
      const clarifications = result.data.metadata?.clarifications;
      expect(clarifications).toBeDefined();
      expect(Array.isArray(clarifications)).toBe(true);
      expect((clarifications as string[]).length).toBeGreaterThan(0);
    }
  });

  it('should map user scenarios to proposed changes', async () => {
    const specContent = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');

    const externalSpec: ExternalSpec = {
      format: 'spec.md',
      version: '2.0.0',
      content: {
        spec: { content: specContent },
      },
    };

    const result = await adapter.convertToAPS(externalSpec);

    expect(result.success).toBe(true);
    if (result.success && result.data) {
      const changes = result.data.proposed_changes;

      // Should have changes for user scenarios
      const scenarioChanges = changes.filter((c) => c.metadata?.userStory);
      expect(scenarioChanges.length).toBeGreaterThan(0);

      // Check first scenario change
      const firstChange = scenarioChanges[0];
      expect(firstChange.metadata?.userStory?.asA).toBeDefined();
      expect(firstChange.metadata?.userStory?.iWantTo).toBeDefined();
      expect(firstChange.metadata?.userStory?.soThat).toBeDefined();
      expect(firstChange.metadata?.acceptanceScenarios).toBeDefined();
      expect(firstChange.metadata?.edgeCases).toBeDefined();
    }
  });

  it('should preserve success criteria structure', async () => {
    const specContent = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');

    const externalSpec: ExternalSpec = {
      format: 'spec.md',
      version: '2.0.0',
      content: {
        spec: { content: specContent },
      },
    };

    const result = await adapter.convertToAPS(externalSpec);

    expect(result.success).toBe(true);
    if (result.success && result.data) {
      const successCriteria = result.data.metadata?.successCriteria as Record<string, unknown>;
      expect(successCriteria).toBeDefined();
      expect(successCriteria.quantitative).toBeDefined();
      expect(successCriteria.qualitative).toBeDefined();
      expect(successCriteria.security).toBeDefined();
      expect(Array.isArray(successCriteria.quantitative)).toBe(true);
    }
  });
});
