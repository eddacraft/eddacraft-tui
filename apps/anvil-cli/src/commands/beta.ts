import { Command } from 'commander';
import chalk from 'chalk';
import { z } from 'zod';
import { adminInvite, adminRevoke } from '../services/admin-client.js';
import { success, error, info } from '../utils/output.js';

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
      const days = parseInt(options.days, 10);
      if (Number.isNaN(days) || days <= 0) {
        error('--days must be a positive integer');
        process.exit(1);
      }

      if (!emailSchema.safeParse(options.email).success) {
        error('--email must be a valid email address');
        process.exit(1);
      }

      try {
        const result = await adminInvite({
          email: options.email,
          days,
          name: options.name,
          notes: options.notes,
        });

        success('Beta access token created');
        console.log('');
        console.log(`  ${chalk.bold('User:')}     ${result.user.email}`);
        console.log(`  ${chalk.bold('Scopes:')}   ${result.scopes.join(', ')}`);
        console.log(`  ${chalk.bold('Expires:')}  ${new Date(result.expiresAt).toLocaleString()}`);
        console.log('');
        console.log(chalk.yellow('  Token (share with user — shown only once):'));
        console.log('');
        console.log(`  ${chalk.cyan(result.token)}`);
        console.log('');
      } catch (err) {
        error(err instanceof Error ? err.message : 'Failed to create invite');
        process.exit(1);
      }
    });

  command
    .command('revoke')
    .description('Revoke all beta access tokens for a user')
    .requiredOption('--email <email>', 'User email address')
    .action(async (options: { email: string }) => {
      if (!emailSchema.safeParse(options.email).success) {
        error('--email must be a valid email address');
        process.exit(1);
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
        process.exit(1);
      }
    });

  return command;
}
