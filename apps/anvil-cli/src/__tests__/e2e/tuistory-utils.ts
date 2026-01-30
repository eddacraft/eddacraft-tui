/**
 * Tuistory-based E2E test utilities for TUI testing
 *
 * Tuistory provides a Playwright-like API for testing terminal applications.
 * This wrapper provides Anvil-specific conveniences.
 *
 * @see https://github.com/remorses/tuistory
 *
 * Key advantages over custom PTY utils:
 * - ANSI-aware text extraction (filter by color, bold, etc.)
 * - More comprehensive keyboard support with modifiers
 * - click() API for text-based interactions
 * - Frame capture for animation/transition testing
 * - Mouse scroll support
 */

import { launchTerminal, Session, type LaunchOptions, type TextOptions, type Key } from 'tuistory';
import { resolve } from 'node:path';
import { existsSync, readFileSync } from 'node:fs';

const CLI_PATH = resolve(__dirname, '../../..', 'dist', 'index.js');
const PROJECT_ROOT = resolve(__dirname, '../../../../..');
const PACKAGE_JSON_PATH = resolve(__dirname, '../../..', 'package.json');

export interface AnvilSessionOptions {
  /** Arguments to pass to the CLI */
  args?: string[];
  /** Working directory (defaults to PROJECT_ROOT) */
  cwd?: string;
  /** Additional environment variables */
  env?: Record<string, string | undefined>;
  /** Terminal columns (default: 120) */
  cols?: number;
  /** Terminal rows (default: 30) */
  rows?: number;
  /** Wait for initial data after launch (default: true) */
  waitForData?: boolean;
  /** Timeout for initial data wait in ms (default: 5000) */
  waitForDataTimeout?: number;
}

/**
 * Read version from package.json to avoid hardcoding
 */
export function getPackageVersion(): string {
  try {
    const pkg = JSON.parse(readFileSync(PACKAGE_JSON_PATH, 'utf-8'));
    return pkg.version;
  } catch {
    return '0.1.0';
  }
}

/**
 * Check if CLI is built before running tests
 */
export function ensureCliBuild(): void {
  if (!existsSync(CLI_PATH)) {
    throw new Error(`CLI not built. Run 'pnpm build' first. Expected: ${CLI_PATH}`);
  }
}

/**
 * Launch Anvil CLI in a tuistory terminal session
 *
 * @example
 * ```ts
 * const session = await launchAnvil({ args: ['--help'] });
 * await session.waitForText('Available commands');
 * session.close();
 * ```
 */
export async function launchAnvil(options: AnvilSessionOptions = {}): Promise<Session> {
  const {
    args = [],
    cwd = PROJECT_ROOT,
    env = {},
    cols = 120,
    rows = 30,
    waitForData = true,
    waitForDataTimeout = 5000,
  } = options;

  const session = await launchTerminal({
    command: 'node',
    args: [CLI_PATH, ...args],
    cols,
    rows,
    cwd,
    env: {
      ...process.env,
      ...env,
      FORCE_COLOR: '1',
      TERM: 'xterm-256color',
      CI: 'false',
      GITHUB_ACTIONS: 'false',
      NO_TUI: '',
    },
    waitForData,
    waitForDataTimeout,
  });

  return session;
}

/**
 * Get text with ANSI style filtering
 *
 * @example
 * ```ts
 * // Get only bold text (headers, emphasis)
 * const headers = await getStyledText(session, { only: { bold: true } });
 *
 * // Get text with a specific foreground color
 * const errorText = await getStyledText(session, { only: { foreground: '#dc2626' } });
 * ```
 */
export async function getStyledText(session: Session, options: TextOptions = {}): Promise<string> {
  return session.text(options);
}

/**
 * Wait for text with a timeout, throwing a descriptive error on failure
 */
export async function waitForTextWithContext(
  session: Session,
  pattern: string | RegExp,
  context: string,
  timeout = 10000
): Promise<string> {
  try {
    return await session.waitForText(pattern, { timeout });
  } catch {
    const currentText = await session.text({ trimEnd: true });
    throw new Error(
      `${context}: Timeout waiting for ${pattern} after ${timeout}ms.\n\nCurrent output:\n${currentText}`
    );
  }
}

/**
 * Safely close a session, catching any errors
 */
export function safeClose(session: Session | null): void {
  try {
    session?.close();
  } catch {
    // Ignore close errors
  }
}

// Re-export types for convenience
export type { Session, LaunchOptions, TextOptions, Key };
export { launchTerminal };
