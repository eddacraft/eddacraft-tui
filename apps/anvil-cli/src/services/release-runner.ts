import { existsSync } from 'node:fs';
import { join } from 'node:path';
import chalk from 'chalk';
import type {
  ReleaseConfig,
  ReleaseProfile,
  ReleaseState,
  ReleaseStep,
  ReleaseStepId,
} from './release-types.js';
import { PROFILES, createInitialSteps, formatTag } from './release-types.js';
import { loadReleaseState, saveReleaseState, clearReleaseState } from './release-state.js';
import { runPreflight } from './release-preflight.js';
import { bumpVersion, readCurrentVersion } from './release-version.js';
import { updateChangelog } from './release-changelog.js';
import { commitTagPush, isCleanWorkingTree, getCurrentBranch } from './release-git.js';
import { monitorWorkflow } from './release-monitor.js';
import { verifyRelease } from './release-verify.js';

function printHeader(state: ReleaseState, profile: ReleaseProfile): void {
  console.log();
  console.log(chalk.bold('  ANVIL RELEASE'));
  console.log();
  console.log(`  Profile:         ${chalk.cyan(profile.name)}`);
  console.log(`  Current version: ${chalk.bold(state.previousVersion)}`);
  if (state.version) {
    console.log(`  Target version:  ${chalk.bold.green(state.version)}`);
  }
  console.log();
}

function printStepList(steps: ReleaseStep[]): void {
  console.log('  ┌─────────────────────────────────────────┐');
  console.log('  │  Steps                                  │');
  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];
    const icon =
      step.status === 'passed'
        ? chalk.green('✓')
        : step.status === 'failed'
          ? chalk.red('✗')
          : step.status === 'skipped'
            ? chalk.dim('–')
            : ' ';
    const label = `${icon} ${i + 1}. ${step.label}`;
    console.log(`  │  ${label.padEnd(40)}│`);
  }
  console.log('  └─────────────────────────────────────────┘');
  console.log();
}

function printStepHeader(index: number, total: number, label: string): void {
  console.log(chalk.bold(`  Step ${index + 1}/${total}: ${label}`));
}

function updateStepStatus(
  state: ReleaseState,
  stepId: ReleaseStepId,
  status: 'running' | 'passed' | 'failed' | 'skipped',
  error?: string
): void {
  const step = state.steps.find((s) => s.id === stepId);
  if (!step) return;
  step.status = status;
  if (status === 'running') step.startedAt = new Date().toISOString();
  if (status === 'passed' || status === 'failed') step.completedAt = new Date().toISOString();
  if (error) step.error = error;
}

function isStepDone(state: ReleaseState, stepId: ReleaseStepId): boolean {
  const step = state.steps.find((s) => s.id === stepId);
  return step?.status === 'passed' || step?.status === 'skipped';
}

