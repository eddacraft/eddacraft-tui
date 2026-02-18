import chokidar from 'chokidar';

export interface WatchEvent {
  type: 'change' | 'add';
  path: string;
}

const WATCH_PATTERNS = ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.jsx'];
const IGNORE_PATTERNS = ['**/node_modules/**', '**/dist/**', '**/.git/**', '**/.anvil/**'];

export interface TutorialWatcher {
  close: () => void;
  ready: Promise<void>;
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
 */
export function createTutorialWatcher(
  workspaceRoot: string,
  onEvent: (event: WatchEvent) => void
): TutorialWatcher {
  const watcher = chokidar.watch(WATCH_PATTERNS, {
    cwd: workspaceRoot,
    ignored: IGNORE_PATTERNS,
    ignoreInitial: true,
  });

  const ready = new Promise<void>((resolve, reject) => {
    watcher.on('ready', () => resolve());
    watcher.on('error', (err) => reject(err));
  });

  watcher.on('change', (path: string) => onEvent({ type: 'change', path }));
  watcher.on('add', (path: string) => onEvent({ type: 'add', path }));

  return { close: () => void watcher.close(), ready };
}

/** Patterns watched by the tutorial watcher (for display). */
export const WATCHED_PATTERNS = WATCH_PATTERNS;
