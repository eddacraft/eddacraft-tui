/**
 * PTY-based E2E test utilities for TUI testing
 *
 * Uses node-pty to spawn real terminal processes with proper
 * keyboard input simulation and ANSI output capture.
 */

import * as nodePty from 'node-pty';
import { resolve } from 'node:path';

export interface PTYSpawnOptions {
  /** Arguments to pass to the CLI */
  args?: string[];
  /** Working directory */
  cwd?: string;
  /** Environment variables */
  env?: Record<string, string>;
  /** Terminal columns (default: 120) */
  cols?: number;
  /** Terminal rows (default: 30) */
  rows?: number;
  /** Timeout in ms (default: 10000) */
  timeout?: number;
}

export interface PTYSession {
  /** Send text input to the PTY */
  write: (data: string) => void;
  /** Send a key sequence */
  sendKey: (key: KeySequence) => void;
  /** Wait for output to contain a string */
  waitFor: (text: string, timeout?: number) => Promise<string>;
  /** Wait for output to match a regex */
  waitForMatch: (pattern: RegExp, timeout?: number) => Promise<RegExpMatchArray>;
  /** Get all output so far */
  getOutput: () => string;
  /** Get the last N lines of output */
  getLastLines: (n: number) => string[];
  /** Kill the PTY process */
  kill: () => void;
  /** Wait for the process to exit */
  waitForExit: (timeout?: number) => Promise<number>;
  /** Check if process is still running */
  isRunning: () => boolean;
}

export type KeySequence =
  | 'enter'
  | 'escape'
  | 'tab'
  | 'backspace'
  | 'up'
  | 'down'
  | 'left'
  | 'right'
  | 'ctrl+c'
  | 'ctrl+d'
  | 'space'
  | 'j'
  | 'k'
  | 'q';

const KEY_CODES: Record<KeySequence, string> = {
  enter: '\r',
  escape: '\x1B',
  tab: '\t',
  backspace: '\x7F',
  up: '\x1B[A',
  down: '\x1B[B',
  left: '\x1B[D',
  right: '\x1B[C',
  'ctrl+c': '\x03',
  'ctrl+d': '\x04',
  space: ' ',
  j: 'j',
  k: 'k',
  q: 'q',
};

/**
 * Spawn the Anvil CLI in a PTY for E2E testing
 */
export function spawnAnvilPTY(options: PTYSpawnOptions = {}): PTYSession {
  const cliPath = resolve(__dirname, '../../..', 'dist', 'index.js');
  const {
    args = [],
    cwd = process.cwd(),
    env = {},
    cols = 120,
    rows = 30,
    timeout = 10000,
  } = options;

  let output = '';
  let exitCode: number | null = null;
  let exitResolvers: Array<(code: number) => void> = [];

  const pty = nodePty.spawn('node', [cliPath, ...args], {
    name: 'xterm-256color',
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
  });

  pty.onData((data) => {
    output += data;
  });

  pty.onExit(({ exitCode: code }) => {
    exitCode = code;
    exitResolvers.forEach((resolve) => resolve(code));
    exitResolvers = [];
  });

  // Set up timeout
  const timeoutId = setTimeout(() => {
    if (exitCode === null) {
      pty.kill();
    }
  }, timeout);

  const session: PTYSession = {
    write: (data: string) => {
      pty.write(data);
    },

    sendKey: (key: KeySequence) => {
      pty.write(KEY_CODES[key]);
    },

    waitFor: async (text: string, waitTimeout = 5000): Promise<string> => {
      const start = Date.now();

      while (Date.now() - start < waitTimeout) {
        if (output.includes(text)) {
          return output;
        }
        await sleep(50);
      }

      throw new Error(
        `Timeout waiting for "${text}" after ${waitTimeout}ms.\n\nOutput received:\n${stripAnsi(output)}`
      );
    },

    waitForMatch: async (pattern: RegExp, waitTimeout = 5000): Promise<RegExpMatchArray> => {
      const start = Date.now();

      while (Date.now() - start < waitTimeout) {
        const match = stripAnsi(output).match(pattern);
        if (match) {
          return match;
        }
        await sleep(50);
      }

      throw new Error(
        `Timeout waiting for pattern ${pattern} after ${waitTimeout}ms.\n\nOutput received:\n${stripAnsi(output)}`
      );
    },

    getOutput: () => output,

    getLastLines: (n: number) => {
      const lines = stripAnsi(output).split('\n');
      return lines.slice(-n);
    },

    kill: () => {
      clearTimeout(timeoutId);
      pty.kill();
    },

    waitForExit: async (waitTimeout = 5000): Promise<number> => {
      if (exitCode !== null) {
        clearTimeout(timeoutId);
        return exitCode;
      }

      return new Promise((resolve, reject) => {
        const timeoutHandle = setTimeout(() => {
          reject(
            new Error(
              `Process did not exit within ${waitTimeout}ms.\n\nOutput:\n${stripAnsi(output)}`
            )
          );
        }, waitTimeout);

        exitResolvers.push((code) => {
          clearTimeout(timeoutHandle);
          clearTimeout(timeoutId);
          resolve(code);
        });
      });
    },

    isRunning: () => exitCode === null,
  };

  return session;
}

/**
 * Strip ANSI escape codes from output for easier assertion
 */
export function stripAnsi(str: string): string {
  // eslint-disable-next-line no-control-regex -- independantly verified by codex 20260205
  return str.replace(/\x1B\[[0-9;]*[a-zA-Z]/g, '').replace(/\x1B\][^\x07]*\x07/g, '');
}

/**
 * Sleep helper
 */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Test helper to delay between key presses
 * (gives React time to process state updates)
 */
export async function typeWithDelay(session: PTYSession, keys: KeySequence[], delayMs = 100) {
  for (const key of keys) {
    session.sendKey(key);
    await sleep(delayMs);
  }
}
