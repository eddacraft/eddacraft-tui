import chalk from 'chalk';

type Log = (message?: unknown, ...optionalParams: unknown[]) => void;

interface ImportsCompletionOptions {
  dryRun: boolean;
  totalErrors: number;
  log?: Log;
  setExitCode?: (code: number) => void;
}

export function printImportsCompletion({
  dryRun,
  totalErrors,
  log = console.log,
  setExitCode = (code) => {
    process.exitCode = code;
  },
}: ImportsCompletionOptions): void {
  if (totalErrors > 0) {
    setExitCode(1);
    log(chalk.red('\n  Codemod completed with errors. Fix the errors above and re-run.\n'));
    return;
  }

  if (dryRun) {
    log(chalk.yellow('\n  [DRY RUN] Run without --dry-run to apply changes.\n'));
  } else {
    log(chalk.green('\n  Changes applied successfully.\n'));
  }
}
