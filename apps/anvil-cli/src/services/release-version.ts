import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import semver from 'semver';
import chalk from 'chalk';
import inquirer from 'inquirer';
import type { ReleaseProfile, VersionFile } from './release-types.js';

export interface VersionBumpResult {
  previousVersion: string;
  newVersion: string;
  modifiedFiles: string[];
}

export function readCurrentVersion(workspaceRoot: string): string {
  const pkgPath = join(workspaceRoot, 'apps/anvil-cli/package.json');
  const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
  return pkg.version as string;
}

interface VersionChoice {
  name: string;
  value: string;
}

function buildVersionChoices(current: string, prerelease?: string): VersionChoice[] {
  const choices: VersionChoice[] = [];
  const releases: Array<{ type: semver.ReleaseType; label: string }> = [
    { type: 'patch', label: 'patch' },
    { type: 'minor', label: 'minor' },
    { type: 'major', label: 'major' },
  ];

  for (const { type, label } of releases) {
    const bumped = semver.inc(current, type);
    if (bumped) {
      choices.push({ name: `${label.padEnd(10)} ${bumped}`, value: bumped });
    }
  }

  if (prerelease) {
    const preReleases: Array<{ type: semver.ReleaseType; label: string }> = [
      { type: 'prepatch', label: 'prepatch' },
      { type: 'preminor', label: 'preminor' },
      { type: 'prerelease', label: 'prerelease' },
    ];

    for (const { type, label } of preReleases) {
      const bumped = semver.inc(current, type, prerelease);
      if (bumped) {
        choices.push({ name: `${label.padEnd(10)} ${bumped}`, value: bumped });
      }
    }
  }

  choices.push({ name: 'custom     Enter manually', value: 'custom' });
  return choices;
}

export async function promptForVersion(
  currentVersion: string,
  profile: ReleaseProfile
): Promise<string> {
  console.log(`  Current version: ${chalk.bold(currentVersion)}\n`);

  const choices = buildVersionChoices(currentVersion, profile.prerelease);

  const { version } = await inquirer.prompt<{ version: string }>([
    {
      type: 'list',
      name: 'version',
      message: 'Select new version:',
      choices,
    },
  ]);

  if (version === 'custom') {
    const { custom } = await inquirer.prompt<{ custom: string }>([
      {
        type: 'input',
        name: 'custom',
        message: 'Enter version:',
        validate: (input: string) => {
          if (!semver.valid(input)) return 'Must be valid semver';
          if (!semver.gt(input, currentVersion)) return `Must be greater than ${currentVersion}`;
          return true;
        },
      },
    ]);
    return custom;
  }

  return version;
}

function applyVersionToFile(
  workspaceRoot: string,
  versionFile: VersionFile,
  newVersion: string,
  dryRun: boolean
): { path: string; before: string; after: string } | null {
  const absPath = join(workspaceRoot, versionFile.path);
  const content = readFileSync(absPath, 'utf8');
  const replacement = versionFile.replacement.replace(/\$\{version\}/g, newVersion);
  const updated = content.replace(versionFile.pattern, replacement);

  if (content === updated) return null;

  if (!dryRun) {
    writeFileSync(absPath, updated, 'utf8');
  }

  // Extract the matched portion for display
  const match = content.match(versionFile.pattern);
  const matchAfter = updated.match(versionFile.pattern);

  return {
    path: versionFile.path,
    before: match ? match[0] : '',
    after: matchAfter ? matchAfter[0] : replacement,
  };
}

export async function bumpVersion(
  workspaceRoot: string,
  profile: ReleaseProfile,
  targetVersion: string | undefined,
  execute: boolean
): Promise<VersionBumpResult> {
  const currentVersion = readCurrentVersion(workspaceRoot);

  let newVersion: string;
  if (targetVersion) {
    if (!semver.valid(targetVersion)) {
      throw new Error(`Invalid version: ${targetVersion}`);
    }
    if (!semver.gt(targetVersion, currentVersion)) {
      throw new Error(`Version ${targetVersion} is not greater than ${currentVersion}`);
    }
    newVersion = targetVersion;
  } else {
    newVersion = await promptForVersion(currentVersion, profile);
  }

  const dryRun = !execute;
  const modifiedFiles: string[] = [];

  console.log();
  for (const versionFile of profile.versionFiles) {
    const result = applyVersionToFile(workspaceRoot, versionFile, newVersion, dryRun);
    if (result) {
      modifiedFiles.push(result.path);
      const prefix = dryRun ? chalk.yellow('[DRY RUN]') : chalk.green('  ✓');
      console.log(`  ${prefix} ${result.path}`);
      console.log(`    ${chalk.red(`- ${result.before}`)}`);
      console.log(`    ${chalk.green(`+ ${result.after}`)}`);
    }
  }

  return { previousVersion: currentVersion, newVersion, modifiedFiles };
}
