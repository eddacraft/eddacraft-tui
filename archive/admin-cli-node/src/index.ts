#!/usr/bin/env node
import { pathToFileURL } from 'node:url';
import { Command, Option } from 'commander';
import { AdminError } from './client.js';
import { PromptEOFError } from './prompt.js';
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

export function buildProgram(): Command {
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

  // Raise commander's exit requests as exceptions so run() owns the final exit
  // code — keeps the exit-code contract in one place and makes handleError testable.
  program.exitOverride();

  return program;
}

export interface RunDeps {
  argv?: readonly string[];
  stderr?: { write: (chunk: string) => boolean | void };
  exit?: (code: number) => never;
}

function isCommanderError(err: unknown): err is { exitCode: number; code: string } {
  if (typeof err !== 'object' || err === null) return false;
  const candidate = err as { exitCode?: unknown; code?: unknown };
  return (
    typeof candidate.exitCode === 'number' &&
    Number.isInteger(candidate.exitCode) &&
    typeof candidate.code === 'string' &&
    candidate.code.startsWith('commander.')
  );
}

// `exit` is typed `never`, but we still `return exit(...)` so a non-terminating
// stub (tests, embedded use) cannot fall through and double-write or double-exit.
export function handleError(
  err: unknown,
  stderr: { write: (chunk: string) => boolean | void },
  exit: (code: number) => never
): void {
  if (isCommanderError(err)) {
    return exit(err.exitCode);
  }
  if (
    err instanceof AdminError ||
    err instanceof MissingConfigError ||
    err instanceof PromptEOFError
  ) {
    stderr.write(formatError(err.message) + '\n');
    return exit(err.exitCode);
  }
  const message = err instanceof Error ? err.message : String(err);
  stderr.write(formatError(message) + '\n');
  return exit(2);
}

export async function run({
  argv = process.argv,
  stderr = process.stderr,
  exit = process.exit as (code: number) => never,
}: RunDeps = {}): Promise<void> {
  const program = buildProgram();
  try {
    await program.parseAsync([...argv]);
  } catch (err) {
    handleError(err, stderr, exit);
  }
}

const invokedAsBin =
  process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;

if (invokedAsBin) {
  void run();
}
