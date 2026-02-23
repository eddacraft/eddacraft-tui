import { execFileSync } from 'node:child_process';
import ora from 'ora';
import chalk from 'chalk';
import { INTERNAL_PACKAGES } from './release-types.js';

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function checkNpmVersion(pkg: string, version: string): boolean {
  try {
    const output = execFileSync('npm', ['view', `${pkg}@${version}`, 'version'], {
      encoding: 'utf8',
      stdio: 'pipe',
    });
    return output.trim() === version;
  } catch {
    return false;
  }
}

function checkNpmExists(pkg: string): string | null {
  try {
    return execFileSync('npm', ['view', '--prefer-online', pkg, 'version'], {
      encoding: 'utf8',
      stdio: 'pipe',
    }).trim();
  } catch {
    return null;
  }
}

async function pollForVersion(pkg: string, version: string, timeoutMs: number): Promise<boolean> {
  const interval = 5000;
  const maxAttempts = Math.ceil(timeoutMs / interval);

  for (let i = 0; i < maxAttempts; i++) {
    if (checkNpmVersion(pkg, version)) return true;
    await sleep(interval);
  }

  return false;
}

export interface VerifyResult {
  passed: boolean;
  npmPublished: boolean;
  smokeCheckPassed: boolean;
  internalPackageLeaks: number;
}

export async function verifyRelease(version: string, execute: boolean): Promise<VerifyResult> {
  if (!execute) {
    console.log(`  ${chalk.yellow('[DRY RUN]')} Would verify:`);
    console.log(chalk.dim(`    npm view @eddacraft/anvil-cli@${version} version`));
    console.log(chalk.dim(`    npx -y --package @eddacraft/anvil-cli@${version} anvil --help`));
    console.log(chalk.dim('    Check internal packages not published'));
    return { passed: true, npmPublished: true, smokeCheckPassed: true, internalPackageLeaks: 0 };
  }

  let npmPublished = false;
  let smokeCheckPassed = false;
  let internalPackageLeaks = 0;

  // Poll for the published version
  const spinner = ora({
    text: `Waiting for @eddacraft/anvil-cli@${version} on npm...`,
    prefixText: '  ',
  }).start();

  const found = await pollForVersion('@eddacraft/anvil-cli', version, 120_000);

  if (!found) {
    spinner.warn(`@eddacraft/anvil-cli@${version} not found after 2 minutes`);
    console.log(chalk.dim('  Check manually:'));
    console.log(chalk.dim(`    npm view @eddacraft/anvil-cli@${version} version`));
  } else {
    spinner.succeed(`@eddacraft/anvil-cli@${version} published`);
    npmPublished = true;
  }

  // Smoke check from npm (only if published)
  if (npmPublished) {
    const smokeSpinner = ora({ text: 'Running smoke check from npm...', prefixText: '  ' }).start();
    try {
      execFileSync(
        'npx',
        ['-y', '--package', `@eddacraft/anvil-cli@${version}`, 'anvil', '--help'],
        { encoding: 'utf8', stdio: 'pipe' }
      );
      smokeSpinner.succeed('anvil --help works from npm');
      smokeCheckPassed = true;
    } catch {
      smokeSpinner.fail('anvil --help failed from npm');
    }
  }

  // Check internal packages were NOT published
  console.log(chalk.dim('  Checking internal packages not published...'));
  for (const pkg of INTERNAL_PACKAGES) {
    const ver = checkNpmExists(pkg);
    if (ver) {
      console.log(`  ${chalk.red('✗')} ${pkg} found on npm (${ver}) — unexpected!`);
      internalPackageLeaks++;
    }
  }

  if (internalPackageLeaks === 0) {
    console.log(`  ${chalk.green('✓')} No internal packages leaked to npm`);
  }

  const passed = npmPublished && smokeCheckPassed && internalPackageLeaks === 0;
  return { passed, npmPublished, smokeCheckPassed, internalPackageLeaks };
}
