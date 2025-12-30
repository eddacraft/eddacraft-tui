import { existsSync, mkdirSync, writeFileSync } from 'fs';
import { join } from 'path';

const ANVIL_DIR = '.anvil';
const FIRST_RUN_MARKER = 'first-run';
const SKIP_WELCOME_ENV = 'ANVIL_SKIP_WELCOME';

export interface FirstRunOptions {
  projectRoot?: string;
}

export function isFirstRun(options: FirstRunOptions = {}): boolean {
  if (process.env[SKIP_WELCOME_ENV] === '1' || process.env[SKIP_WELCOME_ENV] === 'true') {
    return false;
  }

  const projectRoot = options.projectRoot ?? process.cwd();
  const markerPath = join(projectRoot, ANVIL_DIR, FIRST_RUN_MARKER);

  return !existsSync(markerPath);
}

export function markFirstRunComplete(options: FirstRunOptions = {}): void {
  const projectRoot = options.projectRoot ?? process.cwd();
  const anvilDir = join(projectRoot, ANVIL_DIR);
  const markerPath = join(anvilDir, FIRST_RUN_MARKER);

  if (!existsSync(anvilDir)) {
    mkdirSync(anvilDir, { recursive: true });
  }

  const content = JSON.stringify(
    {
      createdAt: new Date().toISOString(),
      version: '1.0.0',
    },
    null,
    2
  );

  writeFileSync(markerPath, content, 'utf-8');
}

export function isWelcomeSkipped(): boolean {
  return process.env[SKIP_WELCOME_ENV] === '1' || process.env[SKIP_WELCOME_ENV] === 'true';
}

export function getMarkerPath(options: FirstRunOptions = {}): string {
  const projectRoot = options.projectRoot ?? process.cwd();
  return join(projectRoot, ANVIL_DIR, FIRST_RUN_MARKER);
}
