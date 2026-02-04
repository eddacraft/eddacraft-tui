export type { McpConfig, McpConfigOptions } from './types.js';
export { generateClaudeCodeConfig } from './claude-code.js';
export { generateCursorConfig } from './cursor.js';
export { generateWindsurfConfig } from './windsurf.js';
export { generateVscodeConfig } from './vscode.js';

import type { McpConfig, McpConfigOptions } from './types.js';
import { generateClaudeCodeConfig } from './claude-code.js';
import { generateCursorConfig } from './cursor.js';
import { generateWindsurfConfig } from './windsurf.js';
import { generateVscodeConfig } from './vscode.js';

export type McpConfigTarget = 'claude-code' | 'cursor' | 'windsurf' | 'vscode';

export const SUPPORTED_TARGETS: McpConfigTarget[] = ['claude-code', 'cursor', 'windsurf', 'vscode'];

export function generateMcpConfig(target: McpConfigTarget, options?: McpConfigOptions): McpConfig {
  switch (target) {
    case 'claude-code':
      return generateClaudeCodeConfig(options);
    case 'cursor':
      return generateCursorConfig(options);
    case 'windsurf':
      return generateWindsurfConfig(options);
    case 'vscode':
      return generateVscodeConfig(options);
  }
}