export async function runRelease(config: ReleaseConfig): Promise<void> {
  const workspaceRoot = process.cwd();

  // ── Profile ──────────────────────────────────────────────────────────
  const profile = PROFILES[config.profile];
  if (!profile) {
    if (config.profile === 'stable' || config.profile === 'hotfix') {
      console.log(chalk.yellow(`\n  Profile '${config.profile}' is not yet implemented.`));
      console.log(chalk.dim('  Only the beta profile is available in this release.\n'));
      process.exit(1);
    }
    console.error(chalk.red(`  Unknown profile: ${config.profile}`));
    process.exit(1);
  }

  // ── Monorepo guard ──────────────────────────────────────────────────
  if (!existsSync(join(workspaceRoot, 'nx.json'))) {
    console.error(chalk.red('\n  ✗ Not in the Anvil monorepo root.'));
    console.error(
      chalk.dim('    Run this command from the workspace root (where nx.json lives).\n')
    );
    process.exit(1);
  }

  const branch = getCurrentBranch(workspaceRoot);
  if (branch !== 'main' && config.execute) {
    console.log(chalk.yellow(`\n  ⚠ On branch '${branch}', not main.`));
    console.log(chalk.dim('    Use --execute only from main. Dry-run is allowed on any branch.\n'));
    process.exit(1);
  }

  if (config.execute && !isCleanWorkingTree(workspaceRoot)) {
    console.error(chalk.red('\n  ✗ Working tree is not clean. Commit or stash changes first.\n'));
    process.exit(1);
  }

  // ── State ───────────────────────────────────────────────────────────
  let state: ReleaseState;

  if (config.resume) {
    const existing = loadReleaseState(workspaceRoot);
    if (existing) {
      state = existing;
      console.log(chalk.dim(`\n  Resuming release (started ${existing.startedAt})`));
    } else {
      console.log(chalk.dim('\n  No saved state found, starting fresh.'));
      state = createState(workspaceRoot, profile);
    }
  } else {
    clearReleaseState(workspaceRoot);
    state = createState(workspaceRoot, profile);
  }

  printHeader(state, profile);
  printStepList(state.steps);

  if (!config.execute) {
    console.log(chalk.yellow('  Running in dry-run mode. Use --execute to perform changes.\n'));
  }

  // ── Run steps ────────────────────────────────────────────────────────
  const stepIndex = (id: ReleaseStepId) => state.steps.findIndex((s) => s.id === id);
  const totalSteps = state.steps.length;

  // 1. Preflight
  if (profile.steps.includes('preflight') && !isStepDone(state, 'preflight')) {
    if (config.skipPreflight) {
      updateStepStatus(state, 'preflight', 'skipped');
      console.log(chalk.dim('  Preflight skipped (--skip-preflight)\n'));
    } else {
      printStepHeader(stepIndex('preflight'), totalSteps, 'Preflight checks');
      updateStepStatus(state, 'preflight', 'running');
      saveReleaseState(workspaceRoot, state);

      const result = await runPreflight(workspaceRoot, config.verbose);

      if (result.allPassed) {
        updateStepStatus(state, 'preflight', 'passed');
        const total = (result.totalDurationMs / 1000).toFixed(1);
        console.log(chalk.dim(`  All checks passed in ${total}s\n`));
      } else {
        updateStepStatus(state, 'preflight', 'failed');
        saveReleaseState(workspaceRoot, state);
        console.error(chalk.red('\n  ✗ Preflight failed. Fix issues and run with --resume.\n'));
        process.exit(1);
      }
      saveReleaseState(workspaceRoot, state);
    }
  }

  // 2. Version
  if (profile.steps.includes('version') && !isStepDone(state, 'version')) {
    printStepHeader(stepIndex('version'), totalSteps, 'Version bump');
    updateStepStatus(state, 'version', 'running');

    const result = await bumpVersion(workspaceRoot, profile, config.targetVersion, config.execute);
    state.previousVersion = result.previousVersion;
    state.version = result.newVersion;
    state.tagName = formatTag(profile, result.newVersion);
    state.modifiedFiles = [...result.modifiedFiles];

    updateStepStatus(state, 'version', 'passed');
    saveReleaseState(workspaceRoot, state);
    console.log();
  }

  // 3. Changelog
  if (profile.steps.includes('changelog') && !isStepDone(state, 'changelog')) {
    printStepHeader(stepIndex('changelog'), totalSteps, 'Changelog');
    updateStepStatus(state, 'changelog', 'running');

    const changelogFiles = await updateChangelog(workspaceRoot, state.version, config.execute);
    state.modifiedFiles.push(...changelogFiles);

    updateStepStatus(state, 'changelog', 'passed');
    saveReleaseState(workspaceRoot, state);
    console.log();
  }

  // 4. Commit + tag + push
  if (profile.steps.includes('commit-tag-push') && !isStepDone(state, 'commit-tag-push')) {
    printStepHeader(stepIndex('commit-tag-push'), totalSteps, 'Commit + tag + push');
    updateStepStatus(state, 'commit-tag-push', 'running');

    try {
      const result = await commitTagPush(
        workspaceRoot,
        state.version,
        state.tagName,
        state.modifiedFiles,
        config.execute
      );
      state.tagName = result.tagName;
      updateStepStatus(state, 'commit-tag-push', 'passed');
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      updateStepStatus(state, 'commit-tag-push', 'failed', msg);
      saveReleaseState(workspaceRoot, state);
      console.error(chalk.red(`\n  ✗ Git operation failed: ${msg}`));
      console.log(chalk.dim('  Check git status and run with --resume.\n'));
      process.exit(1);
    }
    saveReleaseState(workspaceRoot, state);
    console.log();
  }

  // 5. Monitor
  if (profile.steps.includes('monitor') && !isStepDone(state, 'monitor')) {
    printStepHeader(stepIndex('monitor'), totalSteps, 'Monitor workflow');
    updateStepStatus(state, 'monitor', 'running');

    const runId = await monitorWorkflow(workspaceRoot, state.tagName, config.execute);
    if (runId) state.workflowRunId = runId;

    updateStepStatus(state, 'monitor', 'passed');
    saveReleaseState(workspaceRoot, state);
    console.log();
  }

  // 6. Verify
  if (profile.steps.includes('verify') && !isStepDone(state, 'verify')) {
    printStepHeader(stepIndex('verify'), totalSteps, 'Post-release verification');
    updateStepStatus(state, 'verify', 'running');

    await verifyRelease(state.version, config.execute);

    updateStepStatus(state, 'verify', 'passed');
    saveReleaseState(workspaceRoot, state);
    console.log();
  }

  // ── Done ─────────────────────────────────────────────────────────────
  clearReleaseState(workspaceRoot);

  if (config.execute) {
    console.log(chalk.bold.green(`  ✓ Release ${state.tagName} complete!\n`));
  } else {
    console.log(
      chalk.bold.yellow(`  ✓ Dry run complete. Run with --execute to perform the release.\n`)
    );
  }
}

function createState(workspaceRoot: string, profile: ReleaseProfile): ReleaseState {
  return {
    version: '',
    previousVersion: readCurrentVersion(workspaceRoot),
    profile: profile.name,
    steps: createInitialSteps(profile),
    startedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    tagName: '',
    modifiedFiles: [],
  };
}
