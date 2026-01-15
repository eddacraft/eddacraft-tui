import { describe, it, expect } from 'vitest';
import {
  loadCommandSafetyRules,
  resolveCommandSafetyConfig,
  DEFAULT_COMMAND_SAFETY_CONFIG,
} from './command-safety-config.js';
import type { CommandSafetyConfig } from '../rules/types.js';

describe('loadCommandSafetyRules', () => {
  it('returns default rules when no config provided', () => {
    const rules = loadCommandSafetyRules({});
    expect(rules.length).toBeGreaterThan(30);
    expect(rules.some((r) => r.id === 'git-reset-hard')).toBe(true);
    expect(rules.some((r) => r.id === 'rm-rf-root')).toBe(true);
  });

  it('disables rules by ID', () => {
    const config: CommandSafetyConfig = {
      rules: {
        disabled: ['git-reset-hard', 'rm-rf-root'],
      },
    };
    const rules = loadCommandSafetyRules(config);
    expect(rules.some((r) => r.id === 'git-reset-hard')).toBe(false);
    expect(rules.some((r) => r.id === 'rm-rf-root')).toBe(false);
    expect(rules.some((r) => r.id === 'git-push-force')).toBe(true);
  });

  it('overrides rule action', () => {
    const config: CommandSafetyConfig = {
      rules: {
        overrides: [{ id: 'git-push-force', action: 'warn' }],
      },
    };
    const rules = loadCommandSafetyRules(config);
    const rule = rules.find((r) => r.id === 'git-push-force');
    expect(rule?.action).toBe('warn');
  });

  it('overrides rule severity', () => {
    const config: CommandSafetyConfig = {
      rules: {
        overrides: [{ id: 'git-clean-force', severity: 'error' }],
      },
    };
    const rules = loadCommandSafetyRules(config);
    const rule = rules.find((r) => r.id === 'git-clean-force');
    expect(rule?.severity).toBe('error');
  });

  it('removes rule with action=disable override', () => {
    const config: CommandSafetyConfig = {
      rules: {
        overrides: [{ id: 'git-reset-hard', action: 'disable' }],
      },
    };
    const rules = loadCommandSafetyRules(config);
    expect(rules.some((r) => r.id === 'git-reset-hard')).toBe(false);
  });

  it('adds custom rules', () => {
    const config: CommandSafetyConfig = {
      rules: {
        custom: [
          {
            id: 'custom-rule',
            category: 'custom',
            command: 'custom-cmd',
            action: 'block',
            severity: 'error',
            reason: 'Custom reason',
          },
        ],
      },
    };
    const rules = loadCommandSafetyRules(config);
    const customRule = rules.find((r) => r.id === 'custom-rule');
    expect(customRule).toBeDefined();
    expect(customRule?.command).toBe('custom-cmd');
  });
});

describe('resolveCommandSafetyConfig', () => {
  it('returns defaults when no config provided', () => {
    const resolved = resolveCommandSafetyConfig();
    expect(resolved.enabled).toBe(true);
    expect(resolved.strict).toBe(false);
    expect(resolved.workingDirectory.allowDeleteInCwd).toBe(false);
    expect(resolved.output.verbose).toBe(true);
  });

  it('merges user config with defaults', () => {
    const resolved = resolveCommandSafetyConfig({
      enabled: false,
      strict: true,
      output: { verbose: false },
    });
    expect(resolved.enabled).toBe(false);
    expect(resolved.strict).toBe(true);
    expect(resolved.output.verbose).toBe(false);
    expect(resolved.output.showSuggestions).toBe(true);
  });

  it('merges working directory config', () => {
    const resolved = resolveCommandSafetyConfig({
      workingDirectory: { allowDeleteInCwd: true },
    });
    expect(resolved.workingDirectory.allowDeleteInCwd).toBe(true);
    expect(resolved.workingDirectory.tempDirPatterns).toContain('/tmp');
  });
});

describe('DEFAULT_COMMAND_SAFETY_CONFIG', () => {
  it('has correct default values', () => {
    expect(DEFAULT_COMMAND_SAFETY_CONFIG.enabled).toBe(true);
    expect(DEFAULT_COMMAND_SAFETY_CONFIG.strict).toBe(false);
    expect(DEFAULT_COMMAND_SAFETY_CONFIG.rules.length).toBeGreaterThan(30);
  });
});
