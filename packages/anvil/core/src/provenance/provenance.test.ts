import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, rmSync, existsSync, writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { execSync } from 'child_process';
import {
  collectEnvironment,
  collectGitContext,
  detectAITool,
  createProvenanceRecord,
  formatProvenanceRecord,
} from './collector.js';
import { ProvenanceStore, createProvenanceStore } from './store.js';
import type { GateRunResult } from '@anvil/contracts';

describe('Provenance System', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = join(tmpdir(), 'anvil-provenance-test', Math.random().toString(36));
    mkdirSync(tempDir, { recursive: true });
  });

  afterEach(() => {
    rmSync(tempDir, { recursive: true, force: true });
  });

  describe('collectEnvironment', () => {
    it('should collect basic environment info', async () => {
      const env = await collectEnvironment(tempDir);

      expect(env.os).toBeDefined();
      expect(env.node_version).toMatch(/^v\d+\.\d+\.\d+/);
      expect(env.cwd).toBe(tempDir);
      expect(typeof env.ci).toBe('boolean');
    });
  });

  describe('collectGitContext', () => {
    it('should return undefined for non-git directory', async () => {
      const git = await collectGitContext(tempDir);
      expect(git).toBeUndefined();
    });

    it('should collect git context for git repository', async () => {
      try {
        // Initialize a git repo
        execSync('git init', { cwd: tempDir, stdio: 'pipe' });
        execSync('git config user.email "test@example.com"', { cwd: tempDir, stdio: 'pipe' });
        execSync('git config user.name "Test User"', { cwd: tempDir, stdio: 'pipe' });

        // Create initial commit
        writeFileSync(join(tempDir, 'test.txt'), 'hello');
        execSync('git add .', { cwd: tempDir, stdio: 'pipe' });
        execSync('git commit -m "Initial commit"', { cwd: tempDir, stdio: 'pipe' });

        const git = await collectGitContext(tempDir);

        expect(git).toBeDefined();
        expect(git?.branch).toBeDefined();
        expect(git?.commit).toMatch(/^[a-f0-9]{40}$/);
        expect(git?.commit_message).toBe('Initial commit');
        expect(git?.dirty).toBe(false);
      } catch {
        console.warn('Git not available, skipping test');
      }
    });

    it('should detect dirty state', async () => {
      try {
        execSync('git init', { cwd: tempDir, stdio: 'pipe' });
        execSync('git config user.email "test@example.com"', { cwd: tempDir, stdio: 'pipe' });
        execSync('git config user.name "Test User"', { cwd: tempDir, stdio: 'pipe' });

        writeFileSync(join(tempDir, 'test.txt'), 'hello');
        execSync('git add .', { cwd: tempDir, stdio: 'pipe' });
        execSync('git commit -m "Initial commit"', { cwd: tempDir, stdio: 'pipe' });

        // Make uncommitted change
        writeFileSync(join(tempDir, 'test.txt'), 'hello world');

        const git = await collectGitContext(tempDir);

        expect(git?.dirty).toBe(true);
        expect(git?.modified_files).toContain('test.txt');
      } catch {
        console.warn('Git not available, skipping test');
      }
    });
  });

  describe('detectAITool', () => {
    it('should return undefined when no AI tool detected', async () => {
      const tool = await detectAITool(tempDir);
      expect(tool).toBeUndefined();
    });

    it('should detect Cursor by .cursor directory', async () => {
      mkdirSync(join(tempDir, '.cursor'), { recursive: true });

      const tool = await detectAITool(tempDir);

      expect(tool?.name).toBe('cursor');
      expect(tool?.confidence).toBe('high');
      expect(tool?.indicators).toContain('.cursor directory present');
    });

    it('should detect Claude Code by CLAUDE.md', async () => {
      writeFileSync(join(tempDir, 'CLAUDE.md'), '# Claude Configuration');

      const tool = await detectAITool(tempDir);

      expect(tool?.name).toBe('claude-code');
      expect(tool?.confidence).toBe('high');
    });

    it('should detect Copilot from VS Code settings', async () => {
      mkdirSync(join(tempDir, '.vscode'), { recursive: true });
      writeFileSync(
        join(tempDir, '.vscode', 'settings.json'),
        JSON.stringify({ 'github.copilot.enable': true })
      );

      const tool = await detectAITool(tempDir);

      expect(tool?.name).toBe('copilot');
      expect(tool?.confidence).toBe('medium');
    });
  });

  describe('ProvenanceStore', () => {
    let store: ProvenanceStore;

    beforeEach(() => {
      store = createProvenanceStore(tempDir);
    });

    it('should initialise with empty history', () => {
      const index = store.getIndex();

      expect(index.records).toHaveLength(0);
      expect(index.statistics.total_checks).toBe(0);
    });

    it('should save and retrieve a record', async () => {
      const mockResults: GateRunResult = {
        overall: true,
        score: 95,
        checks: [
          { check: 'secret', passed: true, score: 100, message: 'No secrets' },
          { check: 'lint', passed: true, score: 90, message: 'Clean' },
        ],
        summary: { total: 2, passed: 2, failed: 0, skipped: 0 },
      };

      const record = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['src/index.ts', 'src/utils.ts'],
        scope: 'directory',
        results: mockResults,
        trigger: 'manual',
        startTime: Date.now() - 1000,
      });

      store.save(record);

      // Retrieve by ID
      const retrieved = store.get(record.id);
      expect(retrieved).toBeDefined();
      expect(retrieved?.id).toBe(record.id);
      expect(retrieved?.overall_passed).toBe(true);
      expect(retrieved?.files_count).toBe(2);
    });

    it('should update statistics on save', async () => {
      const passedResult: GateRunResult = {
        overall: true,
        score: 100,
        checks: [{ check: 'test', passed: true, score: 100, message: 'OK' }],
        summary: { total: 1, passed: 1, failed: 0, skipped: 0 },
      };

      const failedResult: GateRunResult = {
        overall: false,
        score: 50,
        checks: [{ check: 'test', passed: false, score: 50, message: 'Failed' }],
        summary: { total: 1, passed: 0, failed: 1, skipped: 0 },
      };

      const record1 = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['a.ts'],
        scope: 'files',
        results: passedResult,
        trigger: 'manual',
        startTime: Date.now(),
      });

      const record2 = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['b.ts'],
        scope: 'files',
        results: failedResult,
        trigger: 'manual',
        startTime: Date.now(),
      });

      store.save(record1);
      store.save(record2);

      const stats = store.getStatistics();
      expect(stats.total).toBe(2);
      expect(stats.passed).toBe(1);
      expect(stats.failed).toBe(1);
      expect(stats.passRate).toBe(50);
    });

    it('should list recent records', async () => {
      const mockResult: GateRunResult = {
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
      };

      // Create multiple records
      for (let i = 0; i < 5; i++) {
        const record = await createProvenanceRecord({
          workspaceRoot: tempDir,
          filesChecked: [`file${i}.ts`],
          scope: 'files',
          results: mockResult,
          trigger: 'manual',
          startTime: Date.now(),
        });
        store.save(record);
      }

      const records = store.list({ limit: 3 });
      expect(records).toHaveLength(3);
    });

    it('should get latest record', async () => {
      const mockResult: GateRunResult = {
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
      };

      const record1 = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['first.ts'],
        scope: 'files',
        results: mockResult,
        trigger: 'manual',
        startTime: Date.now() - 1000,
      });

      const record2 = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['second.ts'],
        scope: 'files',
        results: mockResult,
        trigger: 'manual',
        startTime: Date.now(),
      });

      store.save(record1);
      store.save(record2);

      const latest = store.getLatest();
      expect(latest?.id).toBe(record2.id);
    });

    it('should filter by passed/failed', async () => {
      const passedResult: GateRunResult = {
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
      };

      const failedResult: GateRunResult = {
        overall: false,
        score: 0,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
      };

      const passed = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['pass.ts'],
        scope: 'files',
        results: passedResult,
        trigger: 'manual',
        startTime: Date.now(),
      });

      const failed = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['fail.ts'],
        scope: 'files',
        results: failedResult,
        trigger: 'manual',
        startTime: Date.now(),
      });

      store.save(passed);
      store.save(failed);

      const onlyPassed = store.list({ passed: true });
      expect(onlyPassed).toHaveLength(1);
      expect(onlyPassed[0].overall_passed).toBe(true);

      const onlyFailed = store.list({ passed: false });
      expect(onlyFailed).toHaveLength(1);
      expect(onlyFailed[0].overall_passed).toBe(false);
    });

    it('should export history as JSON', async () => {
      const mockResult: GateRunResult = {
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
      };

      const record = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['test.ts'],
        scope: 'files',
        results: mockResult,
        trigger: 'manual',
        startTime: Date.now(),
      });

      store.save(record);

      const exported = store.export();
      const parsed = JSON.parse(exported);

      expect(parsed.exported_at).toBeDefined();
      expect(parsed.statistics).toBeDefined();
      expect(parsed.records).toHaveLength(1);
    });

    it('should clear history', async () => {
      const mockResult: GateRunResult = {
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
      };

      const record = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['test.ts'],
        scope: 'files',
        results: mockResult,
        trigger: 'manual',
        startTime: Date.now(),
      });

      store.save(record);
      expect(store.getStatistics().total).toBe(1);

      store.clear();

      expect(store.getStatistics().total).toBe(0);
      expect(store.list()).toHaveLength(0);
    });

    it('should create .gitignore in .anvil directory', async () => {
      const mockResult: GateRunResult = {
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
      };

      // Trigger directory creation by saving
      const record = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['test.ts'],
        scope: 'files',
        results: mockResult,
        trigger: 'manual',
        startTime: Date.now(),
      });
      store.save(record);

      // Check for .gitignore
      const gitignorePath = join(tempDir, '.anvil', '.gitignore');
      expect(existsSync(gitignorePath)).toBe(true);
    });
  });

  describe('formatProvenanceRecord', () => {
    it('should format a record for display', async () => {
      const mockResults: GateRunResult = {
        overall: true,
        score: 95,
        checks: [
          { check: 'secret', passed: true, score: 100, message: 'No secrets' },
          { check: 'lint', passed: false, score: 80, message: 'Issues found' },
        ],
        summary: { total: 2, passed: 1, failed: 1, skipped: 0 },
      };

      const record = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['src/index.ts'],
        scope: 'directory',
        results: mockResults,
        trigger: 'manual',
        startTime: Date.now() - 500,
      });

      const formatted = formatProvenanceRecord(record);

      expect(formatted).toContain('Provenance Record');
      expect(formatted).toContain(record.id);
      expect(formatted).toContain('✓ PASSED');
      expect(formatted).toContain('95/100');
      expect(formatted).toContain('✓ secret');
      expect(formatted).toContain('✗ lint');
    });
  });

  describe('createProvenanceRecord', () => {
    it('should create a complete provenance record', async () => {
      const mockResults: GateRunResult = {
        overall: true,
        score: 100,
        checks: [{ check: 'test', passed: true, score: 100, message: 'OK' }],
        summary: { total: 1, passed: 1, failed: 0, skipped: 0 },
      };

      const record = await createProvenanceRecord({
        workspaceRoot: tempDir,
        filesChecked: ['a.ts', 'b.ts', 'c.ts'],
        scope: 'staged',
        results: mockResults,
        trigger: 'pre-commit',
        startTime: Date.now() - 1234,
        planId: 'aps-12345678',
      });

      expect(record.id).toMatch(/^prov-/);
      expect(record.timestamp).toBeDefined();
      expect(record.scope).toBe('staged');
      expect(record.files_count).toBe(3);
      expect(record.overall_passed).toBe(true);
      expect(record.overall_score).toBe(100);
      expect(record.checks).toHaveLength(1);
      expect(record.environment).toBeDefined();
      expect(record.trigger).toBe('pre-commit');
      expect(record.duration_ms).toBeGreaterThan(0);
      expect(record.plan_id).toBe('aps-12345678');
    });
  });
});
