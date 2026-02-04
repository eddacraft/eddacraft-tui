import chokidar from 'chokidar';

export interface WatchEvent {
  type: 'change' | 'add';
  path: string;
}

const WATCH_PATTERNS = ['**/*.ts', '**/*.tsx', '**/*.js', '**/*.jsx'];
const IGNORE_PATTERNS = ['**/node_modules/**', '**/dist/**', '**/.git/**', '**/.anvil/**'];

/**
 * Create a file watcher for the tutorial's Watch step.
 *
 * Extracted into its own module so tests can mock this function
 * without needing to mock chokidar internals.
 */
export function createTutorialWatcher(
  workspaceRoot: string,
  onEvent: (event: WatchEvent) => void
): { close: () => void } {
  const watcher = chokidar.watch(WATCH_PATTERNS, {
    cwd: workspaceRoot,
    ignored: IGNORE_PATTERNS,
    ignoreInitial: true,
  });

  watcher.on('change', (path: string) => onEvent({ type: 'change', path }));
  watcher.on('add', (path: string) => onEvent({ type: 'add', path }));

  return { close: () => void watcher.close() };
}

/** Patterns watched by the tutorial watcher (for display). */
export const WATCHED_PATTERNS = WATCH_PATTERNS;
