import { Command } from 'commander';
import chalk from 'chalk';
import { loadAuth } from '../services/auth-store.js';
import { loadLicence, resolveLicencePath } from '../services/licence-store.js';
import { verifyLicence } from '../services/licence-verifier.js';
import { blank, error, print } from '../utils/output.js';
import { CliError } from '../utils/cli-error.js';

export function createWhoamiCommand(): Command {
  const command = new Command('whoami');

  command.description('Display current authentication and licence info').action(async () => {
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

    // Licence info
    const jwt = loadLicence(process.cwd());
    if (jwt) {
      const result = await verifyLicence(jwt);
      if (result.valid) {
        blank();
        print(chalk.bold('Licence\n'));
        print(`  Tier:       ${chalk.cyan(result.claims.tier)}`);
        print(`  Org:        ${result.claims.org ?? chalk.dim('none')}`);
        if (result.claims.identity?.id) {
          print(`  Identity:   ${result.claims.identity.provider}:${result.claims.identity.id}`);
        }
        print(`  Expires:    ${new Date(result.claims.exp * 1000).toLocaleString()}`);
        const rcDate = new Date(result.claims.rcAfter * 1000);
        const daysUntilCheck = Math.max(0, Math.ceil((rcDate.getTime() - Date.now()) / 86400000));
        print(
          `  Next check: ${rcDate.toLocaleDateString()}${daysUntilCheck > 0 ? ` (in ${daysUntilCheck} days)` : chalk.yellow(' (pending)')}`
        );

        const licPath = resolveLicencePath(process.cwd());
        if (licPath) print(`  Licence:    ${chalk.dim(licPath)}`);
      } else {
        blank();
        print(chalk.yellow(`  Licence: invalid (${result.reason})`));
      }
    } else {
      blank();
      print(chalk.dim('  No licence file found'));
    }

    blank();
  });

  return command;
}
