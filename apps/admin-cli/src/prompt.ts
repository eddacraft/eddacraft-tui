/**
 * Interactive prompt helper shared across commands that need operator
 * confirmation (approve, revoke).
 *
 * Reads from stdin, writes the prompt to stderr so it does not pollute
 * piped stdout (e.g. `anvil-admin revoke foo | jq`).
 */
export async function defaultPrompt(message: string): Promise<string> {
  const { createInterface } = await import('node:readline/promises');
  const rl = createInterface({ input: process.stdin, output: process.stderr });
  try {
    return await rl.question(message);
  } finally {
    rl.close();
  }
}

/**
 * Returns true only when BOTH streams actually used by `defaultPrompt`
 * (stdin for input, stderr for output) are TTYs. Using stdout.isTTY is
 * wrong: piping stdout (common with --json | jq) would incorrectly
 * report non-interactive even when the operator can still type.
 */
export function isInteractiveTTY(): boolean {
  return !!(process.stdin.isTTY && process.stderr.isTTY);
}
