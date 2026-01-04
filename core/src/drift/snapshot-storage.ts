import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import {
  DriftSnapshotSchema,
  type DriftSnapshot,
  type SnapshotMetadata,
  generateSnapshotFilename,
  generateNamedSnapshotFilename,
} from './snapshot-schema.js';

export const SNAPSHOTS_DIR = 'snapshots';
export const ANVIL_DIR = '.anvil';

function getSnapshotsPath(workspaceRoot: string): string {
  return path.join(workspaceRoot, ANVIL_DIR, SNAPSHOTS_DIR);
}

export async function ensureSnapshotsDir(workspaceRoot: string): Promise<string> {
  const snapshotsPath = getSnapshotsPath(workspaceRoot);
  await fs.mkdir(snapshotsPath, { recursive: true });
  return snapshotsPath;
}

export async function saveSnapshot(
  workspaceRoot: string,
  snapshot: DriftSnapshot,
  name?: string
): Promise<string> {
  const snapshotsPath = await ensureSnapshotsDir(workspaceRoot);

  const filename = name
    ? generateNamedSnapshotFilename(name)
    : generateSnapshotFilename(new Date(snapshot.created_at));

  const filePath = path.join(snapshotsPath, filename);

  await fs.writeFile(filePath, JSON.stringify(snapshot, null, 2), 'utf-8');

  return filePath;
}

function isTimestampIdentifier(identifier: string): boolean {
  return /^\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}/.test(identifier);
}

export async function loadSnapshot(
  workspaceRoot: string,
  nameOrFilename: string
): Promise<DriftSnapshot | null> {
  const snapshotsPath = getSnapshotsPath(workspaceRoot);

  let filename = nameOrFilename;
  if (!filename.endsWith('.json')) {
    if (isTimestampIdentifier(nameOrFilename)) {
      filename = `snapshot-${nameOrFilename}.json`;
    } else {
      filename = generateNamedSnapshotFilename(nameOrFilename);
    }
  }

  const filePath = path.join(snapshotsPath, filename);

  try {
    const content = await fs.readFile(filePath, 'utf-8');
    const parsed = JSON.parse(content);
    const result = DriftSnapshotSchema.safeParse(parsed);

    if (result.success) {
      return result.data;
    }

    console.error('Invalid snapshot format:', result.error.format());
    return null;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return null;
    }
    throw error;
  }
}

export async function listSnapshots(workspaceRoot: string): Promise<SnapshotMetadata[]> {
  const snapshotsPath = getSnapshotsPath(workspaceRoot);

  try {
    const files = await fs.readdir(snapshotsPath);
    const snapshots: SnapshotMetadata[] = [];

    for (const file of files) {
      if (!file.startsWith('snapshot-') || !file.endsWith('.json')) {
        continue;
      }

      try {
        const snapshot = await loadSnapshot(workspaceRoot, file);
        if (snapshot) {
          snapshots.push({
            filename: file,
            name: snapshot.name,
            created_at: snapshot.created_at,
            metrics: snapshot.metrics,
          });
        }
      } catch {
        // Skip invalid files
      }
    }

    // Sort by creation date, newest first
    return snapshots.sort(
      (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return [];
    }
    throw error;
  }
}

export async function deleteSnapshot(
  workspaceRoot: string,
  nameOrFilename: string
): Promise<boolean> {
  const snapshotsPath = getSnapshotsPath(workspaceRoot);

  let filename = nameOrFilename;
  if (!filename.endsWith('.json')) {
    if (isTimestampIdentifier(nameOrFilename)) {
      filename = `snapshot-${nameOrFilename}.json`;
    } else {
      filename = generateNamedSnapshotFilename(nameOrFilename);
    }
  }

  const filePath = path.join(snapshotsPath, filename);

  try {
    await fs.unlink(filePath);
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return false;
    }
    throw error;
  }
}

export async function snapshotExists(
  workspaceRoot: string,
  nameOrFilename: string
): Promise<boolean> {
  const snapshotsPath = getSnapshotsPath(workspaceRoot);

  let filename = nameOrFilename;
  if (!filename.endsWith('.json')) {
    if (isTimestampIdentifier(nameOrFilename)) {
      filename = `snapshot-${nameOrFilename}.json`;
    } else {
      filename = generateNamedSnapshotFilename(nameOrFilename);
    }
  }

  const filePath = path.join(snapshotsPath, filename);

  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

export async function getLatestSnapshot(workspaceRoot: string): Promise<DriftSnapshot | null> {
  const snapshots = await listSnapshots(workspaceRoot);

  if (snapshots.length === 0) {
    return null;
  }

  return loadSnapshot(workspaceRoot, snapshots[0].filename);
}

export function resolveSnapshotName(identifier: string): {
  type: 'named' | 'filename' | 'timestamp';
  value: string;
} {
  if (identifier.endsWith('.json')) {
    return { type: 'filename', value: identifier };
  }

  if (isTimestampIdentifier(identifier)) {
    return { type: 'timestamp', value: identifier };
  }

  return { type: 'named', value: identifier };
}

export class SnapshotStore {
  private workspaceRoot: string;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
  }

  async save(snapshot: DriftSnapshot, name?: string): Promise<string> {
    return saveSnapshot(this.workspaceRoot, snapshot, name);
  }

  async load(nameOrFilename: string): Promise<DriftSnapshot | null> {
    return loadSnapshot(this.workspaceRoot, nameOrFilename);
  }

  async list(): Promise<SnapshotMetadata[]> {
    return listSnapshots(this.workspaceRoot);
  }

  async delete(nameOrFilename: string): Promise<boolean> {
    return deleteSnapshot(this.workspaceRoot, nameOrFilename);
  }

  async exists(nameOrFilename: string): Promise<boolean> {
    return snapshotExists(this.workspaceRoot, nameOrFilename);
  }

  async getLatest(): Promise<DriftSnapshot | null> {
    return getLatestSnapshot(this.workspaceRoot);
  }
}
