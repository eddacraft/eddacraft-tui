/**
 * Golden File Tests
 *
 * These tests ensure hash stability and cross-platform consistency
 * by validating against known-good reference plans.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import { generateHash, verifyHash } from './crypto/index.js';
import { validator } from './validation/index.js';
import type { APSPlan } from './schema/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = join(__dirname, '__fixtures__', 'golden-plans');

describe('Golden File Tests', () => {
  const goldenFiles = ['simple-plan.json', 'complex-plan.json', 'minimal-plan.json'];

  let plans: Map<string, APSPlan>;

  beforeAll(async () => {
    plans = new Map();

    // Load all golden files
    for (const file of goldenFiles) {
      const path = join(FIXTURES_DIR, file);
      const content = await readFile(path, 'utf8');
      const plan = JSON.parse(content) as APSPlan;
      plans.set(file, plan);
    }
  });

  describe('Hash Stability', () => {
    it.each(goldenFiles)('should maintain stable hash for %s', async (file) => {
      const plan = plans.get(file)!;
      const { hash: expectedHash, ...planWithoutHash } = plan;

      // Generate hash and verify it matches the golden hash
      const actualHash = generateHash(planWithoutHash);
      expect(actualHash).toBe(expectedHash);

      // Verify using the verifyHash function
      expect(verifyHash(planWithoutHash, expectedHash)).toBe(true);
    });

    it('should detect hash tampering', async () => {
      const plan = plans.get('simple-plan.json')!;
      const { hash: originalHash, ...planWithoutHash } = plan;

      // Tamper with the plan
      const tamperedPlan = {
        ...planWithoutHash,
        intent: 'Modified intent',
      };

      // Hash should not match
      expect(verifyHash(tamperedPlan, originalHash)).toBe(false);
    });
  });

  describe('Cross-platform Consistency', () => {
    it('should generate consistent hashes regardless of property order', async () => {
      const plan = plans.get('complex-plan.json')!;
      const { hash: expectedHash, ...planWithoutHash } = plan;

      // Reorder top-level properties
      const reorderedPlan = {
        provenance: planWithoutHash.provenance,
        hash: expectedHash,
        proposed_changes: planWithoutHash.proposed_changes,
        id: planWithoutHash.id,
        validations: planWithoutHash.validations,
        schema_version: planWithoutHash.schema_version,
        intent: planWithoutHash.intent,
        metadata: planWithoutHash.metadata,
      };

      // Extract hash and verify
      const { hash: _, ...reorderedWithoutHash } = reorderedPlan;
      const actualHash = generateHash(reorderedWithoutHash);
      expect(actualHash).toBe(expectedHash);
    });

    it('should handle nested object property reordering', async () => {
      const plan = plans.get('complex-plan.json')!;
      const { hash: expectedHash, ...planWithoutHash } = plan;

      // Reorder nested properties
      const reorderedPlan = {
        ...planWithoutHash,
        provenance: {
          version: planWithoutHash.provenance.version,
          repository: planWithoutHash.provenance.repository,
          timestamp: planWithoutHash.provenance.timestamp,
          commit: planWithoutHash.provenance.commit,
          source: planWithoutHash.provenance.source,
          branch: planWithoutHash.provenance.branch,
          author: planWithoutHash.provenance.author,
        },
      };

      const actualHash = generateHash(reorderedPlan);
      expect(actualHash).toBe(expectedHash);
    });
  });

  describe('Schema Validation', () => {
    it.each(goldenFiles)('should validate %s against schema', async (file) => {
      const plan = plans.get(file)!;

      const result = await validator.validateSchema(plan);
      if (!result.valid) {
        console.error(`Validation failed for ${file}:`, result.formattedErrors);
      }
      expect(result.valid).toBe(true);
      expect(result.issues).toBeUndefined();
    });

    it.each(goldenFiles)('should validate %s with hash validation', async (file) => {
      const plan = plans.get(file)!;

      const result = await validator.validate(plan, { validateHash: true });
      if (!result.valid) {
        console.error(`Hash validation failed for ${file}:`, result.summary);
        console.error('Issues:', result.issues);
      }
      expect(result.valid).toBe(true);
      expect(result.summary).toContain('Hash validation passed');
    });
  });

  describe('Regression Prevention', () => {
    it('should ensure hash algorithm compatibility', async () => {
      // Test with a known input/output pair
      const testData = {
        test: 'data',
        nested: {
          value: 123,
          array: [1, 2, 3],
        },
      };

      // This hash was generated with SHA-256
      const knownHash = 'b78c23e6744a95d3f57d354ec40ddfba50c5f92089e4934552aeff53bd0ad6cb';
      const actualHash = generateHash(testData);

      expect(actualHash).toBe(knownHash);
    });

    it('should handle edge cases consistently', async () => {
      const edgeCases = [
        { data: null, hash: '74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b' },
        {
          data: undefined,
          hash: 'eb045d78d273107348b0300c01d29b7552d622abbc6faf81b3ec55359aa9950c',
        },
        { data: '', hash: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' },
        { data: {}, hash: '44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a' },
        { data: [], hash: '4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945' },
      ];

      for (const { data, hash } of edgeCases) {
        expect(generateHash(data)).toBe(hash);
      }
    });
  });

  describe('Plan Examples', () => {
    it('simple-plan.json should represent a basic APS plan', async () => {
      const plan = plans.get('simple-plan.json')!;

      expect(plan.proposed_changes).toHaveLength(2);
      expect(plan.proposed_changes[0].type).toBe('file_create');
      expect(plan.proposed_changes[1].type).toBe('file_update');
      expect(plan.validations.required_checks).toContain('lint');
      expect(plan.validations.required_checks).toContain('test');
    });

    it('complex-plan.json should represent advanced features', async () => {
      const plan = plans.get('complex-plan.json')!;

      expect(plan.proposed_changes).toHaveLength(4);
      expect(plan.proposed_changes.some((c) => c.metadata?.dependencies)).toBe(true);
      expect(plan.provenance.repository).toBeDefined();
      expect(plan.provenance.branch).toBeDefined();
      expect(plan.provenance.commit).toBeDefined();
      expect(plan.validations.custom_rules).toBeDefined();
      expect(plan.metadata).toBeDefined();
      expect(plan.metadata?.tags).toContain('security');
    });

    it('minimal-plan.json should have only required fields', async () => {
      const plan = plans.get('minimal-plan.json')!;

      expect(plan.proposed_changes).toHaveLength(0);
      expect(plan.validations).toBeDefined();
      expect(plan.validations.required_checks).toHaveLength(0);
      expect(plan.metadata).toBeUndefined();
      expect(plan.hash).toBeDefined();
    });
  });
});
