import { Command } from 'commander';
import chalk from 'chalk';
import { loadAuth } from '../services/auth-store.js';
import { error } from '../utils/output.js';

export function createWhoamiCommand(): Command {
  const command = new Command('whoami');

  command.description('Display current authentication session info').action(() => {
    const auth = loadAuth();

    if (!auth) {
      error('Not authenticated. Run `anvil login` to authenticate.');
      process.exit(1);
    }

    console.log(chalk.bold('\nSession Info\n'));
    console.log(`  Email:    ${chalk.cyan(auth.user.email)}`);
    console.log(`  Scopes:   ${auth.scopes.join(', ')}`);
    console.log(`  Expires:  ${new Date(auth.expiresAt).toLocaleString()}`);
    console.log(`  Verified: ${new Date(auth.verifiedAt).toLocaleString()}`);
    console.log('');
  });

  return command;
}
