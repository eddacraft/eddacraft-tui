/**
 * Platform-default transport path resolution.
 *
 * Mirrors the daemon's `crates/anvil-intercept/src/ipc.rs::resolve_socket_path`
 * (Unix) and `resolve_pipe_name` (Windows) algorithms. Drift between
 * the two would mean the client looks somewhere the daemon never
 * binds, so the resolver is intentionally simple and deterministic.
 *
 * The resolver is platform-conditional via `process.platform` at
 * call-time. Tests on every platform exercise the Unix branch
 * deterministically by injecting an env-var pair via {@link
 * resolveDefaultSocketPath}'s explicit options.
 */

import path from 'node:path';

export type ResolvedPath =
  | { kind: 'unix'; socketPath: string }
  | { kind: 'windows'; pipeName: string };

export interface ResolveOptions {
  platform?: NodeJS.Platform;
  env?: Pick<NodeJS.ProcessEnv, 'XDG_RUNTIME_DIR' | 'HOME' | 'USERPROFILE'>;
  /** When provided, used directly (skips env-var resolution). Lets
   *  consumers point at a non-default daemon (test rigs, side-by-side
   *  installs). */
  socketPath?: string;
  pipeName?: string;
}

export class PathResolutionError extends Error {
  public readonly code: 'no-socket-dir' | 'no-pipe-name' | 'unsupported-platform';
  public constructor(
    code: 'no-socket-dir' | 'no-pipe-name' | 'unsupported-platform',
    message: string
  ) {
    super(message);
    this.name = 'PathResolutionError';
    this.code = code;
  }
}

/**
 * Resolve the default daemon transport path for the current platform.
 * Pure function; no filesystem access.
 *
 * Unix: `$XDG_RUNTIME_DIR/anvil/intercept.sock`, falls back to
 * `$HOME/.local/state/anvil/intercept.sock`.
 *
 * Windows: requires `pipeName` to be supplied — the daemon uses a
 * SID-derived suffix (`anvil-intercept-<sid>`) which Node has no
 * cheap way to fetch in pure JS. Wave 2 ships the daemon on Linux
 * primarily (per the brief's INTD-012 framing for Wave 1); the
 * Windows resolver returns a `no-pipe-name` error if the consumer
 * doesn't pass an explicit override, surfacing the gap honestly
 * rather than guessing.
 */
export function resolveDefaultSocketPath(options: ResolveOptions = {}): ResolvedPath {
  const platform = options.platform ?? process.platform;
  const env = options.env ?? process.env;

  if (options.socketPath !== undefined) {
    return { kind: 'unix', socketPath: options.socketPath };
  }
  if (options.pipeName !== undefined) {
    return { kind: 'windows', pipeName: options.pipeName };
  }

  if (platform === 'win32') {
    throw new PathResolutionError(
      'no-pipe-name',
      'Windows transport requires an explicit pipeName. The daemon uses a SID-derived ' +
        'suffix that this client does not yet auto-resolve — pass DriverClientOptions.pipeName ' +
        'with the value the daemon logs at startup.'
    );
  }

  if (
    platform !== 'linux' &&
    platform !== 'darwin' &&
    platform !== 'freebsd' &&
    platform !== 'openbsd' &&
    platform !== 'sunos' &&
    platform !== 'aix'
  ) {
    throw new PathResolutionError(
      'unsupported-platform',
      `unsupported platform '${platform}' — DriverClient supports Unix domain sockets (Linux/macOS/BSD) and Windows named pipes only`
    );
  }

  const xdg = env.XDG_RUNTIME_DIR;
  if (xdg && xdg.length > 0) {
    return { kind: 'unix', socketPath: path.join(xdg, 'anvil', 'intercept.sock') };
  }
  const home = env.HOME;
  if (home && home.length > 0) {
    return {
      kind: 'unix',
      socketPath: path.join(home, '.local', 'state', 'anvil', 'intercept.sock'),
    };
  }
  throw new PathResolutionError(
    'no-socket-dir',
    'cannot resolve default socket path: $XDG_RUNTIME_DIR is unset and $HOME is unset'
  );
}
