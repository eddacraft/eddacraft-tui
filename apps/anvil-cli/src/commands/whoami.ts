import { Command } from 'commander';
import chalk from 'chalk';
import { loadAuth } from '../services/auth-store.js';
import { blank, error, print } from '../utils/output.js';
import { CliError } from '../utils/cli-error.js';

export function createWhoamiCommand(): Command {
  const command = new Command('whoami');

  command.description('Display current authentication session info').action(() => {
    const auth = loadAuth();

    if (!auth) {
      error('Not authenticated. Run `anvil login` to authenticate.');
      throw new CliError('Not authenticated');
    }

    print(chalk.bold('\nSession Info\n'));
    print(`  Email:    ${chalk.cyan(auth.user.email)}`);
    print(`  Scopes:   ${auth.scopes.join(', ')}`);
    print(`  Expires:  ${new Date(auth.expiresAt).toLocaleString()}`);
    print(`  Verified: ${new Date(auth.verifiedAt).toLocaleString()}`);
    blank();
  });

  return command;
}
