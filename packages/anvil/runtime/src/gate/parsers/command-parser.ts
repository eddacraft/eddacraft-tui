import { parse as shellParse } from 'shell-quote';
import type { ParsedCommand } from '../rules/types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('gate');

const MAX_UNWRAP_DEPTH = 5;

const SHELL_WRAPPERS = ['bash', 'sh', 'zsh', 'dash'];
const PRIVILEGED_WRAPPERS = ['sudo', 'doas'];
const ENV_WRAPPERS = ['env', 'command', 'nohup', 'nice', 'time', 'strace'];

const INTERPRETER_COMMANDS = ['python', 'python3', 'node', 'ruby', 'perl', 'php'];

const SHELL_LIKE_INTERPRETERS = ['bash', 'sh', 'zsh', 'dash'];

const COMMAND_OPERATORS = ['and', 'or', ';', '|', '||', '&&'];

function isShellWrapper(cmd: string): boolean {
  return SHELL_WRAPPERS.includes(cmd);
}

function isPrivilegedWrapper(cmd: string): boolean {
  return PRIVILEGED_WRAPPERS.includes(cmd);
}

function isEnvWrapper(cmd: string): boolean {
  return ENV_WRAPPERS.includes(cmd);
}

function isInterpreter(cmd: string): boolean {
  return INTERPRETER_COMMANDS.includes(cmd);
}

interface GlobToken {
  op: 'glob';
  pattern: string;
}

interface OperatorToken {
  op: string;
}

type ShellToken = string | GlobToken | OperatorToken | Record<string, unknown>;

function isGlobToken(token: unknown): token is GlobToken {
  return (
    typeof token === 'object' &&
    token !== null &&
    'op' in token &&
    (token as { op: string }).op === 'glob' &&
    'pattern' in token
  );
}

function isOperatorToken(token: unknown): token is OperatorToken {
  return (
    typeof token === 'object' &&
    token !== null &&
    'op' in token &&
    COMMAND_OPERATORS.includes((token as { op: string }).op)
  );
}

function splitByOperators(tokens: ShellToken[]): Array<{ tokens: string[]; operator?: string }> {
  const commands: Array<{ tokens: string[]; operator?: string }> = [];
  let currentTokens: string[] = [];
  let lastOperator: string | undefined;

  for (const token of tokens) {
    if (isOperatorToken(token)) {
      if (currentTokens.length > 0) {
        commands.push({ tokens: currentTokens, operator: lastOperator });
        currentTokens = [];
      }
      lastOperator = token.op;
    } else if (typeof token === 'string') {
      currentTokens.push(token);
    } else if (isGlobToken(token)) {
      currentTokens.push(token.pattern);
    }
  }

  if (currentTokens.length > 0) {
    commands.push({ tokens: currentTokens, operator: lastOperator });
  }

  return commands;
}

function tokenise(cmd: string): string[] {
  const parsed = shellParse(cmd) as ShellToken[];
  return parsed
    .map((token) => {
      if (typeof token === 'string') {
        return token;
      }
      if (isGlobToken(token)) {
        return token.pattern;
      }
      return null;
    })
    .filter((token): token is string => token !== null && token.length > 0);
}

function tokeniseWithOperators(cmd: string): {
  tokens: string[];
  isCompound: boolean;
  subCommands: Array<{ tokens: string[]; operator?: string }>;
} {
  const parsed = shellParse(cmd) as ShellToken[];
  const hasOperator = parsed.some(isOperatorToken);

  if (!hasOperator) {
    const tokens = parsed
      .map((token) => {
        if (typeof token === 'string') return token;
        if (isGlobToken(token)) return token.pattern;
        return null;
      })
      .filter((token): token is string => token !== null && token.length > 0);

    return { tokens, isCompound: false, subCommands: [{ tokens }] };
  }

  const subCommands = splitByOperators(parsed);
  const allTokens = subCommands.flatMap((sc) => sc.tokens);

  return { tokens: allTokens, isCompound: true, subCommands };
}

interface UnwrapResult {
  unwrapped: string;
  wrappers: string[];
}

function extractShellWrapperArg(tokens: string[]): string | null {
  const cmdIndex = tokens.findIndex((t) => t === '-c');
  if (cmdIndex !== -1 && cmdIndex + 1 < tokens.length) {
    return tokens[cmdIndex + 1];
  }
  return null;
}

function extractEnvCommand(tokens: string[]): string[] | null {
  let startIndex = 1;
  while (startIndex < tokens.length) {
    const token = tokens[startIndex];
    if (token.includes('=') || token.startsWith('-')) {
      startIndex++;
    } else {
      break;
    }
  }
  if (startIndex < tokens.length) {
    return tokens.slice(startIndex);
  }
  return null;
}

