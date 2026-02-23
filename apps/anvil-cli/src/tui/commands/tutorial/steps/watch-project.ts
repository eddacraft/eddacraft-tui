import chokidar from 'chokidar';
import { extname } from 'node:path';

export interface WatchEvent {
  type: 'change' | 'add';
  path: string;
}

/** Extensions the tutorial watcher cares about. */
const WATCH_EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.jsx']);

/** Directories to ignore (checked as path segments). */
const IGNORE_DIRS = ['node_modules', 'dist', '.git', '.anvil'];

export interface TutorialWatcher {
  close: () => void;
  ready: Promise<void>;
}

/**
 * Check whether a path should be ignored.
 *
 * chokidar v5 removed glob support, so we watch the workspace root
 * and filter by extension + ignored directories in this callback.
 */
function shouldIgnore(path: string): boolean {
  for (const dir of IGNORE_DIRS) {
    if (path.includes(`/${dir}/`) || path.includes(`\\${dir}\\`)) return true;
  }
  // Only accept files with matching extensions (allow directories through
  // so chokidar can traverse into them).
  const ext = extname(path);
  if (ext && !WATCH_EXTENSIONS.has(ext)) return true;
  return false;
}

/**
 * Create a file watcher for the tutorial's Watch step.
 *
 * Extracted into its own module so tests can mock this function
 * without needing to mock chokidar internals.
 *
 * The returned `ready` promise resolves once chokidar has completed
 * its initial scan and is actively watching for changes. File edits
 * before `ready` resolves may be silently missed.
 *
 * NOTE: chokidar v5 removed glob pattern support. We watch the
 * workspace root directory and filter via the `ignored` callback.
 */
export function createTutorialWatcher(
  workspaceRoot: string,
  onEvent: (event: WatchEvent) => void
): TutorialWatcher {
  const watcher = chokidar.watch(workspaceRoot, {
    ignored: shouldIgnore,
    ignoreInitial: true,
    depth: 10,
  });

  const ready = new Promise<void>((resolve, reject) => {
    watcher.on('ready', () => resolve());
    watcher.on('error', (err) => reject(err));
  });

  watcher.on('change', (path: string) => onEvent({ type: 'change', path }));
  watcher.on('add', (path: string) => onEvent({ type: 'add', path }));

  return { close: () => void watcher.close(), ready };
}

/** Display-friendly list of watched extensions. */
export const WATCHED_PATTERNS = ['*.ts', '*.tsx', '*.js', '*.jsx'];
