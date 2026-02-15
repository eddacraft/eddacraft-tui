/**
 * Unit Tests for Policy diagnostic checks
 *
 * Tests the four policy-related doctor checks:
 * - PolicyConfigCheck — .anvil/config.yml existence
 * - PolicyDirectoryCheck — .rego files and tests
 * - PolicyDocumentationCheck — reasons and owners in config
 * - PolicyOrgVersionCheck — org source pinned version
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import {
  PolicyConfigCheck,
  PolicyDirectoryCheck,
  PolicyDocumentationCheck,
  PolicyOrgVersionCheck,
} from '../checks/PolicyCheck.js';
import type { DiagnosticContext } from '../types.js';

describe('PolicyChecks', () => {
  const tempDir = path.join(process.cwd(), 'tmp-policy-checks-test');

  beforeEach(() => {
    fs.mkdirSync(tempDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  const ctx = (): DiagnosticContext => ({
    projectRoot: tempDir,
    verbose: false,
  });

  describe('PolicyConfigCheck', () => {
    it('should have correct id and name', () => {
      const check = new PolicyConfigCheck();
      expect(check.id).toBe('policy-config');
      expect(check.name).toBe('Policy Configuration');
    });

    it('should warn when .anvil/config.yml is missing', async () => {
      const check = new PolicyConfigCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('No .anvil/config.yml');
      expect(result.fixable).toBe(false);
      expect(result.suggestion).toContain('anvil init --org');
    });

    it('should pass when .anvil/config.yml exists', async () => {
      const anvilDir = path.join(tempDir, '.anvil');
      fs.mkdirSync(anvilDir, { recursive: true });
      fs.writeFileSync(path.join(anvilDir, 'config.yml'), 'policies: {}', 'utf-8');

      const check = new PolicyConfigCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('pass');
      expect(result.message).toContain('.anvil/config.yml found');
    });
  });

  describe('PolicyDirectoryCheck', () => {
    it('should have correct id and name', () => {
      const check = new PolicyDirectoryCheck();
      expect(check.id).toBe('policy-directory');
      expect(check.name).toBe('Policy Files');
    });

    it('should warn when .anvil/policies/ does not exist', async () => {
      const check = new PolicyDirectoryCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('No .anvil/policies/ directory');
      expect(result.fixable).toBe(false);
      expect(result.suggestion).toContain('anvil policy init');
    });

    it('should warn when directory exists but has no .rego files', async () => {
      const policyDir = path.join(tempDir, '.anvil', 'policies');
      fs.mkdirSync(policyDir, { recursive: true });

      const check = new PolicyDirectoryCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('no .rego files');
      expect(result.fixable).toBe(false);
    });

    it('should warn when policies have no tests', async () => {
      const policyDir = path.join(tempDir, '.anvil', 'policies');
      fs.mkdirSync(policyDir, { recursive: true });
      fs.writeFileSync(path.join(policyDir, 'secret-scan.rego'), '# policy', 'utf-8');
      fs.writeFileSync(path.join(policyDir, 'coverage_min.rego'), '# policy', 'utf-8');

      const check = new PolicyDirectoryCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('2 policies found');
      expect(result.message).toContain('2 missing tests');
      expect(result.details).toContain('secret-scan.rego');
      expect(result.details).toContain('coverage_min.rego');
    });

    it('should pass when all policies have tests', async () => {
      const policyDir = path.join(tempDir, '.anvil', 'policies');
      fs.mkdirSync(policyDir, { recursive: true });
      fs.writeFileSync(path.join(policyDir, 'secret-scan.rego'), '# policy', 'utf-8');
      fs.writeFileSync(path.join(policyDir, 'secret-scan_test.rego'), '# test', 'utf-8');

      const check = new PolicyDirectoryCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('pass');
      expect(result.message).toContain('1 policies, all with tests');
    });

    it('should report partial test coverage', async () => {
      const policyDir = path.join(tempDir, '.anvil', 'policies');
      fs.mkdirSync(policyDir, { recursive: true });
      fs.writeFileSync(path.join(policyDir, 'secret-scan.rego'), '# policy', 'utf-8');
      fs.writeFileSync(path.join(policyDir, 'secret-scan_test.rego'), '# test', 'utf-8');
      fs.writeFileSync(path.join(policyDir, 'coverage_min.rego'), '# policy', 'utf-8');
      // coverage_min has no test

      const check = new PolicyDirectoryCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('2 policies found');
      expect(result.message).toContain('1 missing tests');
      expect(result.details).toContain('coverage_min.rego');
      expect(result.details).not.toContain('secret-scan.rego');
    });

    it('should have a fix method that returns guidance', async () => {
      const check = new PolicyDirectoryCheck();
      const fixResult = await check.fix!(ctx());

      expect(fixResult.success).toBe(false);
      expect(fixResult.message).toContain('anvil policy init');
    });
  });

  describe('PolicyDocumentationCheck', () => {
    it('should have correct id and name', () => {
      const check = new PolicyDocumentationCheck();
      expect(check.id).toBe('policy-docs');
      expect(check.name).toBe('Policy Documentation');
    });

    it('should skip when no config.yml exists', async () => {
      const check = new PolicyDocumentationCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('skip');
      expect(result.message).toContain('no .anvil/config.yml');
    });

    it('should warn when no team policies defined', async () => {
      const anvilDir = path.join(tempDir, '.anvil');
      fs.mkdirSync(anvilDir, { recursive: true });
      fs.writeFileSync(
        path.join(anvilDir, 'config.yml'),
        'policies:\n  org:\n    source: test',
        'utf-8'
      );

      const check = new PolicyDocumentationCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('No team policies');
    });

    it('should warn when reasons are missing', async () => {
      const anvilDir = path.join(tempDir, '.anvil');
      fs.mkdirSync(anvilDir, { recursive: true });
      const yaml = `policies:
  team:
    - name: secret-scan
      owner: "@security"
      enforcement: block
`;
      fs.writeFileSync(path.join(anvilDir, 'config.yml'), yaml, 'utf-8');

      const check = new PolicyDocumentationCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('reasons');
    });

    it('should warn when owners are missing', async () => {
      const anvilDir = path.join(tempDir, '.anvil');
      fs.mkdirSync(anvilDir, { recursive: true });
      const yaml = `policies:
  team:
    - name: secret-scan
      reason: "Prevent leaks"
      enforcement: block
`;
      fs.writeFileSync(path.join(anvilDir, 'config.yml'), yaml, 'utf-8');

      const check = new PolicyDocumentationCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('owners');
    });

    it('should pass when team policies have reasons and owners', async () => {
      const anvilDir = path.join(tempDir, '.anvil');
      fs.mkdirSync(anvilDir, { recursive: true });
      const yaml = `policies:
  team:
    - name: secret-scan
      reason: "Prevent leaks"
      owner: "@security"
      enforcement: block
`;
      fs.writeFileSync(path.join(anvilDir, 'config.yml'), yaml, 'utf-8');

      const check = new PolicyDocumentationCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('pass');
      expect(result.message).toContain('reasons and owners documented');
    });
  });

  describe('PolicyOrgVersionCheck', () => {
    it('should have correct id and name', () => {
      const check = new PolicyOrgVersionCheck();
      expect(check.id).toBe('policy-org-version');
      expect(check.name).toBe('Org Policy Version');
    });

    it('should skip when no config.yml exists', async () => {
      const check = new PolicyOrgVersionCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('skip');
      expect(result.message).toContain('no .anvil/config.yml');
    });

    it('should skip when no org source configured', async () => {
      const anvilDir = path.join(tempDir, '.anvil');
      fs.mkdirSync(anvilDir, { recursive: true });
      fs.writeFileSync(path.join(anvilDir, 'config.yml'), 'policies:\n  team: []', 'utf-8');

      const check = new PolicyOrgVersionCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('skip');
      expect(result.message).toContain('No org source configured');
    });

    it('should warn when org source has no pinned ref', async () => {
      const anvilDir = path.join(tempDir, '.anvil');
      fs.mkdirSync(anvilDir, { recursive: true });
      const yaml = `policies:
  org:
    source: "git@github.com:acme/policies.git"
`;
      fs.writeFileSync(path.join(anvilDir, 'config.yml'), yaml, 'utf-8');

      const check = new PolicyOrgVersionCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('warn');
      expect(result.message).toContain('no pinned version ref');
      expect(result.suggestion).toContain('ref');
    });

    it('should pass when org source has pinned ref', async () => {
      const anvilDir = path.join(tempDir, '.anvil');
      fs.mkdirSync(anvilDir, { recursive: true });
      const yaml = `policies:
  org:
    source: "git@github.com:acme/policies.git"
    ref: "v1.0.0"
`;
      fs.writeFileSync(path.join(anvilDir, 'config.yml'), yaml, 'utf-8');

      const check = new PolicyOrgVersionCheck();
      const result = await check.run(ctx());

      expect(result.status).toBe('pass');
      expect(result.message).toContain('Org source version pinned');
    });
  });
});
