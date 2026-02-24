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
  console.log(chalk.bold('\nAI Authorship Log'));
  console.log(chalk.hex(theme.colours.smoke)('─'.repeat(50)));
  console.log();

  // Files with AI attribution
  console.log(
    chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} FILES WITH AI ATTRIBUTION`)
  );

  const files = Object.keys(log.attestations).sort();
  if (files.length === 0) {
    console.log(chalk.hex(theme.colours.smoke)('  No files attributed'));
  } else {
    for (const file of files) {
      console.log(chalk.hex(theme.colours.steel)(`  ${file}`));
      for (const attestation of log.attestations[file]) {
        console.log(
          chalk.hex(theme.colours.smoke)(
            `    ${attestation.sessionHash.slice(0, 8)}... → lines ${attestation.lineRanges}`
          )
        );
      }
    }
  }
  console.log();

  // Sessions
  console.log(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} AI SESSIONS`));

  const sessions = Object.entries(log.metadata.prompts);
  for (const [hash, prompt] of sessions) {
    console.log(chalk.hex(theme.colours.molten)(`  Session: ${hash}`));
    console.log(chalk.hex(theme.colours.smoke)(`    Tool: ${prompt.agent_id.tool}`));
    if (prompt.agent_id.model) {
      console.log(chalk.hex(theme.colours.smoke)(`    Model: ${prompt.agent_id.model}`));
    }
    console.log(
      chalk.hex(theme.colours.smoke)(
        `    Lines: ${chalk.green(`+${prompt.total_additions}`)} ${chalk.red(`-${prompt.total_deletions}`)}`
      )
    );
    console.log(
      chalk.hex(theme.colours.smoke)(
        `    Accepted: ${prompt.accepted_lines}, Human-modified: ${prompt.overridden_lines}`
      )
    );
    if (prompt.human_author) {
      console.log(chalk.hex(theme.colours.smoke)(`    Author: ${prompt.human_author}`));
    }

    // Show message summary
    if (prompt.messages.length > 0) {
      const userMsgs = prompt.messages.filter((m) => m.type === 'user').length;
      const assistantMsgs = prompt.messages.filter((m) => m.type === 'assistant').length;
      console.log(
        chalk.hex(theme.colours.smoke)(
          `    Conversation: ${userMsgs} user, ${assistantMsgs} assistant messages`
        )
      );
    }
  }
  console.log();

  // Metadata
  console.log(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} METADATA`));
  console.log(chalk.hex(theme.colours.smoke)(`  Schema: ${log.metadata.schema_version}`));
  console.log(
    chalk.hex(theme.colours.smoke)(`  Commit: ${log.metadata.base_commit_sha.slice(0, 8)}...`)
  );
  console.log();
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
          console.log(JSON.stringify({ found: false, commit }));
        } else {
          console.log(
            chalk.hex(theme.colours.molten)(
              `\n${theme.icons.info} No AI authorship information found for ${commit}\n`
            )
          );
        }
        return;
      }

      if (options.json) {
        console.log(JSON.stringify(log, null, 2));
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
      const limit = parseInt(options.limit ?? '10', 10);

      const commits = await listAuthorshipNotes(workspaceRoot);

      if (options.json) {
        console.log(JSON.stringify({ total: commits.length, commits: commits.slice(0, limit) }));
        return;
      }

      if (commits.length === 0) {
        console.log(
          chalk.hex(theme.colours.molten)(
            `\n${theme.icons.info} No commits with AI authorship found\n`
          )
        );
        console.log(chalk.hex(theme.colours.smoke)('AI authorship is stored in refs/notes/ai'));
        console.log(
          chalk.hex(theme.colours.smoke)(
            'Use `git fetch origin refs/notes/ai:refs/notes/ai` to fetch from remote\n'
          )
        );
        return;
      }

      console.log(chalk.bold(`\nCommits with AI authorship (${commits.length} total):`));
      console.log(chalk.hex(theme.colours.smoke)('─'.repeat(40)));

      for (const sha of commits.slice(0, limit)) {
        console.log(chalk.hex(theme.colours.steel)(`  ${sha.slice(0, 8)}`));
      }

      if (commits.length > limit) {
        console.log(chalk.hex(theme.colours.smoke)(`  ... and ${commits.length - limit} more`));
      }
      console.log();
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
        console.log(JSON.stringify(stats, null, 2));
        return;
      }

      console.log(chalk.bold(`\nAI Authorship Statistics for ${range}`));
      console.log(chalk.hex(theme.colours.smoke)('─'.repeat(50)));
      console.log();

      console.log(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} COVERAGE`));
      const percentage =
        stats.totalCommits > 0 ? Math.round((stats.commitsWithAI / stats.totalCommits) * 100) : 0;
      console.log(
        chalk.hex(theme.colours.smoke)(
          `  ${stats.commitsWithAI}/${stats.totalCommits} commits have AI authorship (${percentage}%)`
        )
      );
      console.log();

      console.log(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} LINE CHANGES`));
      console.log(
        chalk.hex(theme.colours.smoke)(`  Additions: ${chalk.green(`+${stats.totalAdditions}`)}`)
      );
      console.log(
        chalk.hex(theme.colours.smoke)(`  Deletions: ${chalk.red(`-${stats.totalDeletions}`)}`)
      );
      console.log();

      if (Object.keys(stats.tools).length > 0) {
        console.log(chalk.hex(theme.colours.ember).bold(`${theme.icons.bullet} TOOLS USED`));
        for (const [tool, count] of Object.entries(stats.tools).sort((a, b) => b[1] - a[1])) {
          console.log(chalk.hex(theme.colours.smoke)(`  ${tool}: ${count} session(s)`));
        }
        console.log();
      }
    });

  return command;
}
