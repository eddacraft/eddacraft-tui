/**
 * TTY detection for TUI availability.
 *
 * Priority order:
 * 1. Explicit --no-tui flag → false
 * 2. Explicit --tui flag → true (warns if unavailable)
 * 3. JSON or quiet mode → false
 * 4. NO_TUI=1 env var → false
 * 5. CI environment → false
 * 6. Non-TTY stdout → false
 * 7. Otherwise → true
 */

export interface TUIDetectionOptions {
  tui?: boolean;
  noTui?: boolean;
  json?: boolean;
  quiet?: boolean;
}

export function isTUIAvailable(options: TUIDetectionOptions = {}): boolean {
  if (options.noTui) {
    return false;
  }

  if (options.tui) {
    if (!process.stdout.isTTY) {
      console.warn('Warning: --tui requested but stdout is not a TTY');
      return false;
    }
    return true;
  }

  if (options.json || options.quiet) {
    return false;
  }

  if (process.env['NO_TUI'] === '1' || process.env['NO_TUI'] === 'true') {
    return false;
  }

  if (process.env['CI'] === 'true' || process.env['CI'] === '1') {
    return false;
  }

  if (process.env['GITHUB_ACTIONS'] === 'true') {
    return false;
  }

  if (!process.stdout.isTTY) {
    return false;
  }

  return true;
}

export function getTerminalSize(): { columns: number; rows: number } {
  return {
    columns: process.stdout.columns || 80,
    rows: process.stdout.rows || 24,
  };
}

export function supportsColour(): boolean {
  if (process.env['NO_COLOR'] !== undefined) {
    return false;
  }

  if (process.env['FORCE_COLOR'] !== undefined) {
    return true;
  }

  return process.stdout.isTTY ?? false;
}
