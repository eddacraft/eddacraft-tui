import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as os from 'node:os';
import { SuppressionStore } from './store.js';
import type { ParsedSuppression } from './parser.js';

describe('SuppressionStore', () => {
  let tempDir: string;
  let anvilDir: string;

  beforeEach(async () => {
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'anvil-store-test-'));
    anvilDir = path.join(tempDir, '.anvil');
  });

  afterEach(async () => {
    await fs.rm(tempDir, { recursive: true, force: true });
  });

  describe('load', () => {
    it('creates empty store when file does not exist', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      expect(store.isLoaded).toBe(true);
      expect(store.count).toBe(0);
    });

    it('loads existing suppressions from file', async () => {
      await fs.mkdir(anvilDir, { recursive: true });
      const data = {
        version: 1,
        suppressions: [
          {
            id: 'test.ts:10:AP-001',
            pattern_id: 'AP-001',
            file: 'test.ts',
            line: 10,
            reason: 'Legacy code',
            timestamp: '2025-01-01T00:00:00.000Z',
            scope: 'line',
          },
        ],
        lastUpdated: '2025-01-01T00:00:00.000Z',
      };
      await fs.writeFile(path.join(anvilDir, 'suppressions.json'), JSON.stringify(data));

      const store = new SuppressionStore(anvilDir);
      await store.load();

      expect(store.count).toBe(1);
      expect(store.getAll()[0].pattern_id).toBe('AP-001');
    });

    it('handles corrupted file gracefully', async () => {
      await fs.mkdir(anvilDir, { recursive: true });
      await fs.writeFile(path.join(anvilDir, 'suppressions.json'), 'not valid json');

      const store = new SuppressionStore(anvilDir);
      await store.load();

      expect(store.isLoaded).toBe(true);
      expect(store.count).toBe(0);
    });
  });

  describe('save', () => {
    it('creates directory and saves file', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'test.ts:5:AP-001',
        pattern_id: 'AP-001',
        file: 'test.ts',
        line: 5,
        reason: 'Test reason',
        timestamp: new Date().toISOString(),
        scope: 'line',
      });

      await store.save();

      const content = await fs.readFile(path.join(anvilDir, 'suppressions.json'), 'utf-8');
      const data = JSON.parse(content);

      expect(data.version).toBe(1);
      expect(data.suppressions).toHaveLength(1);
      expect(data.suppressions[0].pattern_id).toBe('AP-001');
    });
  });

  describe('add', () => {
    it('adds new suppression', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 1,
        reason: 'Test',
        timestamp: new Date().toISOString(),
        scope: 'line',
      });

      expect(store.count).toBe(1);
    });

    it('updates existing suppression with same id', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 1,
        reason: 'Original',
        timestamp: new Date().toISOString(),
        scope: 'line',
      });

      store.add({
        id: 'file.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 1,
        reason: 'Updated',
        timestamp: new Date().toISOString(),
        scope: 'line',
      });

      expect(store.count).toBe(1);
      expect(store.getAll()[0].reason).toBe('Updated');
    });
  });

  describe('remove', () => {
    it('removes existing suppression', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 1,
        reason: 'Test',
        timestamp: new Date().toISOString(),
        scope: 'line',
      });

      const removed = store.remove('file.ts:1:AP-001');

      expect(removed).toBe(true);
      expect(store.count).toBe(0);
    });

    it('returns false for non-existent id', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      const removed = store.remove('non-existent');

      expect(removed).toBe(false);
    });
  });

  describe('isSuppressed', () => {
    it('returns match for suppressed warning', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:10:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 10,
        reason: 'Test',
        timestamp: new Date().toISOString(),
        scope: 'line',
      });

      const match = store.isSuppressed('AP-001', 'file.ts', 10);

      expect(match).not.toBeNull();
      expect(match?.record.pattern_id).toBe('AP-001');
      expect(match?.isExpired).toBe(false);
    });

    it('returns null for non-matching file', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:10:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 10,
        reason: 'Test',
        timestamp: new Date().toISOString(),
        scope: 'line',
      });

      const match = store.isSuppressed('AP-001', 'other.ts', 10);

      expect(match).toBeNull();
    });

    it('returns null for non-matching pattern', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:10:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 10,
        reason: 'Test',
        timestamp: new Date().toISOString(),
        scope: 'line',
      });

      const match = store.isSuppressed('AP-002', 'file.ts', 10);

      expect(match).toBeNull();
    });

    it('matches file scope to any line', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 1,
        reason: 'File-level',
        timestamp: new Date().toISOString(),
        scope: 'file',
      });

      const match = store.isSuppressed('AP-001', 'file.ts', 100);

      expect(match).not.toBeNull();
    });

    it('matches statement scope to next line', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:10:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 10,
        reason: 'Statement',
        timestamp: new Date().toISOString(),
        scope: 'statement',
      });

      expect(store.isSuppressed('AP-001', 'file.ts', 11)).not.toBeNull();
      expect(store.isSuppressed('AP-001', 'file.ts', 10)).toBeNull();
      expect(store.isSuppressed('AP-001', 'file.ts', 12)).toBeNull();
    });

    it('identifies expired suppression', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      const record = {
        id: 'file.ts:10:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 10,
        reason: 'Test',
        timestamp: new Date().toISOString(),
        scope: 'file' as const,
        expires_at: '2020-01-01T00:00:00.000Z',
      };

      store.add(record);

      const match = store.isSuppressed('AP-001', 'file.ts', 10, new Date('2025-01-01'));

      expect(match).not.toBeNull();
      expect(match?.isExpired).toBe(true);
    });
  });

  describe('getExpired', () => {
    it('returns expired suppressions', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 1,
        reason: 'Permanent',
        timestamp: new Date().toISOString(),
        scope: 'file',
      });

      const expiredRecord = {
        id: 'file.ts:2:AP-002',
        pattern_id: 'AP-002',
        file: 'file.ts',
        line: 2,
        reason: 'Expired',
        timestamp: new Date().toISOString(),
        scope: 'file' as const,
        expires_at: '2020-01-01T00:00:00.000Z',
      };
      store.add(expiredRecord);

      const expired = store.getExpired(new Date('2025-01-01'));

      expect(expired).toHaveLength(1);
      expect(expired[0].pattern_id).toBe('AP-002');
    });
  });

  describe('pruneExpired', () => {
    it('removes expired suppressions', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'file.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'file.ts',
        line: 1,
        reason: 'Permanent',
        timestamp: new Date().toISOString(),
        scope: 'file',
      });

      const expiredRecord = {
        id: 'file.ts:2:AP-002',
        pattern_id: 'AP-002',
        file: 'file.ts',
        line: 2,
        reason: 'Expired',
        timestamp: new Date().toISOString(),
        scope: 'file' as const,
        expires_at: '2020-01-01T00:00:00.000Z',
      };
      store.add(expiredRecord);

      const pruned = store.pruneExpired(new Date('2025-01-01'));

      expect(pruned).toBe(1);
      expect(store.count).toBe(1);
      expect(store.getAll()[0].pattern_id).toBe('AP-001');
    });
  });

  describe('createRecordFromParsed', () => {
    it('creates record from parsed suppression', async () => {
      const store = new SuppressionStore(anvilDir);

      const parsed: ParsedSuppression = {
        warningId: 'AP-001',
        reason: 'Test reason',
        line: 10,
        scope: 'statement',
        raw: '// @anvil-ignore AP-001: Test reason',
      };

      const record = store.createRecordFromParsed(parsed, 'src/file.ts');

      expect(record.id).toBe('src/file.ts:10:AP-001');
      expect(record.pattern_id).toBe('AP-001');
      expect(record.file).toBe('src/file.ts');
      expect(record.line).toBe(10);
      expect(record.reason).toBe('Test reason');
      expect(record.scope).toBe('statement');
    });

    it('includes git commit when provided', async () => {
      const store = new SuppressionStore(anvilDir);

      const parsed: ParsedSuppression = {
        warningId: 'AP-001',
        reason: 'Test',
        line: 1,
        scope: 'file',
        raw: '',
      };

      const record = store.createRecordFromParsed(parsed, 'file.ts', 'abc123');

      expect(record.commit).toBe('abc123');
    });

    it('includes expires_at for time-boxed suppression', async () => {
      const store = new SuppressionStore(anvilDir);

      const parsed: ParsedSuppression = {
        warningId: 'AP-001',
        reason: 'Temporary',
        expiresAt: new Date('2025-06-01'),
        line: 1,
        scope: 'file',
        raw: '',
      };

      const record = store.createRecordFromParsed(parsed, 'file.ts');

      expect((record as Record<string, unknown>)['expires_at']).toBe('2025-06-01T00:00:00.000Z');
    });
  });

  describe('getByFile', () => {
    it('returns suppressions for specific file', async () => {
      const store = new SuppressionStore(anvilDir);
      await store.load();

      store.add({
        id: 'a.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'a.ts',
        line: 1,
        reason: 'A',
        timestamp: new Date().toISOString(),
        scope: 'file',
      });

      store.add({
        id: 'b.ts:1:AP-001',
        pattern_id: 'AP-001',
        file: 'b.ts',
        line: 1,
        reason: 'B',
        timestamp: new Date().toISOString(),
        scope: 'file',
      });

      const result = store.getByFile('a.ts');

      expect(result).toHaveLength(1);
      expect(result[0].file).toBe('a.ts');
    });
  });
});
