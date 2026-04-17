#!/usr/bin/env node
import { Command, Option } from 'commander';
import { AdminError } from './client.js';
import { MissingConfigError } from './config.js';
import { formatError } from './format.js';
import { runListCommand, type ListOptions } from './commands/list.js';
import { runShowCommand, type ShowOptions } from './commands/show.js';
import { runApproveCommand, type ApproveOptions } from './commands/approve.js';
import { runInviteCommand, type InviteOptions, ALLOWED_SCOPES } from './commands/invite.js';
import { runAuditCommand, type AuditOptions } from './commands/audit.js';
import { runRevokeCommand, type RevokeOptions } from './commands/revoke.js';
import {
  runSendMigrationCommand,
  type SendMigrationOptions,
  MIGRATION_SOURCES,
} from './commands/send-migration.js';
import { parseBoundedInt } from './parsers.js';

const program = new Command();

program
  .name('anvil-admin')
  .description('Operator CLI for Anvil beta admin operations')
  .version('0.1.0-beta', '-V, --version', 'print the CLI version')
  .option('--key <key>', 'admin API key (overrides ANVIL_ADMIN_KEY)')
  .option('--url <url>', 'admin API base URL (overrides ANVIL_ADMIN_URL)')
  .option('--actor <actor>', 'operator identity for X-Admin-Actor (overrides ANVIL_ADMIN_ACTOR)');

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
  .action(async (email: string, _options, cmd: Command) => {
    await runShowCommand(email, cmd.optsWithGlobals() as ShowOptions);
  });

program
  .command('approve [email]')
  .description('approve a single email or the oldest N pending entries')
  .addOption(
    new Option('--batch <n>', 'approve the oldest N unapproved entries').argParser(
      parseBoundedInt('--batch', 1, 100)
    )
  )
  .option('-y, --yes', 'skip confirmation prompt')
  .option('--json', 'emit raw JSON')
  .action(async (email: string | undefined, _options, cmd: Command) => {
    await runApproveCommand(email, cmd.optsWithGlobals() as ApproveOptions);
  });

program
  .command('invite <email>')
  .description('invite an email to the beta')
  .option('--name <name>', 'user display name')
  .option('--notes <text>', 'admin notes')
  .addOption(
    new Option('--days <n>', 'token validity in days')
      .default(90)
      .argParser(parseBoundedInt('--days', 1, 365))
  )
  .addOption(new Option('--scope <scopes...>', 'token scopes').choices([...ALLOWED_SCOPES]))
  .option('--token-only', 'skip invite email; return raw token once')
  .option('--json', 'emit raw JSON')
  .action(async (email: string, _options, cmd: Command) => {
    await runInviteCommand(email, cmd.optsWithGlobals() as InviteOptions);
  });

program
  .command('revoke [email]')
  .description('revoke tokens by email or a specific raw token')
  .option('--token <raw>', 'revoke a specific raw token')
  .option('-y, --yes', 'skip confirmation prompt')
  .option('--json', 'emit raw JSON')
  .action(async (email: string | undefined, _options, cmd: Command) => {
    await runRevokeCommand(email, cmd.optsWithGlobals() as RevokeOptions);
  });

program
  .command('audit')
  .description('browse the audit log')
  .option('--action <action>', 'filter by action (exact match)')
  .option('--filter-actor <actor>', 'filter audit entries by actor email')
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
    await runAuditCommand(cmd.optsWithGlobals() as AuditOptions);
  });

program
  .command('send-migration')
  .description('send migration email to waitlist users from a selected source (default: import)')
  .addOption(
    new Option('--source <source>', 'filter by source')
      .choices([...MIGRATION_SOURCES])
      .default('import')
  )
  .addOption(
    new Option('--limit <n>', 'max recipients (1-100)')
      .default(20)
      .argParser(parseBoundedInt('--limit', 1, 100))
  )
  .option('--no-dry-run', 'actually send (default is to preview only)')
  .option('-y, --yes', 'skip confirmation prompt')
  .option('--json', 'emit raw JSON')
  .action(async (_options, cmd: Command) => {
    await runSendMigrationCommand(cmd.optsWithGlobals() as SendMigrationOptions);
  });

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
