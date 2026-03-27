import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { gitExecSync } from '@eddacraft/anvil-core';
import chalk from 'chalk';
import inquirer from 'inquirer';
import { blank, print, debug } from '../utils/output.js';

const CHANGELOG_PATH = 'CHANGELOG.md';

function getLatestTag(workspaceRoot: string): string | null {
  try {
    return gitExecSync(['describe', '--tags', '--abbrev=0'], {
      cwd: workspaceRoot,
    });
  } catch {
    debug('getLatestTag: git describe failed, no tags found');
    return null;
  }
}

function getCommitsSinceTag(workspaceRoot: string, tag: string | null): string[] {
  const range = tag ? `${tag}..HEAD` : 'HEAD';
  try {
    const output = gitExecSync(['log', range, '--oneline', '--no-decorate'], {
      cwd: workspaceRoot,
    });
    return output.split('\n').filter((line: string) => line.length > 0);
  } catch {
    debug('getCommitsSinceTag: git log failed, returning empty list');
    return [];
  }
}

function buildChangelogEntry(version: string, commits: string[]): string {
  const date = new Date().toISOString().split('T')[0];
  const lines = [`## [${version}] - ${date}`, ''];

  if (commits.length === 0) {
    lines.push('- No changes recorded');
  } else {
    for (const commit of commits) {
      // Strip the short hash prefix
      const message = commit.replace(/^[a-f0-9]+ /, '');
      lines.push(`- ${message}`);
    }
  }

  lines.push('');
  return lines.join('\n');
}

export async function updateChangelog(
  workspaceRoot: string,
  version: string,
  execute: boolean
): Promise<string[]> {
  const changelogPath = join(workspaceRoot, CHANGELOG_PATH);
  const latestTag = getLatestTag(workspaceRoot);
  const commits = getCommitsSinceTag(workspaceRoot, latestTag);
  const entry = buildChangelogEntry(version, commits);

  print(chalk.dim('  Commits since last tag:'));
  if (commits.length === 0) {
    print(chalk.dim('    (none)'));
  } else {
    for (const commit of commits.slice(0, 15)) {
      print(chalk.dim(`    ${commit}`));
    }
    if (commits.length > 15) {
      print(chalk.dim(`    ... and ${commits.length - 15} more`));
    }
  }

  blank();
  print(chalk.bold('  Changelog entry:'));
  print(
    chalk.cyan(
      entry
        .split('\n')
        .map((l) => `    ${l}`)
        .join('\n')
    )
  );

  if (!execute) {
    print(`  ${chalk.yellow('[DRY RUN]')} Would prepend to ${CHANGELOG_PATH}`);
    return [];
  }

  const { proceed } = await inquirer.prompt<{ proceed: boolean }>([
    {
      type: 'confirm',
      name: 'proceed',
      message: 'Add this entry to CHANGELOG.md?',
      default: true,
    },
  ]);

  if (!proceed) {
    print(chalk.dim('  Skipping changelog update'));
    return [];
  }

  if (existsSync(changelogPath)) {
    const existing = readFileSync(changelogPath, 'utf8');
    // Insert after the first heading line (# Changelog or similar)
    const firstHeadingEnd = existing.indexOf('\n');
    if (firstHeadingEnd !== -1 && existing.startsWith('#')) {
      const header = existing.slice(0, firstHeadingEnd + 1);
      const rest = existing.slice(firstHeadingEnd + 1);
      writeFileSync(changelogPath, header + '\n' + entry + rest, 'utf8');
    } else {
      writeFileSync(changelogPath, entry + existing, 'utf8');
    }
  } else {
    writeFileSync(changelogPath, `# Changelog\n\n${entry}`, 'utf8');
  }

  print(`  ${chalk.green('✓')} Updated ${CHANGELOG_PATH}`);
  return [CHANGELOG_PATH];
}
