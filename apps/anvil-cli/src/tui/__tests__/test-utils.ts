import React from 'react';
import { render, type Instance } from 'ink-testing-library';

export interface RenderTUIResult<P = Record<string, unknown>> {
  instance: Instance;
  lastFrame: () => string;
  stdin: NodeJS.WriteStream;
  frames: string[];
  unmount: () => void;
  cleanup: () => void;
  rerender: (tree: React.ReactElement<P>) => void;
}

export function renderTUI<P extends object>(component: React.ReactElement<P>): RenderTUIResult<P> {
  const result = render(component);

  return {
    instance: result,
    lastFrame: () => result.lastFrame() ?? '',
    stdin: result.stdin,
    frames: result.frames,
    unmount: () => result.unmount(),
    cleanup: () => result.cleanup(),
    rerender: (tree: React.ReactElement<P>) => result.rerender(tree),
  };
}

export const KEY_SEQUENCES = {
  ENTER: '\r',
  ESCAPE: '\x1B',
  TAB: '\t',
  BACKSPACE: '\x7F',
  DELETE: '\x1B[3~',
  UP: '\x1B[A',
  DOWN: '\x1B[B',
  LEFT: '\x1B[D',
  RIGHT: '\x1B[C',
  HOME: '\x1B[H',
  END: '\x1B[F',
  PAGE_UP: '\x1B[5~',
  PAGE_DOWN: '\x1B[6~',
  CTRL_C: '\x03',
  CTRL_D: '\x04',
} as const;

export type KeyName = keyof typeof KEY_SEQUENCES;

export function simulateKeypress(stdin: NodeJS.WriteStream, key: KeyName | string): void {
  const sequence = KEY_SEQUENCES[key as KeyName] ?? key;
  stdin.write(sequence);
}

export async function flushPromises(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

export async function waitForFrame(
  lastFrame: () => string,
  predicate: (frame: string) => boolean,
  timeout = 1000
): Promise<string> {
  const start = Date.now();

  while (Date.now() - start < timeout) {
    const frame = lastFrame();
    if (predicate(frame)) {
      return frame;
    }
    await flushPromises();
  }

  throw new Error(`Timeout waiting for frame condition after ${timeout}ms`);
}

export interface MockTTYOptions {
  isTTY?: boolean;
  columns?: number;
  rows?: number;
  colorDepth?: 1 | 4 | 8 | 24;
}

export function createMockTTY(options: MockTTYOptions = {}): {
  isTTY: boolean;
  columns: number;
  rows: number;
  getColorDepth: () => number;
} {
  return {
    isTTY: options.isTTY ?? true,
    columns: options.columns ?? 80,
    rows: options.rows ?? 24,
    getColorDepth: () => options.colorDepth ?? 8,
  };
}

export function mockStdout(options: MockTTYOptions = {}): void {
  const mock = createMockTTY(options);
  Object.defineProperty(process.stdout, 'isTTY', { value: mock.isTTY, writable: true });
  Object.defineProperty(process.stdout, 'columns', { value: mock.columns, writable: true });
  Object.defineProperty(process.stdout, 'rows', { value: mock.rows, writable: true });
  Object.defineProperty(process.stdout, 'getColorDepth', {
    value: mock.getColorDepth,
    writable: true,
  });
}

export function mockStdin(options: MockTTYOptions = {}): void {
  const mock = createMockTTY(options);
  Object.defineProperty(process.stdin, 'isTTY', { value: mock.isTTY, writable: true });
}

export function setupTTYMocks(options: MockTTYOptions = {}): () => void {
  const originalStdout = {
    isTTY: process.stdout.isTTY,
    columns: process.stdout.columns,
    rows: process.stdout.rows,
    getColorDepth: process.stdout.getColorDepth,
  };

  const originalStdin = {
    isTTY: process.stdin.isTTY,
  };

  mockStdout(options);
  mockStdin(options);

  return () => {
    Object.defineProperty(process.stdout, 'isTTY', {
      value: originalStdout.isTTY,
      writable: true,
    });
    Object.defineProperty(process.stdout, 'columns', {
      value: originalStdout.columns,
      writable: true,
    });
    Object.defineProperty(process.stdout, 'rows', { value: originalStdout.rows, writable: true });
    Object.defineProperty(process.stdout, 'getColorDepth', {
      value: originalStdout.getColorDepth,
      writable: true,
    });
    Object.defineProperty(process.stdin, 'isTTY', { value: originalStdin.isTTY, writable: true });
  };
}

/**
 * Strip ANSI escape codes from a string for easier assertion.
 */
// eslint-disable-next-line no-control-regex
const ANSI_RE = /\x1B\[[0-9;]*[a-zA-Z]/g;
// eslint-disable-next-line no-control-regex
const OSC_RE = /\x1B\][^\x07]*\x07/g;
export function stripAnsi(str: string): string {
  return str.replace(ANSI_RE, '').replace(OSC_RE, '');
}

export function expectFrame(lastFrame: () => string): {
  toContain: (expected: string) => void;
  toMatch: (pattern: RegExp) => void;
  toNotContain: (unexpected: string) => void;
} {
  return {
    toContain: (expected: string) => {
      const frame = lastFrame();
      if (!frame.includes(expected)) {
        throw new Error(`Expected frame to contain "${expected}" but got:\n${frame}`);
      }
    },
    toMatch: (pattern: RegExp) => {
      const frame = lastFrame();
      if (!pattern.test(frame)) {
        throw new Error(`Expected frame to match ${pattern} but got:\n${frame}`);
      }
    },
    toNotContain: (unexpected: string) => {
      const frame = lastFrame();
      if (frame.includes(unexpected)) {
        throw new Error(`Expected frame to NOT contain "${unexpected}" but it did:\n${frame}`);
      }
    },
  };
}
