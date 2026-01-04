import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as os from 'node:os';
import {
  saveSnapshot,
  loadSnapshot,
  listSnapshots,
  deleteSnapshot,
  snapshotExists,
  getLatestSnapshot,
  SnapshotStore,
  SNAPSHOTS_DIR,
  ANVIL_DIR,
} from './snapshot-storage.js';
import { createEmptySnapshot } from './snapshot-schema.js';

describe('SnapshotStorage', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await fs.mkdtemp(path.join(os.tmpdir(), 'anvil-drift-test-'));
  });

  afterEach(async () => {
    await fs.rm(testDir, { recursive: true, force: true });
  });

  describe('saveSnapshot', () => {
    it('should save snapshot with auto-generated filename', async () => {
      const snapshot = createEmptySnapshot();
      const filePath = await saveSnapshot(testDir, snapshot);

      expect(filePath).toContain(ANVIL_DIR);
      expect(filePath).toContain(SNAPSHOTS_DIR);
      expect(filePath).toMatch(/snapshot-.*\.json$/);

      const content = await fs.readFile(filePath, 'utf-8');
      const saved = JSON.parse(content);
      expect(saved.schema_version).toBe(snapshot.schema_version);
    });

    it('should save snapshot with custom name', async () => {
      const snapshot = createEmptySnapshot({ name: 'release-1.0' });
      const filePath = await saveSnapshot(testDir, snapshot, 'release-1.0');

      expect(filePath).toContain('snapshot-release-1-0.json');
    });

    it('should create snapshots directory if not exists', async () => {
      const snapshot = createEmptySnapshot();
      await saveSnapshot(testDir, snapshot);

      const snapshotsDir = path.join(testDir, ANVIL_DIR, SNAPSHOTS_DIR);
      const stat = await fs.stat(snapshotsDir);
      expect(stat.isDirectory()).toBe(true);
    });
  });

  describe('loadSnapshot', () => {
    it('should load saved snapshot by filename', async () => {
      const snapshot = createEmptySnapshot({ name: 'test' });
      snapshot.metrics.boundary_violations = 5;
      await saveSnapshot(testDir, snapshot, 'test');

      const loaded = await loadSnapshot(testDir, 'snapshot-test.json');

      expect(loaded).not.toBeNull();
      expect(loaded?.name).toBe('test');
      expect(loaded?.metrics.boundary_violations).toBe(5);
    });

    it('should load saved snapshot by name', async () => {
      const snapshot = createEmptySnapshot({ name: 'myrelease' });
      await saveSnapshot(testDir, snapshot, 'myrelease');

      const loaded = await loadSnapshot(testDir, 'myrelease');

      expect(loaded).not.toBeNull();
      expect(loaded?.name).toBe('myrelease');
    });

    it('should return null for non-existent snapshot', async () => {
      const loaded = await loadSnapshot(testDir, 'does-not-exist');
      expect(loaded).toBeNull();
    });

    it('should load snapshot by timestamp identifier (without snapshot- prefix)', async () => {
      const snapshot = createEmptySnapshot();
      snapshot.metrics.boundary_violations = 7;
      const filePath = await saveSnapshot(testDir, snapshot);

      const filenameMatch = filePath.match(/snapshot-(.+)\.json$/);
      expect(filenameMatch).not.toBeNull();
      const timestampPart = filenameMatch![1];

      const loaded = await loadSnapshot(testDir, timestampPart);

      expect(loaded).not.toBeNull();
      expect(loaded?.metrics.boundary_violations).toBe(7);
    });

    it('should load snapshot by timestamp from list output', async () => {
      const snapshot = createEmptySnapshot();
      snapshot.metrics.antipattern_count = 15;
      await saveSnapshot(testDir, snapshot);

      const list = await listSnapshots(testDir);
      expect(list).toHaveLength(1);

      const filenameFromList = list[0].filename;
      const timestampPart = filenameFromList.replace('snapshot-', '').replace('.json', '');

      const loaded = await loadSnapshot(testDir, timestampPart);

      expect(loaded).not.toBeNull();
      expect(loaded?.metrics.antipattern_count).toBe(15);
    });

    it('should distinguish timestamp from named snapshots', async () => {
      const namedSnapshot = createEmptySnapshot({ name: 'release' });
      namedSnapshot.metrics.boundary_violations = 100;
      await saveSnapshot(testDir, namedSnapshot, 'release');

      const timestampSnapshot = createEmptySnapshot();
      timestampSnapshot.metrics.boundary_violations = 200;
      await saveSnapshot(testDir, timestampSnapshot);

      const loadedNamed = await loadSnapshot(testDir, 'release');
      expect(loadedNamed?.metrics.boundary_violations).toBe(100);

      const list = await listSnapshots(testDir);
      const timestampFilename = list.find((s) => !s.name)?.filename;
      expect(timestampFilename).toBeDefined();

      const timestampPart = timestampFilename!.replace('snapshot-', '').replace('.json', '');
      const loadedTimestamp = await loadSnapshot(testDir, timestampPart);
      expect(loadedTimestamp?.metrics.boundary_violations).toBe(200);
    });
  });

  describe('listSnapshots', () => {
    it('should return empty array when no snapshots', async () => {
      const list = await listSnapshots(testDir);
      expect(list).toEqual([]);
    });

    it('should list all snapshots sorted by date', async () => {
      const older = createEmptySnapshot({ name: 'older' });
      older.created_at = '2025-01-01T00:00:00.000Z';
      await saveSnapshot(testDir, older, 'older');

      const newer = createEmptySnapshot({ name: 'newer' });
      newer.created_at = '2025-01-15T00:00:00.000Z';
      await saveSnapshot(testDir, newer, 'newer');

      const list = await listSnapshots(testDir);

      expect(list).toHaveLength(2);
      expect(list[0].name).toBe('newer');
      expect(list[1].name).toBe('older');
    });

    it('should include metrics in listing', async () => {
      const snapshot = createEmptySnapshot();
      snapshot.metrics.boundary_violations = 10;
      snapshot.metrics.antipattern_count = 20;
      await saveSnapshot(testDir, snapshot, 'test');

      const list = await listSnapshots(testDir);

      expect(list[0].metrics.boundary_violations).toBe(10);
      expect(list[0].metrics.antipattern_count).toBe(20);
    });
  });

  describe('deleteSnapshot', () => {
    it('should delete existing snapshot', async () => {
      const snapshot = createEmptySnapshot();
      await saveSnapshot(testDir, snapshot, 'to-delete');

      const deleted = await deleteSnapshot(testDir, 'to-delete');

      expect(deleted).toBe(true);
      expect(await snapshotExists(testDir, 'to-delete')).toBe(false);
    });

    it('should return false for non-existent snapshot', async () => {
      const deleted = await deleteSnapshot(testDir, 'does-not-exist');
      expect(deleted).toBe(false);
    });
  });

  describe('snapshotExists', () => {
    it('should return true for existing snapshot', async () => {
      const snapshot = createEmptySnapshot();
      await saveSnapshot(testDir, snapshot, 'exists');

      expect(await snapshotExists(testDir, 'exists')).toBe(true);
    });

    it('should return false for non-existent snapshot', async () => {
      expect(await snapshotExists(testDir, 'not-here')).toBe(false);
    });
  });

  describe('getLatestSnapshot', () => {
    it('should return null when no snapshots', async () => {
      const latest = await getLatestSnapshot(testDir);
      expect(latest).toBeNull();
    });

    it('should return most recent snapshot', async () => {
      const older = createEmptySnapshot({ name: 'older' });
      older.created_at = '2025-01-01T00:00:00.000Z';
      await saveSnapshot(testDir, older, 'older');

      const newest = createEmptySnapshot({ name: 'newest' });
      newest.created_at = '2025-01-20T00:00:00.000Z';
      await saveSnapshot(testDir, newest, 'newest');

      const latest = await getLatestSnapshot(testDir);

      expect(latest?.name).toBe('newest');
    });
  });
});

