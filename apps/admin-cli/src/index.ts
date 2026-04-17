#!/usr/bin/env node
import { Command, Option } from 'commander';
import { AdminError } from './client.js';
import { MissingConfigError } from './config.js';
import { formatError } from './format.js';
import { runListCommand, type ListOptions } from './commands/list.js';
import { parseBoundedInt } from './parsers.js';

const program = new Command();

program
  .name('anvil-admin')
  .description('Operator CLI for Anvil beta admin operations')
  .version('0.1.0-beta', '-V, --version', 'print the CLI version')
  .option('--key <key>', 'admin API key (overrides ANVIL_ADMIN_KEY)')
  .option('--url <url>', 'admin API base URL (overrides ANVIL_ADMIN_URL)')
  .option('--actor <actor>', 'operator identity for X-Admin-Actor (overrides ANVIL_ADMIN_ACTOR)');

const notImplemented = (task: string) => () => {
  process.stderr.write(
    formatError(`command not yet implemented (pending ${task})`, { colour: false }) + '\n'
  );
  process.exit(64);
};

program
  .command('list')
  .description('list waitlist entries (default: pending)')
  .addOption(
    new Option('--status <status>', 'waitlist status filter')
      .choices(['pending', 'approved', 'all'])
      .default('pending')
  )
  .addOption(
    new Option('--source <source>', 'waitlist source filter')
      .choices(['manual', 'website', 'import', 'all'])
      .default('all')
  )
  .addOption(
    new Option('--limit <n>', 'page size (1-200)')
      .default(50)
      .argParser(parseBoundedInt('--limit', 1, 200))
  )
  .addOption(
    new Option('--offset <n>', 'page offset (>=0)')
      .default(0)
      .argParser(parseBoundedInt('--offset', 0, Number.MAX_SAFE_INTEGER))
  )
  .option('--json', 'emit raw JSON')
  .action(async (_options, cmd: Command) => {
    await runListCommand(cmd.optsWithGlobals() as ListOptions);
  });

program
  .command('show <email>')
  .description('show user, tokens, and recent audit for an email')
  .option('--json', 'emit raw JSON')
  .action(notImplemented('ADMINCLI-007'));

program
  .command('approve [email]')
  .description('approve a single email or the oldest N pending entries')
  .option('--batch <n>', 'approve the oldest N unapproved entries')
  .option('-y, --yes', 'skip confirmation prompt')
  .action(notImplemented('ADMINCLI-008'));

program
  .command('invite <email>')
  .description('invite an email to the beta')
  .option('--name <name>', 'user display name')
  .option('--notes <text>', 'admin notes')
  .option('--days <n>', 'token validity in days', '90')
  .addOption(
    new Option('--scope <scopes...>', 'token scopes').choices(['beta', 'preview', 'internal'])
  )
  .option('--token-only', 'skip invite email; return raw token once')
  .action(notImplemented('ADMINCLI-009'));

program
  .command('revoke [email]')
  .description('revoke tokens by email or a specific raw token')
  .option('--token <raw>', 'revoke a specific raw token')
  .option('-y, --yes', 'skip confirmation prompt')
  .action(notImplemented('ADMINCLI-010'));

program
  .command('audit')
  .description('browse the audit log')
  .option('--action <action>', 'filter by action (exact match)')
  .option('--filter-actor <actor>', 'filter audit entries by actor email')
  .option('--limit <n>', 'page size (1-200)', '50')
  .option('--offset <n>', 'page offset', '0')
  .option('--json', 'emit raw JSON')
  .action(notImplemented('ADMINCLI-011'));

program
  .command('send-migration')
  .description('send migration email to imported waitlist users')
  .option('--source <source>', 'filter by source', 'import')
  .option('--limit <n>', 'max recipients', '20')
  .option('--dry-run', 'preview recipients without sending')
  .option('-y, --yes', 'skip confirmation prompt')
  .action(notImplemented('ADMINCLI-012'));

program.exitOverride((err) => {
  process.exit(err.exitCode);
});

async function main(): Promise<void> {
  try {
    await program.parseAsync(process.argv);
  } catch (err) {
    if (err instanceof AdminError || err instanceof MissingConfigError) {
      process.stderr.write(formatError(err.message) + '\n');
      process.exit(err.exitCode);
    }
    const message = err instanceof Error ? err.message : String(err);
    process.stderr.write(formatError(message) + '\n');
    process.exit(2);
  }
}

void main();
