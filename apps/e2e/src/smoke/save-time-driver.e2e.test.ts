import { afterEach, describe, expect, it } from 'vitest';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync } from 'node:fs';
import { basename, join } from 'node:path';
import { tmpdir } from 'node:os';
import { cliBinaryAvailable, resolveCliBinary, runCli } from '../helpers/cli-runner.js';
import { createE2EWorkspace, type E2EWorkspace } from '../helpers/workspace.js';

const describeCli = cliBinaryAvailable() ? describe : describe.skip;

interface IsolatedEnv {
  root: string;
  runtime: string;
  env: Record<string, string>;
}

type JsonObject = Record<string, unknown>;

const workspaces: E2EWorkspace[] = [];
const homes: IsolatedEnv[] = [];
const daemons: ChildProcessWithoutNullStreams[] = [];

function workspace(name = 'src/index.ts'): E2EWorkspace {
  const ws = createE2EWorkspace({
    withAnvilrc: false,
    withGit: true,
    files: {
      [name]: 'export const value = 1;\n',
    },
  });
  workspaces.push(ws);
  return ws;
}

function isolatedEnv(): IsolatedEnv {
  const root = mkdtempSync(join(tmpdir(), 'anvil-e2e-dsv051-home-'));
  const runtime = join(root, 'runtime');
  const env = {
    HOME: root,
    USERPROFILE: root,
    LOCALAPPDATA: join(root, 'local-app-data'),
    XDG_CONFIG_HOME: join(root, 'xdg'),
    XDG_RUNTIME_DIR: runtime,
    ANVIL_HOME: '',
    ANVIL_DEV: '1',
    ANVIL_SKIP_WELCOME: '1',
    ANVIL_NO_PROMPT: '1',
  };
  const home = { root, runtime, env };
  homes.push(home);
  return home;
}

async function stopDaemon(cwd: string, env: Record<string, string>): Promise<string> {
  const result = await runCli(['intercept', 'stop'], { cwd, env, timeout: 20_000 });
  expect(result.exitCode, result.output).toBe(0);
  return result.output;
}

async function waitForExit(child: ChildProcessWithoutNullStreams): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) return;
  await Promise.race([
    new Promise<void>((resolve) => child.once('exit', () => resolve())),
    new Promise<void>((resolve) => setTimeout(resolve, 5_000)),
  ]);
}

async function stopManagedDaemons(): Promise<void> {
  while (daemons.length > 0) {
    const child = daemons.pop();
    if (!child) continue;
    await waitForExit(child);
    if (child.exitCode === null && child.signalCode === null) {
      child.kill('SIGTERM');
      await waitForExit(child);
    }
  }
}

async function startForegroundDaemon(cwd: string, env: Record<string, string>): Promise<void> {
  const binary = resolveCliBinary();
  if (!binary) throw new Error('anvil CLI binary not built');
  const child = spawn(binary, ['intercept', 'start', '--foreground'], {
    cwd,
    env: {
      ...process.env,
      ...env,
      NO_COLOR: '1',
      FORCE_COLOR: '0',
      CI: 'true',
      NO_TUI: '1',
    },
  });
  daemons.push(child);

  let stderr = '';
  child.stderr.on('data', (chunk) => {
    stderr += chunk.toString();
  });

  await waitFor(async () => {
    if (child.exitCode !== null) {
      throw new Error(`daemon exited with ${child.exitCode}: ${stderr}`);
    }
    const result = await runCli(['intercept', 'status', '--json'], { cwd, env, timeout: 5_000 });
    return result.exitCode === 0 ? result.stdout : undefined;
  }, 'foreground intercept daemon to answer status');
}

afterEach(async () => {
  for (const home of homes) {
    await runCli(['intercept', 'stop'], { env: home.env, timeout: 20_000 });
  }
  await stopManagedDaemons();
  while (workspaces.length > 0) {
    workspaces.pop()?.cleanup();
  }
  while (homes.length > 0) {
    const home = homes.pop();
    if (home && existsSync(home.root)) {
      rmSync(home.root, { recursive: true, force: true });
    }
  }
});

