/**
 * Integration test: real daemon binary roundtrip.
 *
 * Per DRVR-001 brief:
 * > Integration test connects to a real daemon binary and exercises
 * > `session.register` + `session.heartbeat` — this requires
 * > `cargo build -p eddacraft-anvil-intercept` and spawning the
 * > binary. The test should be `pnpm`-runnable from the package and
 * > gated to skip gracefully if the daemon binary cannot be located.
 *
 * Skip behaviour:
 *   - We look for the daemon binary at the cargo workspace's
 *     `target/debug/eddacraft-anvil-intercept` (or `target/release/...`).
 *   - If neither is present, we skip the test rather than fail. The
 *     standard validation gate runs without the daemon binary; CI is
 *     responsible for building Rust before running this suite.
 *
 * Skipped on Windows for now — the daemon binary path resolution and
 * pipe-name handshake on Windows belong to a follow-up alongside
 * INTD-012 Windows CI matrix work.
 */

import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vitest';

import { DriverClient } from '../client/driver-client.js';

const dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(dirname, '../../../..');

function findDaemonBinary(): string | null {
  // The cargo crate is named `eddacraft-anvil-intercept`, but its
  // emitted binary is `anvil-intercept` (see `[[bin]]` in
  // `crates/anvil-intercept/Cargo.toml`).
  //
  // When the suite runs from a worktree, `target/` may live in the
  // primary checkout — try the worktree first, then walk up to the
  // primary repo.
  const candidates = [
    process.env.ANVIL_INTERCEPT_BIN ?? '',
    path.join(repoRoot, 'target', 'debug', 'anvil-intercept'),
    path.join(repoRoot, 'target', 'release', 'anvil-intercept'),
    // Worktree fallback: if `repoRoot` is `.../.worktrees/<name>`,
    // walk up to the primary checkout's `target/`.
    path.join(repoRoot, '..', '..', 'target', 'debug', 'anvil-intercept'),
    path.join(repoRoot, '..', '..', 'target', 'release', 'anvil-intercept'),
  ];
  for (const candidate of candidates) {
    if (candidate.length === 0) {
      continue;
    }
    if (fs.existsSync(candidate)) {
      return path.resolve(candidate);
    }
  }
  return null;
}

const daemonBinary = findDaemonBinary();
const isUnix = process.platform !== 'win32';
const shouldRun = isUnix && daemonBinary !== null;

const skipMessage = !isUnix
  ? 'integration test only runs on Unix-like platforms'
  : 'integration test skipped: daemon binary not built — run `cargo build -p eddacraft-anvil-intercept` (binary lands at target/debug/anvil-intercept) or set ANVIL_INTERCEPT_BIN';

describe.skipIf(!shouldRun)('DriverClient — integration: real daemon', () => {
  let runtimeDir: string | null = null;
  let socketPath: string | null = null;
  let daemonProcess: ChildProcess | null = null;

  beforeAll(async () => {
    if (!shouldRun) {
      return;
    }
    runtimeDir = fs.mkdtempSync(path.join(os.tmpdir(), 'anvil-intg-'));
    fs.chmodSync(runtimeDir, 0o700);
    const anvilDir = path.join(runtimeDir, 'anvil');
    fs.mkdirSync(anvilDir, { mode: 0o700 });
    socketPath = path.join(anvilDir, 'intercept.sock');

    const env = {
      ...process.env,
      XDG_RUNTIME_DIR: runtimeDir,
      // Ensure the daemon doesn't trip on stray HOME-rooted state.
      HOME: runtimeDir,
    };

    daemonProcess = spawn(daemonBinary!, ['start'], {
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    // Surface daemon stderr if the test fails so debugging is possible
    // without re-running with manual instrumentation.
    daemonProcess.stderr?.on('data', (chunk: Buffer) => {
      process.stderr.write(`[daemon] ${chunk.toString('utf8')}`);
    });

    // Wait for the socket to appear.
    const deadline = Date.now() + 5_000;
    while (!fs.existsSync(socketPath) && Date.now() < deadline) {
      await new Promise((r) => setTimeout(r, 50));
    }
    if (!fs.existsSync(socketPath)) {
      throw new Error(
        `daemon did not bind socket at ${socketPath} within 5s — check the binary is current`
      );
    }
  }, 15_000);

  afterEach(() => {
    // Each test instantiates its own client; cleanup happens inside
    // the test body. Nothing to do at this level.
  });

  afterAll(async () => {
    if (daemonProcess !== null) {
      daemonProcess.kill('SIGTERM');
      // Give it a moment to clean up its socket.
      await new Promise((r) => setTimeout(r, 200));
      if (!daemonProcess.killed) {
        daemonProcess.kill('SIGKILL');
      }
    }
    if (runtimeDir !== null) {
      fs.rmSync(runtimeDir, { recursive: true, force: true });
    }
  });

  it('round-trips session.register and session.heartbeat', async () => {
    expect(socketPath).not.toBeNull();
    expect(runtimeDir).not.toBeNull();
    // The daemon canonicalises the worktree path before recording it,
    // so the path must exist on disk for `session.register` to
    // succeed.
    const worktree = path.join(runtimeDir!, 'integration-worktree');
    fs.mkdirSync(worktree, { recursive: true });

    const client = new DriverClient({ socketPath: socketPath! });
    try {
      await client.connect();
      const reg = (await client.request('session.register', {
        session_id: 'integration-session-1',
        worktree,
      })) as { ok: boolean };
      expect(reg.ok).toBe(true);

      const hb = (await client.request('session.heartbeat', {
        session_id: 'integration-session-1',
      })) as { ok: boolean };
      expect(hb.ok).toBe(true);

      const list = (await client.request('session.list')) as Array<{
        id: { 0: string } | string;
      }>;
      expect(list.length).toBeGreaterThan(0);
    } finally {
      await client.close();
    }
  });

  it('surfaces daemon error responses as anvil-daemon-error', async () => {
    expect(socketPath).not.toBeNull();
    const client = new DriverClient({ socketPath: socketPath! });
    try {
      await client.connect();
      let err: unknown;
      try {
        await client.request('session.unknown-method');
      } catch (e) {
        err = e;
      }
      expect((err as { code?: string }).code).toBe('anvil-daemon-error');
    } finally {
      await client.close();
    }
  });
});

if (!shouldRun) {
  describe('DriverClient — integration: real daemon (skipped)', () => {
    it('would run if the daemon binary were present', () => {
      // Surface the skip reason via a side-channel; suite-level
      // skipIf hides this otherwise.
      console.warn(`[integration] ${skipMessage}`);
      expect(true).toBe(true);
    });
  });
}
