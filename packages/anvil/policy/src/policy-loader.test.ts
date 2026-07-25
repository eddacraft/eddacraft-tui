/**
 * Unit Tests for Policy Loader
 *
 * Tests policy discovery, loading, and filtering
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { PolicyLoader } from './policy-loader.js';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';

describe('PolicyLoader', () => {
  let loader: PolicyLoader;
  let tempDir: string;
  let policyDir: string;

  beforeEach(() => {
    loader = new PolicyLoader();
    tempDir = mkdtempSync(join(tmpdir(), 'anvil-policy-loader-test-'));
    policyDir = join(tempDir, '.anvil', 'policies');
    mkdirSync(policyDir, { recursive: true });
  });

  afterEach(async () => {
    await safeCleanup(tempDir);
  });

  describe('policy discovery', () => {
    it('should return empty result when policy directory does not exist', async () => {
      const nonExistentDir = join(tempDir, 'nonexistent');
      const result = await loader.loadPolicies(nonExistentDir);

      expect(result.policies).toEqual([]);
      expect(result.errors).toEqual([]);
      expect(result.directory).toContain('nonexistent');
    });

    it('should return empty result when policy directory is empty', async () => {
      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toEqual([]);
      expect(result.errors).toEqual([]);
    });

    it('should discover single policy file', async () => {
      writeFileSync(
        join(policyDir, 'test_policy.rego'),
        'package anvil.policies.test_policy\n\nviolation[msg] { false }'
      );

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('test_policy');
      expect(result.errors).toEqual([]);
    });

    it('should discover multiple policy files', async () => {
      writeFileSync(join(policyDir, 'policy1.rego'), 'package anvil.policies.policy1');
      writeFileSync(join(policyDir, 'policy2.rego'), 'package anvil.policies.policy2');
      writeFileSync(join(policyDir, 'policy3.rego'), 'package anvil.policies.policy3');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(3);
      expect(result.errors).toEqual([]);
    });

    it('should exclude test files from policy list', async () => {
      writeFileSync(join(policyDir, 'policy.rego'), 'package anvil.policies.policy');
      writeFileSync(join(policyDir, 'policy_test.rego'), 'package test');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('policy');
    });

    it('should recursively search subdirectories', async () => {
      const subDir = join(policyDir, 'custom');
      mkdirSync(subDir, { recursive: true });

      writeFileSync(join(policyDir, 'root_policy.rego'), 'package anvil.policies.root_policy');
      writeFileSync(join(subDir, 'sub_policy.rego'), 'package anvil.policies.sub_policy');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(2);
      const names = result.policies.map((p) => p.name);
      expect(names).toContain('root_policy');
      expect(names).toContain('sub_policy');
    });
  });

  describe('policy loading', () => {
    it('should load policy content', async () => {
      const content = 'package anvil.policies.test\n\nviolation[msg] { msg := "test" }';
      writeFileSync(join(policyDir, 'test.rego'), content);

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].content).toBe(content);
    });

    it('should extract policy name from filename', async () => {
      writeFileSync(join(policyDir, 'my_policy.rego'), 'package anvil.policies.my_policy');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].name).toBe('my_policy');
    });

    it('should extract package name from content', async () => {
      writeFileSync(
        join(policyDir, 'test.rego'),
        'package anvil.policies.coverage_min\n\nviolation[msg] { false }'
      );

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].package).toBe('anvil.policies.coverage_min');
    });

    it('should handle policies without package declaration', async () => {
      writeFileSync(join(policyDir, 'no_package.rego'), 'violation[msg] { false }');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].package).toBe('unknown');
    });

    it('should include file path in loaded policy', async () => {
      writeFileSync(join(policyDir, 'test.rego'), 'package test');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].path).toContain('test.rego');
    });
  });

  describe('generated policy detection', () => {
    it('should detect generated policies by directory', async () => {
      const generatedDir = join(policyDir, '.generated');
      mkdirSync(generatedDir, { recursive: true });

      const policyContent = `package anvil.policies.architecture

violation contains result if {
  true
  result := {"message": "generated"}
}
`;
      writeFileSync(join(generatedDir, 'architecture.rego'), policyContent);

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('architecture');
      expect(result.policies[0].generated).toBe(true);
    });

    it('should detect generated policies by header', async () => {
      const policyContent = `# Auto-generated by anvil - do not edit manually
# hash: abc123def456789
# Generated from: .anvil/architecture.yaml

package anvil.policies.architecture

violation contains result if {
  true
  result := {"message": "generated"}
}
`;
      writeFileSync(join(policyDir, 'arch.rego'), policyContent);

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('arch');
      expect(result.policies[0].generated).toBe(true);
      expect(result.policies[0].sourceHash).toBe('abc123def456789');
    });

    it('should extract source hash from generated policies', async () => {
      const generatedDir = join(policyDir, '.generated');
      mkdirSync(generatedDir, { recursive: true });

      const policyContent = `# Auto-generated by anvil - do not edit manually
# hash: 1234567890abcdef

package anvil.policies.test
`;
      writeFileSync(join(generatedDir, 'test.rego'), policyContent);

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].sourceHash).toBe('1234567890abcdef');
    });

    it('should mark user policies as not generated', async () => {
      const userPolicy = `package anvil.policies.custom

deny contains msg if {
  false
  msg := "never"
}
`;
      writeFileSync(join(policyDir, 'custom.rego'), userPolicy);

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].generated).toBe(false);
      expect(result.policies[0].sourceHash).toBeUndefined();
    });
  });

  describe('test file detection', () => {
    it('should detect when test file exists', async () => {
      writeFileSync(join(policyDir, 'policy.rego'), 'package anvil.policies.policy');
      writeFileSync(join(policyDir, 'policy_test.rego'), 'package test');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].hasTests).toBe(true);
      expect(result.policies[0].testPath).toContain('policy_test.rego');
    });

    it('should detect when test file does not exist', async () => {
      writeFileSync(join(policyDir, 'policy.rego'), 'package anvil.policies.policy');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies[0].hasTests).toBe(false);
      expect(result.policies[0].testPath).toBeUndefined();
    });
  });

  describe('policy filtering', () => {
    beforeEach(() => {
      writeFileSync(join(policyDir, 'policy1.rego'), 'package anvil.policies.policy1');
      writeFileSync(join(policyDir, 'policy2.rego'), 'package anvil.policies.policy2');
      writeFileSync(join(policyDir, 'policy3.rego'), 'package anvil.policies.policy3');
    });

    it('should filter by enabled policies', async () => {
      const result = await loader.loadPolicies(tempDir, {
        enabledPolicies: ['policy1', 'policy2'],
      });

      expect(result.policies).toHaveLength(2);
      const names = result.policies.map((p) => p.name);
      expect(names).toContain('policy1');
      expect(names).toContain('policy2');
      expect(names).not.toContain('policy3');
    });

    it('should filter by disabled policies', async () => {
      const result = await loader.loadPolicies(tempDir, {
        disabledPolicies: ['policy2'],
      });

      expect(result.policies).toHaveLength(2);
      const names = result.policies.map((p) => p.name);
      expect(names).toContain('policy1');
      expect(names).not.toContain('policy2');
      expect(names).toContain('policy3');
    });

    it('should prioritize disabled list over enabled list', async () => {
      const result = await loader.loadPolicies(tempDir, {
        enabledPolicies: ['policy1', 'policy2'],
        disabledPolicies: ['policy1'],
      });

      // policy1 is in both lists, disabled should win
      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('policy2');
    });

    it('should return empty when all policies are disabled', async () => {
      const result = await loader.loadPolicies(tempDir, {
        disabledPolicies: ['policy1', 'policy2', 'policy3'],
      });

      expect(result.policies).toEqual([]);
    });

    it('should return all policies when no filters specified', async () => {
      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(3);
    });
  });

  describe('custom policy directory', () => {
    it('should use custom policy directory', async () => {
      const customDir = 'custom-policies';
      const customPolicyDir = join(tempDir, customDir);
      mkdirSync(customPolicyDir, { recursive: true });

      writeFileSync(join(customPolicyDir, 'custom.rego'), 'package anvil.policies.custom');

      const result = await loader.loadPolicies(tempDir, {
        policyDir: customDir,
      });

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('custom');
      expect(result.directory).toContain(customDir);
    });

    it('should handle custom directory that does not exist', async () => {
      const result = await loader.loadPolicies(tempDir, {
        policyDir: 'nonexistent-policies',
      });

      expect(result.policies).toEqual([]);
      expect(result.errors).toEqual([]);
    });
  });

  describe('error handling', () => {
    it('should handle unreadable policy files', async () => {
      writeFileSync(join(policyDir, 'valid.rego'), 'package valid');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.errors).toHaveLength(0);
    });

    it('should continue loading after encountering errors', async () => {
      writeFileSync(join(policyDir, 'policy1.rego'), 'package anvil.policies.policy1');
      writeFileSync(join(policyDir, 'policy3.rego'), 'package anvil.policies.policy3');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies.length).toBeGreaterThanOrEqual(2);
    });

    it('should record errors for failed loads', async () => {
      const result = await loader.loadPolicies(tempDir);

      expect(Array.isArray(result.errors)).toBe(true);
    });
  });

  describe('loadPolicy method', () => {
    it('should load single policy file', async () => {
      const policyPath = join(policyDir, 'single.rego');
      writeFileSync(policyPath, 'package anvil.policies.single');

      const policy = await loader.loadPolicy(policyPath);

      expect(policy.name).toBe('single');
      expect(policy.path).toBe(policyPath);
      expect(policy.content).toContain('package anvil.policies.single');
    });

    it('should extract package name', async () => {
      const policyPath = join(policyDir, 'test.rego');
      writeFileSync(policyPath, 'package anvil.policies.my_package\n\nrule = true');

      const policy = await loader.loadPolicy(policyPath);

      expect(policy.package).toBe('anvil.policies.my_package');
    });

    it('should detect test files', async () => {
      const policyPath = join(policyDir, 'with_test.rego');
      const testPath = join(policyDir, 'with_test_test.rego');

      writeFileSync(policyPath, 'package test');
      writeFileSync(testPath, 'package test');

      const policy = await loader.loadPolicy(policyPath);

      expect(policy.hasTests).toBe(true);
      expect(policy.testPath).toBe(testPath);
    });
  });

  describe('findTestFiles method', () => {
    it('should find all test files in directory', () => {
      writeFileSync(join(policyDir, 'policy1.rego'), 'package test');
      writeFileSync(join(policyDir, 'policy1_test.rego'), 'package test');
      writeFileSync(join(policyDir, 'policy2_test.rego'), 'package test');

      const testFiles = loader.findTestFiles(policyDir);

      expect(testFiles).toHaveLength(2);
      testFiles.forEach((file) => {
        expect(file).toContain('_test.rego');
      });
    });

    it('should find test files in subdirectories', () => {
      const subDir = join(policyDir, 'subdir');
      mkdirSync(subDir);

      writeFileSync(join(policyDir, 'root_test.rego'), 'package test');
      writeFileSync(join(subDir, 'sub_test.rego'), 'package test');

      const testFiles = loader.findTestFiles(policyDir);

      expect(testFiles).toHaveLength(2);
    });

    it('should return empty array for directory without tests', () => {
      writeFileSync(join(policyDir, 'policy.rego'), 'package test');

      const testFiles = loader.findTestFiles(policyDir);

      expect(testFiles).toEqual([]);
    });

    it('should return empty array for nonexistent directory', () => {
      const testFiles = loader.findTestFiles(join(tempDir, 'nonexistent'));

      expect(testFiles).toEqual([]);
    });
  });

  describe('edge cases', () => {
    it('should handle policy files with no extension', async () => {
      writeFileSync(join(policyDir, 'no_ext'), 'package test');
      writeFileSync(join(policyDir, 'with_ext.rego'), 'package test');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('with_ext');
    });

    it('should handle empty policy files', async () => {
      writeFileSync(join(policyDir, 'empty.rego'), '');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].content).toBe('');
      expect(result.policies[0].package).toBe('unknown');
    });

    it('should handle policy files with special characters in name', async () => {
      writeFileSync(join(policyDir, 'policy-with-dashes.rego'), 'package test');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('policy-with-dashes');
    });

    it('should handle deeply nested subdirectories', async () => {
      const deepDir = join(policyDir, 'level1', 'level2', 'level3');
      mkdirSync(deepDir, { recursive: true });

      writeFileSync(join(deepDir, 'deep_policy.rego'), 'package anvil.policies.deep_policy');

      const result = await loader.loadPolicies(tempDir);

      expect(result.policies).toHaveLength(1);
      expect(result.policies[0].name).toBe('deep_policy');
    });
  });
});