describe('SnapshotStore', () => {
  let testDir: string;
  let store: SnapshotStore;

  beforeEach(async () => {
    testDir = await fs.mkdtemp(path.join(os.tmpdir(), 'anvil-drift-store-'));
    store = new SnapshotStore(testDir);
  });

  afterEach(async () => {
    await fs.rm(testDir, { recursive: true, force: true });
  });

  it('should save and load snapshot', async () => {
    const snapshot = createEmptySnapshot({ name: 'store-test' });
    await store.save(snapshot, 'store-test');

    const loaded = await store.load('store-test');
    expect(loaded?.name).toBe('store-test');
  });

  it('should list snapshots', async () => {
    await store.save(createEmptySnapshot({ name: 'first' }), 'first');
    await store.save(createEmptySnapshot({ name: 'second' }), 'second');

    const list = await store.list();
    expect(list).toHaveLength(2);
  });

  it('should check existence', async () => {
    await store.save(createEmptySnapshot(), 'exists');

    expect(await store.exists('exists')).toBe(true);
    expect(await store.exists('not-here')).toBe(false);
  });

  it('should delete snapshot', async () => {
    await store.save(createEmptySnapshot(), 'to-delete');
    expect(await store.exists('to-delete')).toBe(true);

    await store.delete('to-delete');
    expect(await store.exists('to-delete')).toBe(false);
  });

  it('should get latest snapshot', async () => {
    const first = createEmptySnapshot({ name: 'first' });
    first.created_at = '2025-01-01T00:00:00.000Z';
    await store.save(first, 'first');

    const second = createEmptySnapshot({ name: 'second' });
    second.created_at = '2025-01-15T00:00:00.000Z';
    await store.save(second, 'second');

    const latest = await store.getLatest();
    expect(latest?.name).toBe('second');
  });
});
