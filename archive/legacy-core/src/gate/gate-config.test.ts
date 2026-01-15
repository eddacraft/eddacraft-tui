import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { GateConfigManager } from './gate-config.js';
import { writeFileSync, rmSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

describe('GateConfigManager', () => {
  let configManager: GateConfigManager;
  let tempDir: string;

  beforeEach(() => {
    tempDir = join(tmpdir(), 'anvil-test', Math.random().toString(36));
    mkdirSync(tempDir, { recursive: true });
    configManager = new GateConfigManager(tempDir);
  });

  afterEach(() => {
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('should return default config when no config file exists', () => {
    const config = configManager.loadConfig();

    expect(config.version).toBe(1);
    expect(config.checks).toHaveLength(6);
    expect(config.thresholds.overall_score).toBe(80);
    expect(config.checks.map((c) => c.name)).toContain('eslint');
    expect(config.checks.map((c) => c.name)).toContain('coverage');
    expect(config.checks.map((c) => c.name)).toContain('secret');
    expect(config.checks.map((c) => c.name)).toContain('dependency');
    expect(config.checks.map((c) => c.name)).toContain('policy');
  });

  it('should load existing config file', () => {
    const customConfig = {
      version: 1,
      checks: [
        {
          name: 'custom-check',
          description: 'Custom check',
          enabled: true,
          config: { threshold: 90 },
        },
      ],
      thresholds: {
        overall_score: 90,
      },
    };

    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify(customConfig, null, 2));

    // Create new manager after writing file so it finds the config
    const newManager = new GateConfigManager(tempDir);
    const config = newManager.loadConfig();

    expect(config.thresholds.overall_score).toBe(90);
    expect(config.checks).toHaveLength(1);
    expect(config.checks[0].name).toBe('custom-check');
  });

  it('should save config to file', () => {
    const config = configManager.getDefaultConfig();
    config.thresholds.overall_score = 95;

    configManager.saveConfig(config);

    expect(existsSync(join(tempDir, '.anvilrc'))).toBe(true);

    const loadedConfig = configManager.loadConfig();
    expect(loadedConfig.thresholds.overall_score).toBe(95);
  });

  it('should update existing check', () => {
    const config = configManager.loadConfig();
    configManager.saveConfig(config);

    configManager.updateCheck('eslint', { enabled: false, config: { min_score: 95 } });

    const updatedConfig = configManager.loadConfig();
    const eslintCheck = updatedConfig.checks.find((c) => c.name === 'eslint');

    expect(eslintCheck?.enabled).toBe(false);
    expect(eslintCheck?.config?.min_score).toBe(95);
  });

  it('should add new check if it does not exist', () => {
    configManager.updateCheck('new-check', {
      description: 'New check',
      enabled: true,
      config: { threshold: 85 },
    });

    const config = configManager.loadConfig();
    const newCheck = config.checks.find((c) => c.name === 'new-check');

    expect(newCheck).toBeDefined();
    expect(newCheck?.description).toBe('New check');
    expect(newCheck?.enabled).toBe(true);
  });

  it('should enable check', () => {
    const config = configManager.loadConfig();
    config.checks[0].enabled = false;
    configManager.saveConfig(config);

    configManager.enableCheck('eslint');

    const updatedConfig = configManager.loadConfig();
    const eslintCheck = updatedConfig.checks.find((c) => c.name === 'eslint');
    expect(eslintCheck?.enabled).toBe(true);
  });

  it('should disable check', () => {
    configManager.disableCheck('eslint');

    const config = configManager.loadConfig();
    const eslintCheck = config.checks.find((c) => c.name === 'eslint');
    expect(eslintCheck?.enabled).toBe(false);
  });

  it('should handle invalid JSON gracefully', () => {
    writeFileSync(join(tempDir, '.anvilrc'), 'invalid json');

    // Need to create a new manager to pick up the file
    const newManager = new GateConfigManager(tempDir);

    // Mock console.warn to prevent stderr output during test
    const originalWarn = console.warn;
    const mockWarn = vi.fn();
    console.warn = mockWarn;

    try {
      const config = newManager.loadConfig();

      // Should fall back to default config
      expect(config.version).toBe(1);
      expect(config.checks).toHaveLength(6);

      // Verify that a warning was logged
      expect(mockWarn).toHaveBeenCalled();
      expect(mockWarn.mock.calls[0][0]).toContain('Failed to load gate config');
    } finally {
      // Restore original console.warn
      console.warn = originalWarn;
    }
  });

  it('should validate and normalize config', () => {
    const invalidConfig = {
      // Missing version, checks, thresholds
      someField: 'value',
    };

    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify(invalidConfig));

    // Need to create a new manager to pick up the file
    const newManager = new GateConfigManager(tempDir);
    const config = newManager.loadConfig();

    expect(config.version).toBe(1);
    expect(config.checks).toEqual([]);
    expect(config.thresholds.overall_score).toBe(80);
  });

  describe('multiple config locations', () => {
    it('should load config from .anvilrc first', () => {
      const anvilrcConfig = { version: 1, checks: [], thresholds: { overall_score: 90 } };
      const anvilDirConfig = { version: 1, checks: [], thresholds: { overall_score: 70 } };

      // Create both config files
      writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify(anvilrcConfig));
      mkdirSync(join(tempDir, '.anvil'), { recursive: true });
      writeFileSync(join(tempDir, '.anvil', 'config.json'), JSON.stringify(anvilDirConfig));

      const newManager = new GateConfigManager(tempDir);
      const config = newManager.loadConfig();

      // Should prefer .anvilrc
      expect(config.thresholds.overall_score).toBe(90);
    });

    it('should load config from .anvil/config.json if .anvilrc does not exist', () => {
      const anvilDirConfig = { version: 1, checks: [], thresholds: { overall_score: 70 } };

      // Create only .anvil/config.json
      mkdirSync(join(tempDir, '.anvil'), { recursive: true });
      writeFileSync(join(tempDir, '.anvil', 'config.json'), JSON.stringify(anvilDirConfig));

      const newManager = new GateConfigManager(tempDir);
      const config = newManager.loadConfig();

      expect(config.thresholds.overall_score).toBe(70);
    });

    it('should return correct config path', () => {
      expect(configManager.getConfigPath()).toBe(join(tempDir, '.anvilrc'));

      // With existing config
      writeFileSync(
        join(tempDir, '.anvilrc'),
        JSON.stringify({ version: 1, checks: [], thresholds: { overall_score: 80 } })
      );
      const newManager = new GateConfigManager(tempDir);
      expect(newManager.getConfigPath()).toBe(join(tempDir, '.anvilrc'));
    });

    it('should list all config locations', () => {
      const locations = configManager.getConfigLocations();
      expect(locations).toContain(join(tempDir, '.anvilrc'));
      expect(locations).toContain(join(tempDir, '.anvil', 'config.json'));
    });
  });

  describe('loadConfigWithDetails', () => {
    it('should return isDefault true when no config exists', () => {
      const result = configManager.loadConfigWithDetails();
      expect(result.isDefault).toBe(true);
      expect(result.path).toBeNull();
      expect(result.errors).toHaveLength(0);
    });

    it('should return path and isDefault false when config exists', () => {
      writeFileSync(
        join(tempDir, '.anvilrc'),
        JSON.stringify({ version: 1, checks: [], thresholds: { overall_score: 80 } })
      );
      const newManager = new GateConfigManager(tempDir);

      const result = newManager.loadConfigWithDetails();
      expect(result.isDefault).toBe(false);
      expect(result.path).toBe(join(tempDir, '.anvilrc'));
      expect(result.errors).toHaveLength(0);
    });

    it('should return validation errors for invalid config', () => {
      const invalidConfig = { someField: 'value' };
      writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify(invalidConfig));
      const newManager = new GateConfigManager(tempDir);

      const result = newManager.loadConfigWithDetails();
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors.some((e) => e.includes('version'))).toBe(true);
    });
  });
});
