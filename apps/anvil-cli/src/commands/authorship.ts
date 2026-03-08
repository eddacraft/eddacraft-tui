import { Command } from 'commander';
import chalk from 'chalk';
import {
  readAuthorshipNote,
  listAuthorshipNotes,
  getAuthorshipStats,
  type AuthorshipLog,
} from '@eddacraft/anvil-core';
import { theme } from '../tui/utils/theme.js';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { coercePositiveInt } from '../utils/option-coerce.js';
import { blank, json, print } from '../utils/output.js';

interface AuthorshipShowOptions {
  json?: boolean;
}

interface AuthorshipListOptions {
  limit?: string;
  json?: boolean;
}

interface AuthorshipStatsOptions {
  json?: boolean;
}

/**
 * Format an AuthorshipLog for display
 */
function formatAuthorshipLog(log: AuthorshipLog): void {
  print(chalk.bold('\nAI Authorship Log'));
  print(chalk.hex(theme.colours.smoke)('─'.repeat(50)));
  blank();

  // Files with AI attribution
  print(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} FILES WITH AI ATTRIBUTION`));

  const files = Object.keys(log.attestations).sort();
  if (files.length === 0) {
    print(chalk.hex(theme.colours.smoke)('  No files attributed'));
  } else {
    for (const file of files) {
      print(chalk.hex(theme.colours.steel)(`  ${file}`));
      for (const attestation of log.attestations[file]) {
        print(
          chalk.hex(theme.colours.smoke)(
            `    ${attestation.sessionHash.slice(0, 8)}... → lines ${attestation.lineRanges}`
          )
        );
      }
    }
  }
  blank();

  // Sessions
  print(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} AI SESSIONS`));

  const sessions = Object.entries(log.metadata.prompts);
  for (const [hash, prompt] of sessions) {
    print(chalk.hex(theme.colours.molten)(`  Session: ${hash}`));
    print(chalk.hex(theme.colours.smoke)(`    Tool: ${prompt.agent_id.tool}`));
    if (prompt.agent_id.model) {
      print(chalk.hex(theme.colours.smoke)(`    Model: ${prompt.agent_id.model}`));
    }
    print(
      chalk.hex(theme.colours.smoke)(
        `    Lines: ${chalk.green(`+${prompt.total_additions}`)} ${chalk.red(`-${prompt.total_deletions}`)}`
      )
    );
    print(
      chalk.hex(theme.colours.smoke)(
        `    Accepted: ${prompt.accepted_lines}, Human-modified: ${prompt.overridden_lines}`
      )
    );
    if (prompt.human_author) {
      print(chalk.hex(theme.colours.smoke)(`    Author: ${prompt.human_author}`));
    }

    // Show message summary
    if (prompt.messages.length > 0) {
      const userMsgs = prompt.messages.filter((m) => m.type === 'user').length;
      const assistantMsgs = prompt.messages.filter((m) => m.type === 'assistant').length;
      print(
        chalk.hex(theme.colours.smoke)(
          `    Conversation: ${userMsgs} user, ${assistantMsgs} assistant messages`
        )
      );
    }
  }
  blank();

  // Metadata
  print(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} METADATA`));
  print(chalk.hex(theme.colours.smoke)(`  Schema: ${log.metadata.schema_version}`));
  print(chalk.hex(theme.colours.smoke)(`  Commit: ${log.metadata.base_commit_sha.slice(0, 8)}...`));
  blank();
}

/**
 * Create the authorship command
 */
export function createAuthorshipCommand(): Command {
  const command = new Command('authorship');

  command.description(
    'View AI authorship information tracked via Git Notes (Git AI Standard v3.0.0)'
  );

  // Show subcommand
  command
    .command('show')
    .description('Show AI authorship for a specific commit')
    .argument('[commit]', 'Commit SHA or ref (defaults to HEAD)', 'HEAD')
    .option('--json', 'Output as JSON')
    .action(async (commit: string, options: AuthorshipShowOptions) => {
      const workspaceRoot = getWorkspaceRoot();

      const log = await readAuthorshipNote(commit, workspaceRoot);

      if (!log) {
        if (options.json) {
          json({ found: false, commit }, false);
        } else {
          print(
            chalk.hex(theme.colours.molten)(
              `\n${theme.icons.info} No AI authorship information found for ${commit}\n`
            )
          );
        }
        return;
      }

      if (options.json) {
        json(log);
      } else {
        formatAuthorshipLog(log);
      }
    });

  // List subcommand
  command
    .command('list')
    .description('List commits with AI authorship')
    .option('-n, --limit <n>', 'Maximum number of commits to show', '10')
    .option('--json', 'Output as JSON')
    .action(async (options: AuthorshipListOptions) => {
      const workspaceRoot = getWorkspaceRoot();
      const limit = coercePositiveInt(options.limit ?? '10', '--limit');

      const commits = await listAuthorshipNotes(workspaceRoot);

      if (options.json) {
        json({ total: commits.length, commits: commits.slice(0, limit) }, false);
        return;
      }

      if (commits.length === 0) {
        print(
          chalk.hex(theme.colours.molten)(
            `\n${theme.icons.info} No commits with AI authorship found\n`
          )
        );
        print(chalk.hex(theme.colours.smoke)('AI authorship is stored in refs/notes/ai'));
        print(
          chalk.hex(theme.colours.smoke)(
            'Use `git fetch origin refs/notes/ai:refs/notes/ai` to fetch from remote\n'
          )
        );
        return;
      }

      print(chalk.bold(`\nCommits with AI authorship (${commits.length} total):`));
      print(chalk.hex(theme.colours.smoke)('─'.repeat(40)));

      for (const sha of commits.slice(0, limit)) {
        print(chalk.hex(theme.colours.steel)(`  ${sha.slice(0, 8)}`));
      }

      if (commits.length > limit) {
        print(chalk.hex(theme.colours.smoke)(`  ... and ${commits.length - limit} more`));
      }
      blank();
    });

  // Stats subcommand
  command
    .command('stats')
    .description('Show AI authorship statistics for a commit range')
    .argument('[range]', 'Git revision range (e.g., main..HEAD)', 'HEAD~10..HEAD')
    .option('--json', 'Output as JSON')
    .action(async (range: string, options: AuthorshipStatsOptions) => {
      const workspaceRoot = getWorkspaceRoot();

      const stats = await getAuthorshipStats(range, workspaceRoot);

      if (options.json) {
        json(stats);
        return;
      }

      print(chalk.bold(`\nAI Authorship Statistics for ${range}`));
      print(chalk.hex(theme.colours.smoke)('─'.repeat(50)));
      blank();

      print(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} COVERAGE`));
      const percentage =
        stats.totalCommits > 0 ? Math.round((stats.commitsWithAI / stats.totalCommits) * 100) : 0;
      print(
        chalk.hex(theme.colours.smoke)(
          `  ${stats.commitsWithAI}/${stats.totalCommits} commits have AI authorship (${percentage}%)`
        )
      );
      blank();

      print(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} LINE CHANGES`));
      print(
        chalk.hex(theme.colours.smoke)(`  Additions: ${chalk.green(`+${stats.totalAdditions}`)}`)
      );
      print(
        chalk.hex(theme.colours.smoke)(`  Deletions: ${chalk.red(`-${stats.totalDeletions}`)}`)
      );
      blank();

      if (Object.keys(stats.tools).length > 0) {
        print(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} TOOLS USED`));
        for (const [tool, count] of Object.entries(stats.tools).sort((a, b) => b[1] - a[1])) {
          print(chalk.hex(theme.colours.smoke)(`  ${tool}: ${count} session(s)`));
        }
        blank();
      }
    });

  return command;
}
