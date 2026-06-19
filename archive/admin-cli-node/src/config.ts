import { execFileSync } from 'node:child_process';
import os from 'node:os';

export interface AdminConfig {
  url: string;
  key: string;
  actor: string;
}

export interface ConfigFlags {
  key?: string;
  url?: string;
  actor?: string;
}

export interface ResolveContext {
  env?: NodeJS.ProcessEnv;
  getGitEmail?: () => string | undefined;
  getOsUser?: () => string;
}

export const DEFAULT_URL = 'https://api.eddacraft.ai';

export class MissingConfigError extends Error {
  readonly exitCode = 5;
  constructor(message: string) {
    super(message);
    this.name = 'MissingConfigError';
  }
}

function defaultGitEmail(): string | undefined {
  try {
    const out = execFileSync('git', ['config', '--get', 'user.email'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
      timeout: 500,
    });
    const trimmed = out.trim();
    return trimmed || undefined;
  } catch {
    return undefined;
  }
}

function defaultOsUser(): string {
  return os.userInfo().username;
}

export function resolveConfig(flags: ConfigFlags = {}, ctx: ResolveContext = {}): AdminConfig {
  const env = ctx.env ?? process.env;
  const getGitEmail = ctx.getGitEmail ?? defaultGitEmail;
  const getOsUser = ctx.getOsUser ?? defaultOsUser;

  const url = flags.url ?? env.ANVIL_ADMIN_URL ?? DEFAULT_URL;
  const key = flags.key ?? env.ANVIL_ADMIN_KEY;
  if (!key) {
    throw new MissingConfigError('missing admin key; set ANVIL_ADMIN_KEY or pass --key');
  }

  const actor = flags.actor ?? env.ANVIL_ADMIN_ACTOR ?? getGitEmail() ?? getOsUser();

  return { url, key, actor };
}
