import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { generateWarningId, type Warning } from '@eddacraft/anvil-core';

const RECENT_WARNINGS_FILE = join('.anvil', 'cache', 'recent-warnings.json');

interface RecentWarningsPayload {
  version: '1.0.0';
  generatedAt: string;
  warnings: Warning[];
}

export interface RecentWarningWithId extends Warning {
  warningId: string;
}

export async function saveRecentWarnings(
  workspaceRoot: string,
  warnings: Warning[]
): Promise<void> {
  const path = join(workspaceRoot, RECENT_WARNINGS_FILE);
  await mkdir(dirname(path), { recursive: true });

  const payload: RecentWarningsPayload = {
    version: '1.0.0',
    generatedAt: new Date().toISOString(),
    warnings,
  };

  await writeFile(path, JSON.stringify(payload, null, 2), 'utf-8');
}

export async function loadRecentWarnings(workspaceRoot: string): Promise<RecentWarningWithId[]> {
  const path = join(workspaceRoot, RECENT_WARNINGS_FILE);

  try {
    const raw = await readFile(path, 'utf-8');
    const parsed = JSON.parse(raw) as Partial<RecentWarningsPayload>;
    if (!Array.isArray(parsed.warnings)) {
      return [];
    }

    return parsed.warnings.map((warning) => ({
      ...warning,
      warningId: generateWarningId(warning),
    }));
  } catch {
    return [];
  }
}
