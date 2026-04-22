/**
 * Interactive prompt helper shared across commands that need operator
 * confirmation (approve, revoke).
 *
 * Reads from stdin, writes the prompt to stderr so it does not pollute
 * piped stdout (e.g. `anvil-admin revoke foo | jq`).
 */
/**
 * Thrown when stdin closes (EOF) before the operator types a response.
 * Exit code 4 matches the non-TTY guard so CI jobs see a clear failure.
 */
export class PromptEOFError extends Error {
  readonly exitCode = 4;
  constructor() {
    super('unexpected EOF on prompt — pass --yes for non-interactive execution');
    this.name = 'PromptEOFError';
  }
}

export async function defaultPrompt(message: string): Promise<string> {
  const { createInterface } = await import('node:readline/promises');

  return new Promise<string>((resolve, reject) => {
    const rl = createInterface({ input: process.stdin, output: process.stderr });
    let answered = false;

    rl.once('close', () => {
      if (!answered) {
        reject(new PromptEOFError());
      }
    });

    rl.question(message)
      .then((answer) => {
        answered = true;
        rl.close();
        resolve(answer);
      })
      .catch((err) => {
        rl.close();
        reject(err);
      });
  });
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
