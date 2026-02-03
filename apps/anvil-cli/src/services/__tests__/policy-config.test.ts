/**
 * Unit Tests for PolicyConfigManager
 *
 * Tests YAML-based policy configuration with org/team/local layering:
 * - Loading and saving config.yml
 * - Policy resolution with layer priority
 * - Disable/enable policy mutations
 * - Org scaffold generation
 * - Policies doc generation
 * - Starter profile selection
 * - Effective date (graduated enforcement)
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFileSync, mkdirSync, existsSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import YAML from 'yaml';
import {
  PolicyConfigManager,
  selectStarterProfile,
  getStarterProfile,
  getAllStarterProfiles,
  getConfigPath,
  type AnvilConfig,
  type PolicyEntry,
} from '../policy-config.js';
import { createTestWorkspace, type TestWorkspace } from '../../__tests__/helpers/test-workspace.js';

describe('PolicyConfigManager', () => {
  let workspace: TestWorkspace;
  let configMgr: PolicyConfigManager;

  beforeEach(() => {
    workspace = createTestWorkspace();
    configMgr = new PolicyConfigManager(workspace.root);
  });

  afterEach(() => {
    workspace.cleanup();
  });

  describe('getConfigPath', () => {
    it('should return .anvil/config.yml path', () => {
      const configPath = getConfigPath(workspace.root);
      expect(configPath).toBe(join(workspace.root, '.anvil', 'config.yml'));
    });
  });

  describe('exists()', () => {
    it('should return false when config.yml does not exist', () => {
      expect(configMgr.exists()).toBe(false);
    });

    it('should return true when config.yml exists', () => {
      const configPath = join(workspace.root, '.anvil', 'config.yml');
      mkdirSync(join(workspace.root, '.anvil'), { recursive: true });
      writeFileSync(configPath, 'policies: {}', 'utf-8');
      expect(configMgr.exists()).toBe(true);
    });
  });

  describe('load()', () => {
    it('should return empty config when file does not exist', () => {
      const config = configMgr.load();
      expect(config).toEqual({});
    });

    it('should parse YAML config correctly', () => {
      const configData: AnvilConfig = {
        policies: {
          org: { source: 'git@github.com:acme/policies.git', ref: 'v1.0.0' },
          team: [
            {
              name: 'secret-scan',
              reason: 'Prevent secrets in code',
              owner: '@security',
              enforcement: 'block',
            },
          ],
        },
      };

      mkdirSync(join(workspace.root, '.anvil'), { recursive: true });
      writeFileSync(
        join(workspace.root, '.anvil', 'config.yml'),
        YAML.stringify(configData),
        'utf-8'
      );

      const loaded = configMgr.load();
      expect(loaded.policies?.org?.source).toBe('git@github.com:acme/policies.git');
      expect(loaded.policies?.org?.ref).toBe('v1.0.0');
      expect(loaded.policies?.team).toHaveLength(1);
      expect(loaded.policies?.team?.[0].name).toBe('secret-scan');
      expect(loaded.policies?.team?.[0].enforcement).toBe('block');
    });

    it('should handle empty YAML file', () => {
      mkdirSync(join(workspace.root, '.anvil'), { recursive: true });
      writeFileSync(join(workspace.root, '.anvil', 'config.yml'), '', 'utf-8');

      const loaded = configMgr.load();
      expect(loaded).toEqual({});
    });
  });

  describe('save()', () => {
    it('should create .anvil directory if missing', () => {
      // Remove the .anvil dir that createTestWorkspace makes
      rmSync(join(workspace.root, '.anvil'), { recursive: true, force: true });

      configMgr.save({ policies: { team: [] } });
      expect(existsSync(join(workspace.root, '.anvil', 'config.yml'))).toBe(true);
    });

    it('should write valid YAML', () => {
      const config: AnvilConfig = {
        policies: {
          team: [{ name: 'test-policy', enforcement: 'warn', reason: 'Testing' }],
        },
      };

      configMgr.save(config);

      const raw = readFileSync(join(workspace.root, '.anvil', 'config.yml'), 'utf-8');
      const parsed = YAML.parse(raw);
      expect(parsed.policies.team[0].name).toBe('test-policy');
      expect(parsed.policies.team[0].enforcement).toBe('warn');
    });

    it('should roundtrip config correctly', () => {
      const config: AnvilConfig = {
        policies: {
          org: { source: 'git@github.com:org/policies.git', ref: 'v2.0.0' },
          team: [
            { name: 'coverage_min', enforcement: 'block', reason: 'Coverage matters' },
            { name: 'change_scope', enforcement: 'warn' },
          ],
          local: [{ name: 'coverage_min', enforcement: 'off', reason: 'Disabled locally' }],
          starter_profile: 'web-frontend',
        },
        announcements: [{ message: 'New policy coming', level: 'info', expires: '2026-12-01' }],
      };

      configMgr.save(config);
      const loaded = configMgr.load();

      expect(loaded.policies?.org?.source).toBe(config.policies?.org?.source);
      expect(loaded.policies?.org?.ref).toBe(config.policies?.org?.ref);
      expect(loaded.policies?.team).toHaveLength(2);
      expect(loaded.policies?.local).toHaveLength(1);
      expect(loaded.policies?.starter_profile).toBe('web-frontend');
      expect(loaded.announcements).toHaveLength(1);
    });
  });

  describe('getPath()', () => {
    it('should return the config file path', () => {
      expect(configMgr.getPath()).toBe(join(workspace.root, '.anvil', 'config.yml'));
    });
  });

  describe('resolvePolicies()', () => {
    it('should return empty array when no config and no rego files', () => {
      const resolved = configMgr.resolvePolicies();
      expect(resolved).toEqual([]);
    });

    it('should discover rego files as starter policies', () => {
      const policyDir = join(workspace.root, '.anvil', 'policies');
      mkdirSync(policyDir, { recursive: true });
      writeFileSync(join(policyDir, 'secret-scan.rego'), '# policy', 'utf-8');

      const resolved = configMgr.resolvePolicies();
      expect(resolved).toHaveLength(1);
      expect(resolved[0].name).toBe('secret-scan');
      expect(resolved[0].source).toBe('starter');
      expect(resolved[0].enforcement).toBe('block');
      expect(resolved[0].active).toBe(true);
      expect(resolved[0].hasRegoFile).toBe(true);
    });

    it('should ignore test rego files', () => {
      const policyDir = join(workspace.root, '.anvil', 'policies');
      mkdirSync(policyDir, { recursive: true });
      writeFileSync(join(policyDir, 'secret-scan.rego'), '# policy', 'utf-8');
      writeFileSync(join(policyDir, 'secret-scan_test.rego'), '# test', 'utf-8');

      const resolved = configMgr.resolvePolicies();
      expect(resolved).toHaveLength(1);
      expect(resolved[0].name).toBe('secret-scan');
    });

    it('should layer team policies over starter policies', () => {
      const policyDir = join(workspace.root, '.anvil', 'policies');
      mkdirSync(policyDir, { recursive: true });
      writeFileSync(join(policyDir, 'secret-scan.rego'), '# policy', 'utf-8');

      const config: AnvilConfig = {
        policies: {
          team: [
            {
              name: 'secret-scan',
              enforcement: 'warn',
              reason: 'Team override',
              owner: '@security',
            },
          ],
        },
      };

      const resolved = configMgr.resolvePolicies(config);
      expect(resolved).toHaveLength(1);
      expect(resolved[0].source).toBe('team');
      expect(resolved[0].enforcement).toBe('warn');
      expect(resolved[0].reason).toBe('Team override');
      expect(resolved[0].owner).toBe('@security');
      expect(resolved[0].hasRegoFile).toBe(true);
    });

    it('should layer local overrides over team policies', () => {
      const config: AnvilConfig = {
        policies: {
          team: [{ name: 'coverage_min', enforcement: 'block', reason: 'High coverage' }],
          local: [{ name: 'coverage_min', enforcement: 'warn', reason: 'Relaxed locally' }],
        },
      };

      const resolved = configMgr.resolvePolicies(config);
      const coverage = resolved.find((p) => p.name === 'coverage_min');
      expect(coverage?.source).toBe('local');
      expect(coverage?.enforcement).toBe('warn');
      expect(coverage?.reason).toBe('Relaxed locally');
    });

    it('should add team policies not in starter set', () => {
      const config: AnvilConfig = {
        policies: {
          team: [{ name: 'new-policy', enforcement: 'info', reason: 'Brand new' }],
        },
      };

      const resolved = configMgr.resolvePolicies(config);
      expect(resolved).toHaveLength(1);
      expect(resolved[0].name).toBe('new-policy');
      expect(resolved[0].source).toBe('team');
      expect(resolved[0].hasRegoFile).toBe(false);
    });

    it('should mark policies with enforcement "off" as inactive', () => {
      const config: AnvilConfig = {
        policies: {
          team: [{ name: 'disabled-policy', enforcement: 'off' }],
        },
      };

      const resolved = configMgr.resolvePolicies(config);
      expect(resolved[0].active).toBe(false);
    });

    it('should mark policies with future effective date as inactive', () => {
      const futureDate = new Date(Date.now() + 365 * 24 * 60 * 60 * 1000)
        .toISOString()
        .split('T')[0];
      const config: AnvilConfig = {
        policies: {
          team: [{ name: 'future-policy', enforcement: 'block', effective: futureDate }],
        },
      };

      const resolved = configMgr.resolvePolicies(config);
      expect(resolved[0].active).toBe(false);
      expect(resolved[0].effective).toBe(futureDate);
    });

    it('should mark policies with past effective date as active', () => {
      const pastDate = '2020-01-01';
      const config: AnvilConfig = {
        policies: {
          team: [{ name: 'active-policy', enforcement: 'block', effective: pastDate }],
        },
      };

      const resolved = configMgr.resolvePolicies(config);
      expect(resolved[0].active).toBe(true);
    });

    it('should preserve tags from config', () => {
      const config: AnvilConfig = {
        policies: {
          team: [
            {
              name: 'tagged-policy',
              enforcement: 'warn',
              tags: ['security', 'compliance'],
            },
          ],
        },
      };

      const resolved = configMgr.resolvePolicies(config);
      expect(resolved[0].tags).toEqual(['security', 'compliance']);
    });
  });

  describe('disablePolicy()', () => {
    it('should add local override with enforcement "off"', () => {
      // Save initial config with a team policy
      configMgr.save({
        policies: {
          team: [{ name: 'secret-scan', enforcement: 'block' }],
        },
      });

      const result = configMgr.disablePolicy('secret-scan');
      expect(result.policies?.local).toHaveLength(1);
      expect(result.policies?.local?.[0].name).toBe('secret-scan');
      expect(result.policies?.local?.[0].enforcement).toBe('off');
    });

    it('should update existing local override to "off"', () => {
      configMgr.save({
        policies: {
          local: [{ name: 'secret-scan', enforcement: 'warn' }],
        },
      });

      const result = configMgr.disablePolicy('secret-scan');
      expect(result.policies?.local).toHaveLength(1);
      expect(result.policies?.local?.[0].enforcement).toBe('off');
    });

    it('should persist to disk', () => {
      configMgr.save({ policies: {} });
      configMgr.disablePolicy('my-policy');

      const loaded = configMgr.load();
      expect(loaded.policies?.local?.find((p) => p.name === 'my-policy')?.enforcement).toBe('off');
    });

    it('should create policies section if missing', () => {
      configMgr.save({});
      const result = configMgr.disablePolicy('new-policy');
      expect(result.policies?.local).toHaveLength(1);
    });
  });

  describe('enablePolicy()', () => {
    it('should remove local override when enforcement is block (default)', () => {
      configMgr.save({
        policies: {
          local: [{ name: 'secret-scan', enforcement: 'off' }],
        },
      });

      const result = configMgr.enablePolicy('secret-scan');
      expect(result.policies?.local).toHaveLength(0);
    });

    it('should update enforcement level when not block', () => {
      configMgr.save({
        policies: {
          local: [{ name: 'secret-scan', enforcement: 'off' }],
        },
      });

      const result = configMgr.enablePolicy('secret-scan', 'warn');
      expect(result.policies?.local).toHaveLength(1);
      expect(result.policies?.local?.[0].enforcement).toBe('warn');
    });

    it('should do nothing when no local override exists', () => {
      configMgr.save({
        policies: {
          team: [{ name: 'secret-scan', enforcement: 'block' }],
        },
      });

      const result = configMgr.enablePolicy('secret-scan');
      expect(result.policies?.local).toBeUndefined();
    });

    it('should persist to disk', () => {
      configMgr.save({
        policies: {
          local: [{ name: 'my-policy', enforcement: 'off' }],
        },
      });

      configMgr.enablePolicy('my-policy');
      const loaded = configMgr.load();
      expect(loaded.policies?.local).toHaveLength(0);
    });
  });

  describe('setTeamPolicy()', () => {
    it('should add a new team policy', () => {
      configMgr.save({ policies: {} });

      const entry: PolicyEntry = {
        name: 'new-rule',
        enforcement: 'warn',
        reason: 'Important rule',
        owner: '@platform',
      };

      const result = configMgr.setTeamPolicy(entry);
      expect(result.policies?.team).toHaveLength(1);
      expect(result.policies?.team?.[0]).toEqual(entry);
    });

    it('should update existing team policy', () => {
      configMgr.save({
        policies: {
          team: [{ name: 'existing', enforcement: 'warn' }],
        },
      });

      const updated: PolicyEntry = {
        name: 'existing',
        enforcement: 'block',
        reason: 'Upgraded',
      };

      const result = configMgr.setTeamPolicy(updated);
      expect(result.policies?.team).toHaveLength(1);
      expect(result.policies?.team?.[0].enforcement).toBe('block');
      expect(result.policies?.team?.[0].reason).toBe('Upgraded');
    });
  });

  describe('setOrgSource()', () => {
    it('should set org source', () => {
      configMgr.save({ policies: {} });

      const result = configMgr.setOrgSource({
        source: 'git@github.com:acme/policies.git',
        ref: 'v1.0.0',
      });

      expect(result.policies?.org?.source).toBe('git@github.com:acme/policies.git');
      expect(result.policies?.org?.ref).toBe('v1.0.0');
    });

    it('should persist to disk', () => {
      configMgr.save({ policies: {} });
      configMgr.setOrgSource({ source: 'git@github.com:org/p.git' });

      const loaded = configMgr.load();
      expect(loaded.policies?.org?.source).toBe('git@github.com:org/p.git');
    });
  });

  describe('generateOrgScaffold()', () => {
    it('should generate valid YAML with team policies', () => {
      configMgr.save({
        policies: {
          team: [
            { name: 'secret-scan', enforcement: 'block', reason: 'Security' },
            { name: 'coverage_min', enforcement: 'warn', reason: 'Quality' },
          ],
        },
      });

      const yaml = configMgr.generateOrgScaffold('acme');
      const parsed = YAML.parse(yaml);

      expect(parsed.policies.team).toHaveLength(2);
      expect(parsed.policies.team[0].name).toBe('secret-scan');
    });

    it('should omit team section when no team policies exist', () => {
      configMgr.save({ policies: {} });

      const yaml = configMgr.generateOrgScaffold('acme');
      const parsed = YAML.parse(yaml);

      expect(parsed.policies.team).toBeUndefined();
    });
  });

  describe('generatePoliciesDoc()', () => {
    it('should generate markdown with header', () => {
      configMgr.save({ policies: {} });

      const doc = configMgr.generatePoliciesDoc();
      expect(doc).toContain('# Policy Documentation');
      expect(doc).toContain('Auto-generated by `anvil policy doc`');
    });

    it('should include active policies table', () => {
      configMgr.save({
        policies: {
          team: [
            {
              name: 'secret-scan',
              enforcement: 'block',
              reason: 'Prevent leaks',
              owner: '@security',
            },
          ],
        },
      });

      const doc = configMgr.generatePoliciesDoc();
      expect(doc).toContain('## Active Policies');
      expect(doc).toContain('secret-scan');
      expect(doc).toContain('block');
      expect(doc).toContain('Prevent leaks');
      expect(doc).toContain('@security');
    });

    it('should include inactive policies section', () => {
      configMgr.save({
        policies: {
          team: [{ name: 'off-policy', enforcement: 'off' }],
        },
      });

      const doc = configMgr.generatePoliciesDoc();
      expect(doc).toContain('## Pending / Disabled Policies');
      expect(doc).toContain('off-policy');
    });

    it('should include org source info', () => {
      configMgr.save({
        policies: {
          org: { source: 'git@github.com:acme/policies.git', ref: 'v1.0.0' },
        },
      });

      const doc = configMgr.generatePoliciesDoc();
      expect(doc).toContain('## Org Source');
      expect(doc).toContain('git@github.com:acme/policies.git');
      expect(doc).toContain('v1.0.0');
    });

    it('should include non-expired announcements', () => {
      const futureDate = new Date(Date.now() + 365 * 24 * 60 * 60 * 1000)
        .toISOString()
        .split('T')[0];
      configMgr.save({
        policies: {},
        announcements: [{ message: 'New policy incoming', level: 'info', expires: futureDate }],
      });

      const doc = configMgr.generatePoliciesDoc();
      expect(doc).toContain('## Announcements');
      expect(doc).toContain('New policy incoming');
    });

    it('should exclude expired announcements', () => {
      configMgr.save({
        policies: {},
        announcements: [{ message: 'Old news', level: 'info', expires: '2020-01-01' }],
      });

      const doc = configMgr.generatePoliciesDoc();
      expect(doc).not.toContain('## Announcements');
    });

    it('should include generation date', () => {
      configMgr.save({ policies: {} });
      const today = new Date().toISOString().split('T')[0];

      const doc = configMgr.generatePoliciesDoc();
      expect(doc).toContain(`Generated on ${today}`);
    });
  });
});

describe('Starter Profiles', () => {
  describe('selectStarterProfile()', () => {
    it('should return monorepo profile for any monorepo type', () => {
      const profile = selectStarterProfile('react', 'nx');
      expect(profile.name).toBe('monorepo');
    });

    it('should return monorepo profile for turborepo', () => {
      const profile = selectStarterProfile('unknown', 'turborepo');
      expect(profile.name).toBe('monorepo');
    });

    it('should return fullstack profile for nextjs', () => {
      const profile = selectStarterProfile('nextjs', 'none');
      expect(profile.name).toBe('fullstack');
    });

    it('should return web-frontend for react', () => {
      const profile = selectStarterProfile('react', 'none');
      expect(profile.name).toBe('web-frontend');
    });

    it('should return web-frontend for vue', () => {
      const profile = selectStarterProfile('vue', 'none');
      expect(profile.name).toBe('web-frontend');
    });

    it('should return web-frontend for svelte', () => {
      const profile = selectStarterProfile('svelte', 'none');
      expect(profile.name).toBe('web-frontend');
    });

    it('should return web-frontend for angular', () => {
      const profile = selectStarterProfile('angular', 'none');
      expect(profile.name).toBe('web-frontend');
    });

    it('should return web-backend for express', () => {
      const profile = selectStarterProfile('express', 'none');
      expect(profile.name).toBe('web-backend');
    });

    it('should return web-backend for nestjs', () => {
      const profile = selectStarterProfile('nestjs', 'none');
      expect(profile.name).toBe('web-backend');
    });

    it('should return library for node', () => {
      const profile = selectStarterProfile('node', 'none');
      expect(profile.name).toBe('library');
    });

    it('should return generic for unknown framework', () => {
      const profile = selectStarterProfile('unknown', 'none');
      expect(profile.name).toBe('generic');
    });
  });

  describe('getStarterProfile()', () => {
    it('should return the named profile', () => {
      const profile = getStarterProfile('web-frontend');
      expect(profile.name).toBe('web-frontend');
      expect(profile.policies.length).toBeGreaterThan(0);
    });
  });

  describe('getAllStarterProfiles()', () => {
    it('should return all 6 profiles', () => {
      const profiles = getAllStarterProfiles();
      expect(profiles).toHaveLength(6);

      const names = profiles.map((p) => p.name);
      expect(names).toContain('web-frontend');
      expect(names).toContain('web-backend');
      expect(names).toContain('fullstack');
      expect(names).toContain('library');
      expect(names).toContain('monorepo');
      expect(names).toContain('generic');
    });

    it('should have secret-scan in every profile', () => {
      const profiles = getAllStarterProfiles();
      for (const profile of profiles) {
        const hasSecretScan = profile.policies.some((p) => p.name === 'secret-scan');
        expect(hasSecretScan).toBe(true);
      }
    });

    it('should have descriptions for all profiles', () => {
      const profiles = getAllStarterProfiles();
      for (const profile of profiles) {
        expect(profile.description).toBeTruthy();
      }
    });
  });
});
