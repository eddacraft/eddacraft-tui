/**
 * Recent warnings store
 *
 * Persists the most recent `anvil check` warnings to disk so that
 * `anvil explain` can operate on concrete warning instances without
 * requiring the user to re-run the check.
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { generateWarningId, WarningSchema, type Warning } from '@eddacraft/anvil-core';
import { print } from '../utils/output.js';

const RECENT_WARNINGS_FILE = join('.anvil', 'cache', 'recent-warnings.json');

interface RecentWarningsPayload {
  version: '1.0.0';
  generatedAt: string;
  warnings: Warning[];
}

export interface RecentWarningWithId extends Warning {
  warningId: string;
}

/** Persist warnings from the latest check run to `.anvil/cache/recent-warnings.json`. */
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

/**
 * Load warnings saved by the most recent `anvil check` run.
 *
 * Returns an empty array when the cache file is missing or contains
 * invalid data.  Only warnings that pass schema validation are returned.
 */
export async function loadRecentWarnings(workspaceRoot: string): Promise<RecentWarningWithId[]> {
  const path = join(workspaceRoot, RECENT_WARNINGS_FILE);

  try {
    const raw = await readFile(path, 'utf-8');
    const parsed = JSON.parse(raw) as Partial<RecentWarningsPayload>;
    if (!Array.isArray(parsed.warnings)) {
      return [];
    }

    return parsed.warnings
      .filter((w) => WarningSchema.safeParse(w).success)
      .map((warning) => ({
        ...warning,
        warningId: generateWarningId(warning),
      }));
  } catch (error) {
    const err = error as NodeJS.ErrnoException;
    if (err?.code !== 'ENOENT') {
      print(`Failed to load recent warnings from "${path}":`, err);
    }
    return [];
  }
}
