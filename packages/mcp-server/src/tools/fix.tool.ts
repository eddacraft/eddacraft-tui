import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import {
  readFileSync,
  writeFileSync,
  realpathSync,
  mkdirSync,
  unlinkSync,
  existsSync,
} from 'node:fs';
import { resolve, relative, dirname, sep } from 'node:path';

/** Simple file lock for preventing concurrent file modifications */
async function withFileLock<T>(lockPath: string, fn: () => T | Promise<T>): Promise<T> {
  const dir = dirname(lockPath);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  // Attempt to create lock file (exclusive)
  try {
    writeFileSync(lockPath, String(process.pid), { flag: 'wx' });
  } catch {
    throw new Error('File is locked by another process');
  }
  try {
    return await fn();
  } finally {
    try {
      unlinkSync(lockPath);
    } catch {
      /* ignore cleanup errors */
    }
  }
}

/**
 * Deterministic mechanical transforms for known antipattern warnings.
 *
 * These are safe, non-heuristic replacements:
 * - AP-001: Broad `eslint-disable` -> next-line `eslint-disable-next-line`
 * - AP-003: `: any` -> `: unknown`
 * - AP-004: `@ts-ignore` -> `@ts-expect-error`
 */
const FIXABLE_PATTERNS: Record<
  string,
  { description: string; apply: (line: string) => string | null }
> = {
  'AP-003': {
    // Limitation: This regex-based fix only handles simple `: any` annotations.
    // It will NOT correctly transform: generic parameters (Array<any>), union
    // types (string | any), function signatures ((...args: any[]) => void).
    // It does not handle `: any` inside template-literal expressions (`${...}`).
    // Manual review is advised.
    description: 'Replace explicit `any` type with `unknown`',
    apply: (line) => {
      const trimmed = line.trimStart();
      // Skip full-line comments: //, /* ... */, and JSDoc continuation lines (* )
      if (trimmed.startsWith('//') || trimmed.startsWith('* ') || trimmed.startsWith('/*')) {
        return null;
      }
      if (!/:(\s*)any\b/.test(line)) {
        return null;
      }
      let result = '';
      let inString: string | null = null;
      let inBlockComment = false;
      for (let i = 0; i < line.length; i++) {
        const ch = line[i];
        const next = line[i + 1];
        if (inString) {
          result += ch;
          if (ch === '\\') {
            result += line[++i] ?? '';
            continue;
          }
          if (ch === inString) inString = null;
          continue;
        }
        if (inBlockComment) {
          result += ch;
          if (ch === '*' && next === '/') {
            result += '/';
            i++;
            inBlockComment = false;
          }
          continue;
        }
        // Entering a line comment — rest of line is not code
        if (ch === '/' && next === '/') {
          result += line.slice(i);
          break;
        }
        if (ch === '/' && next === '*') {
          result += '/*';
          i++;
          inBlockComment = true;
          continue;
        }
        if (ch === '"' || ch === "'" || ch === '`') {
          inString = ch;
          result += ch;
          continue;
        }
        if (ch === ':' && /^:\s*any\b/.test(line.slice(i))) {
          const m = line.slice(i).match(/^(:\s*)any\b/);
          if (m) {
            result += m[1] + 'unknown';
            i += m[0].length - 1;
            continue;
          }
        }
        result += ch;
      }
      return result !== line ? result : null;
    },
  },
  'AP-004': {
    description: 'Replace @ts-ignore with @ts-expect-error',
    apply: (line) => {
      if (line.includes('@ts-ignore')) {
        return line.replace(/@ts-ignore/g, '@ts-expect-error');
      }
      return null;
    },
  },
  'AP-001': {
    description: 'Replace broad eslint-disable with eslint-disable-next-line',
    apply: (line) => {
      const match = line.match(/\/\*\s*eslint-disable\s*\*\//);
      if (match) {
        return line.replace(/\/\*\s*eslint-disable\s*\*\//, '// eslint-disable-next-line');
      }
      return null;
    },
  },
};

export function registerFixTool(server: McpServer, getWorkspaceRoot: () => string): void {
  server.registerTool(
    'anvil_fix',
    {
      title: 'Anvil Fix',
      description:
        'Apply deterministic auto-fixes for known antipattern warnings. ' +
        'Supports AP-001 (broad eslint-disable), AP-003 (explicit any), AP-004 (@ts-ignore). ' +
        'LIMITATION: AP-003 uses line-by-line regex, so it may match `: any` inside string ' +
        'literals or comments. It does not handle generic parameters (Array<any>), union types, ' +
        'or complex function signatures. Always review the applied changes.',
      inputSchema: {
        filePath: z.string().describe('File path relative to workspace root'),
        warningId: z.string().describe('Warning/pattern ID (e.g., AP-003, AP-004)'),
        line: z.number().describe('Line number of the warning (1-based)'),
      },
      annotations: {
        readOnlyHint: false,
        destructiveHint: true,
        idempotentHint: true,
      },
    },
    async ({ filePath, warningId, line }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const fixer = FIXABLE_PATTERNS[warningId];
        if (!fixer) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify({
                  fixed: false,
                  reason: `No auto-fix available for ${warningId}. Fixable patterns: ${Object.keys(FIXABLE_PATTERNS).join(', ')}`,
                }),
              },
            ],
          };
        }

        // Validate path stays within workspace (logical + symlink check)
        const absPath = resolve(workspaceRoot, filePath);
        const rel = relative(workspaceRoot, absPath);
        if (rel.startsWith('..') || resolve(workspaceRoot, rel) !== absPath) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify({
                  fixed: false,
                  reason: `Path "${filePath}" resolves outside workspace root`,
                }),
              },
            ],
          };
        }

        // Resolve symlinks to prevent escaping via symlink targets
        const realRoot = realpathSync(workspaceRoot);
        const realAbs = realpathSync(absPath);
        if (!realAbs.startsWith(realRoot + sep) && realAbs !== realRoot) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify({
                  fixed: false,
                  reason: `Path "${filePath}" resolves outside workspace root (symlink)`,
                }),
              },
            ],
          };
        }

        return await withFileLock(absPath + '.lock', () => {
          const content = readFileSync(absPath, 'utf-8');
          const lines = content.split('\n');
          const lineIndex = line - 1;

          if (lineIndex < 0 || lineIndex >= lines.length) {
            return {
              content: [
                {
                  type: 'text' as const,
                  text: JSON.stringify({
                    fixed: false,
                    reason: `Line ${line} out of range (file has ${lines.length} lines)`,
                  }),
                },
              ],
            };
          }

          const before = lines[lineIndex];
          const after = fixer.apply(before);

          if (after === null) {
            return {
              content: [
                {
                  type: 'text' as const,
                  text: JSON.stringify({
                    fixed: false,
                    reason: `Pattern ${warningId} not found on line ${line}`,
                  }),
                },
              ],
            };
          }

          lines[lineIndex] = after;
          writeFileSync(absPath, lines.join('\n'), 'utf-8');

          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify(
                  {
                    fixed: true,
                    description: fixer.description,
                    filePath,
                    line,
                    before: before.trim(),
                    after: after.trim(),
                  },
                  null,
                  2
                ),
              },
            ],
          };
        });
      } catch (error) {
        return {
          content: [
            {
              type: 'text' as const,
              text: JSON.stringify({
                error: error instanceof Error ? error.message : String(error),
              }),
            },
          ],
          isError: true,
        };
      }
    }
  );
}