async function waitFor<T>(
  probe: () => Promise<T | undefined>,
  label: string,
  timeoutMs = 20_000
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;
  while (Date.now() < deadline) {
    try {
      const value = await probe();
      if (value !== undefined) return value;
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`${label} did not become true in time${lastError ? `: ${lastError}` : ''}`);
}

async function statusJson(cwd: string, env: Record<string, string>): Promise<JsonObject> {
  const result = await runCli(['status', '--json'], { cwd, env, timeout: 20_000 });
  expect(result.exitCode, result.output).toBe(0);
  return JSON.parse(result.stdout) as JsonObject;
}

async function interceptStatusJson(cwd: string, env: Record<string, string>): Promise<JsonObject> {
  const result = await runCli(['intercept', 'status', '--json'], { cwd, env, timeout: 20_000 });
  expect(result.exitCode, result.output).toBe(0);
  return JSON.parse(result.stdout) as JsonObject;
}

function worktreeEntries(status: JsonObject): JsonObject[] {
  const entries = status.worktrees;
  return Array.isArray(entries) ? (entries as JsonObject[]) : [];
}

function driverState(status: JsonObject, root: string): string | undefined {
  const canonical = realpathSync(root);
  const entry = worktreeEntries(status).find((worktree) => worktree.worktree === canonical);
  const state = entry?.save_time_driver;
  return typeof state === 'string' ? state : undefined;
}

async function waitForDriverState(
  root: string,
  env: Record<string, string>,
  expected: string
): Promise<void> {
  await waitFor(async () => {
    const status = await statusJson(root, env);
    return status.save_time_driver === expected ? expected : undefined;
  }, `save_time_driver=${expected} for ${root}`);
}

function driverLogPath(root: string, runtime: string): string {
  const canonical = realpathSync(root);
  const hash = createHash('sha256').update(canonical).digest('hex').slice(0, 12);
  const leaf = basename(canonical).replace(/[\\/: ]/g, '-');
  return join(runtime, 'anvil', 'save-time-drivers', `${leaf}-${hash}.log`);
}

function driverDirectoryDebug(runtime: string): string {
  const dir = join(runtime, 'anvil', 'save-time-drivers');
  if (!existsSync(dir)) return `${dir} does not exist`;
  return readdirSync(dir)
    .map((entry) => {
      const path = join(dir, entry);
      const content = readFileSync(path, 'utf-8');
      return `${entry}: ${content.slice(0, 2_000)}`;
    })
    .join('\n---\n');
}

describeCli('Smoke › save-time background driver', () => {
  it('starts under --no-mcp, attaches a driver, and records planted findings in the driver log', async () => {
    const ws = workspace();
    const home = isolatedEnv();

    await startForegroundDaemon(ws.root, home.env);

    const start = await runCli(['--no-tui', 'start', '--no-mcp'], {
      cwd: ws.root,
      env: home.env,
      timeout: 30_000,
    });

    expect(start.exitCode, start.output).toBe(0);
    expect(start.stdout).not.toContain('anvil watch');

    await waitForDriverState(ws.root, home.env, 'attached');

    const logPath = driverLogPath(ws.root, home.runtime);
    let writeAttempt = 0;
    let lastWrite = 0;

    const log = await waitFor(
      () => {
        if (Date.now() - lastWrite > 1_500) {
          writeAttempt += 1;
          lastWrite = Date.now();
          ws.writeFile('src/index.ts', `const value${writeAttempt}: any = source;\n`);
        }
        if (!existsSync(logPath)) {
          throw new Error(driverDirectoryDebug(home.runtime));
        }
        const content = readFileSync(logPath, 'utf-8');
        if (!content.includes('AP-003')) {
          throw new Error(driverDirectoryDebug(home.runtime));
        }
        return Promise.resolve(content);
      },
      'driver findings log with planted antipattern finding',
      60_000
    );

    expect(log).toContain('src/index.ts');
    expect(log).toContain('finding(s)');
  });

  it('registers a second worktree once, reattaches after daemon restart, and stop warns about lost protection', async () => {
    const first = workspace('src/first.ts');
    const second = workspace('src/second.ts');
    const home = isolatedEnv();

    await startForegroundDaemon(first.root, home.env);

    const start = await runCli(['--no-tui', 'start', '--no-mcp'], {
      cwd: first.root,
      env: home.env,
      timeout: 30_000,
    });
    expect(start.exitCode, start.output).toBe(0);
    await waitForDriverState(first.root, home.env, 'attached');

    const register = await runCli(['workspace', 'register', second.root], {
      cwd: first.root,
      env: home.env,
      timeout: 20_000,
    });
    expect(register.exitCode, register.output).toBe(0);

    await waitFor(
      async () => {
        const status = await interceptStatusJson(first.root, home.env);
        const attached = new Set(
          worktreeEntries(status)
            .filter((entry) => entry.save_time_driver === 'attached')
            .map((entry) => entry.worktree)
        );
        return attached.has(realpathSync(first.root)) && attached.has(realpathSync(second.root))
          ? attached
          : undefined;
      },
      'two distinct attached save-time drivers',
      60_000
    );

    const duplicate = await runCli(['workspace', 'register', second.root], {
      cwd: first.root,
      env: home.env,
      timeout: 20_000,
    });
    expect(duplicate.exitCode, duplicate.output).toBe(0);

    const deduped = await interceptStatusJson(first.root, home.env);
    const distinctAttached = new Set(
      worktreeEntries(deduped)
        .filter((entry) => entry.save_time_driver === 'attached')
        .map((entry) => entry.worktree)
    );
    expect(distinctAttached.size).toBe(2);

    const stop = await stopDaemon(first.root, home.env);
    expect(stop).toContain('will lose protection');
    await stopManagedDaemons();

    await startForegroundDaemon(first.root, home.env);

    const restart = await runCli(['--no-tui', 'start', '--no-mcp'], {
      cwd: first.root,
      env: home.env,
      timeout: 30_000,
    });
    expect(restart.exitCode, restart.output).toBe(0);

    await waitFor(
      async () => {
        const status = await interceptStatusJson(first.root, home.env);
        return driverState(status, first.root) === 'attached' &&
          driverState(status, second.root) === 'attached'
          ? status
          : undefined;
      },
      'registered save-time drivers reattached after daemon restart',
      60_000
    );
  });

  it('honours ANVIL_NO_SAVE_TIME_DRIVER while keeping worktree registration visible', async () => {
    const ws = workspace();
    const home = isolatedEnv();
    const env = { ...home.env, ANVIL_NO_SAVE_TIME_DRIVER: '1' };

    await startForegroundDaemon(ws.root, env);

    const start = await runCli(['--no-tui', 'start', '--no-mcp'], {
      cwd: ws.root,
      env,
      timeout: 30_000,
    });
    expect(start.exitCode, start.output).toBe(0);

    await waitForDriverState(ws.root, env, 'absent');
    const status = await interceptStatusJson(ws.root, env);
    expect(worktreeEntries(status).some((entry) => entry.worktree === realpathSync(ws.root))).toBe(
      true
    );
  });
});