/**
 * Best-effort extraction of commands executed via interpreter flags (-c, -e).
 *
 * Limitations: this uses pattern matching and will miss obfuscated invocations
 * (string concatenation, hex encoding, template literals, variable
 * interpolation, eval of computed strings). Treat the result as a heuristic —
 * a null return does NOT mean the script is safe.
 */
function extractInterpreterCommand(tokens: string[], interpreter?: string): string | null {
  const cIndex = tokens.findIndex((t) => t === '-c' || t === '-e');
  if (cIndex !== -1 && cIndex + 1 < tokens.length) {
    const script = tokens[cIndex + 1];
    const execPatterns: RegExp[] = [
      /os\.system\s*\(\s*['"](.*?)['"]\s*\)/,
      /subprocess\.(?:run|call|Popen)\s*\(\s*['"](.*?)['"]/,
      /exec\s*\(\s*['"](.*?)['"]\s*\)/,
      /execSync\s*\(\s*['"](.*?)['"]\s*\)/,
      /`([^`]+)`/,
      /system\s*\(\s*['"](.*?)['"]\s*\)/,
      /\beval\s*\(\s*['"](.*?)['"]\s*\)/,
    ];

    // $() is a shell construct — only match it for shell-like interpreters
    if (!interpreter || SHELL_LIKE_INTERPRETERS.includes(interpreter)) {
      execPatterns.push(/\$\(\s*(.*?)\s*\)/);
    }

    for (const pattern of execPatterns) {
      const match = script.match(pattern);
      if (match?.[1]) {
        return match[1];
      }
    }
  }
  return null;
}

function unwrapCommand(cmd: string, depth = 0): UnwrapResult {
  if (depth >= MAX_UNWRAP_DEPTH) {
    return { unwrapped: cmd, wrappers: [] };
  }

  const trimmed = cmd.trim();
  if (!trimmed) {
    return { unwrapped: cmd, wrappers: [] };
  }

  const tokens = tokenise(trimmed);
  if (tokens.length === 0) {
    return { unwrapped: cmd, wrappers: [] };
  }

  const firstToken = tokens[0];

  if (isShellWrapper(firstToken)) {
    const innerCmd = extractShellWrapperArg(tokens);
    if (innerCmd) {
      const inner = unwrapCommand(innerCmd, depth + 1);
      return {
        unwrapped: inner.unwrapped,
        wrappers: [firstToken, ...inner.wrappers],
      };
    }
  }

  if (isPrivilegedWrapper(firstToken)) {
    const sudoFlagsWithArgs = ['-u', '-g', '-H', '-C', '-h', '-p', '-r', '-t', '-T', '-U'];
    let startIndex = 1;
    while (startIndex < tokens.length) {
      const token = tokens[startIndex];
      if (token.startsWith('-')) {
        if (sudoFlagsWithArgs.includes(token) && startIndex + 1 < tokens.length) {
          startIndex += 2;
        } else {
          startIndex++;
        }
      } else {
        break;
      }
    }
    if (startIndex < tokens.length) {
      const remaining = tokens.slice(startIndex).join(' ');
      const inner = unwrapCommand(remaining, depth + 1);
      return {
        unwrapped: inner.unwrapped,
        wrappers: [firstToken, ...inner.wrappers],
      };
    }
  }

  if (isEnvWrapper(firstToken)) {
    const remaining = extractEnvCommand(tokens);
    if (remaining) {
      const inner = unwrapCommand(remaining.join(' '), depth + 1);
      return {
        unwrapped: inner.unwrapped,
        wrappers: [firstToken, ...inner.wrappers],
      };
    }
  }

  if (isInterpreter(firstToken)) {
    const innerCmd = extractInterpreterCommand(tokens, firstToken);
    if (innerCmd) {
      const inner = unwrapCommand(innerCmd, depth + 1);
      return {
        unwrapped: inner.unwrapped,
        wrappers: [firstToken, ...inner.wrappers],
      };
    }
  }

  return { unwrapped: trimmed, wrappers: [] };
}

function expandCombinedFlags(flags: string[]): string[] {
  const expanded: string[] = [];

  for (const flag of flags) {
    if (flag.startsWith('--')) {
      expanded.push(flag);
    } else if (flag.startsWith('-') && flag.length > 2) {
      for (let i = 1; i < flag.length; i++) {
        expanded.push(`-${flag[i]}`);
      }
    } else {
      expanded.push(flag);
    }
  }

  return expanded;
}

function extractSubcommand(command: string, args: string[]): string | undefined {
  const commandsWithSubcommands = [
    'git',
    'npm',
    'yarn',
    'pnpm',
    'docker',
    'kubectl',
    'cargo',
    'go',
  ];

  if (commandsWithSubcommands.includes(command) && args.length > 0) {
    const firstArg = args[0];
    if (!firstArg.startsWith('-') && !firstArg.includes('/') && !firstArg.includes('=')) {
      return firstArg;
    }
  }

  return undefined;
}

function parseFromTokens(tokens: string[], rawCmd: string, wrappers: string[]): ParsedCommand {
  if (tokens.length === 0) {
    return {
      raw: rawCmd,
      command: '',
      subcommand: undefined,
      flags: [],
      args: [],
      unwrapped: rawCmd,
      wrapperChain: wrappers,
    };
  }

  const [command, ...rest] = tokens;

  const rawFlags = rest.filter((t) => t.startsWith('-'));
  const flags = expandCombinedFlags(rawFlags);

  const args = rest.filter((t) => !t.startsWith('-'));
  const subcommand = extractSubcommand(command, args);
  const remainingArgs = subcommand ? args.slice(1) : args;

  return {
    raw: rawCmd,
    command,
    subcommand,
    flags,
    args: remainingArgs,
    unwrapped: tokens.join(' '),
    wrapperChain: wrappers,
  };
}

export function parseCommand(cmd: string): ParsedCommand {
  debug(`parseCommand: raw=${cmd}`);
  const { unwrapped, wrappers } = unwrapCommand(cmd);

  if (wrappers.length > 0) {
    debug('parseCommand: unwrapped through', { wrappers });
  }

  const tokens = tokenise(unwrapped);

  if (tokens.length === 0) {
    debug('parseCommand: empty command after tokenisation');
    return {
      raw: cmd,
      command: '',
      subcommand: undefined,
      flags: [],
      args: [],
      unwrapped,
      wrapperChain: wrappers,
    };
  }

  const [command, ...rest] = tokens;

  const rawFlags = rest.filter((t) => t.startsWith('-'));
  const flags = expandCombinedFlags(rawFlags);

  const args = rest.filter((t) => !t.startsWith('-'));
  const subcommand = extractSubcommand(command, args);
  const remainingArgs = subcommand ? args.slice(1) : args;

  debug('parseCommand result', { command, subcommand, flags });
  return {
    raw: cmd,
    command,
    subcommand,
    flags,
    args: remainingArgs,
    unwrapped,
    wrapperChain: wrappers,
  };
}

export interface CompoundCommandResult {
  isCompound: boolean;
  commands: ParsedCommand[];
  operators: string[];
}

export function parseCompoundCommand(cmd: string): CompoundCommandResult {
  debug(`parseCompoundCommand: raw=${cmd}`);
  const { unwrapped, wrappers } = unwrapCommand(cmd);
  const { isCompound, subCommands } = tokeniseWithOperators(unwrapped);

  if (!isCompound || subCommands.length <= 1) {
    debug('parseCompoundCommand: single command');
    return {
      isCompound: false,
      commands: [parseCommand(cmd)],
      operators: [],
    };
  }

  debug(`parseCompoundCommand: compound with ${subCommands.length} sub-commands`);

  const commands: ParsedCommand[] = [];
  const operators: string[] = [];

  for (const subCmd of subCommands) {
    if (subCmd.tokens.length > 0) {
      const parsed = parseFromTokens(subCmd.tokens, subCmd.tokens.join(' '), wrappers);
      commands.push(parsed);
    }
    if (subCmd.operator) {
      operators.push(subCmd.operator);
    }
  }

  return {
    isCompound: true,
    commands,
    operators,
  };
}

export class CommandParser {
  parse(cmd: string): ParsedCommand {
    return parseCommand(cmd);
  }

  parseCompound(cmd: string): CompoundCommandResult {
    return parseCompoundCommand(cmd);
  }

  parseMultiple(commands: string[]): ParsedCommand[] {
    return commands.map((cmd) => this.parse(cmd));
  }

  parseAllCommands(cmd: string): ParsedCommand[] {
    const result = parseCompoundCommand(cmd);
    return result.commands;
  }

  isWrapped(cmd: string): boolean {
    const result = unwrapCommand(cmd);
    return result.wrappers.length > 0;
  }

  isCompound(cmd: string): boolean {
    const result = parseCompoundCommand(cmd);
    return result.isCompound;
  }

  getWrappers(cmd: string): string[] {
    const result = unwrapCommand(cmd);
    return result.wrappers;
  }
}
