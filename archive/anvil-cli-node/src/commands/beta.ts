import { Command } from 'commander';
import chalk from 'chalk';
import { z } from 'zod';
import { adminInvite, adminRevoke } from '../services/admin-client.js';
import { success, error, info, blank, print } from '../utils/output.js';
import { CliError } from '../utils/cli-error.js';
import { coercePositiveInt } from '../utils/option-coerce.js';

const emailSchema = z.string().email();

export function createBetaCommand(): Command {
  const command = new Command('beta');

  command.description('Beta access management (requires ANVIL_ADMIN_KEY)');

  command
    .command('invite')
    .description('Invite a user and generate a beta access token')
    .requiredOption('--email <email>', 'User email address')
    .option('--days <days>', 'Token validity in days', '90')
    .option('--name <name>', 'User display name')
    .option('--notes <notes>', 'Internal notes about this user')
    .action(async (options: { email: string; days: string; name?: string; notes?: string }) => {
      const days = coercePositiveInt(options.days, '--days');

      if (!emailSchema.safeParse(options.email).success) {
        error('--email must be a valid email address');
        throw new CliError('--email must be a valid email address');
      }

      try {
        const result = await adminInvite({
          email: options.email,
          days,
          name: options.name,
          notes: options.notes,
        });

        success('Beta access token created');
        blank();
        print(`  ${chalk.bold('User:')}     ${result.user.email}`);
        print(`  ${chalk.bold('Scopes:')}   ${result.scopes.join(', ')}`);
        print(`  ${chalk.bold('Expires:')}  ${new Date(result.expiresAt).toLocaleString()}`);
        blank();
        print(chalk.yellow('  Token (share with user — shown only once):'));
        blank();
        print(`  ${chalk.cyan(result.token)}`);
        blank();
      } catch (err) {
        error(err instanceof Error ? err.message : 'Failed to create invite');
        throw new CliError('Failed to create invite');
      }
    });

  command
    .command('revoke')
    .description('Revoke all beta access tokens for a user')
    .requiredOption('--email <email>', 'User email address')
    .action(async (options: { email: string }) => {
      if (!emailSchema.safeParse(options.email).success) {
        error('--email must be a valid email address');
        throw new CliError('--email must be a valid email address');
      }

      try {
        const result = await adminRevoke(options.email);

        if (result.revoked > 0) {
          success(`Revoked ${result.revoked} token(s) for ${options.email}`);
        } else {
          info(`No active tokens found for ${options.email}`);
        }
      } catch (err) {
        error(err instanceof Error ? err.message : 'Failed to revoke tokens');
        throw new CliError('Failed to revoke tokens');
      }
    });

  return command;
}
