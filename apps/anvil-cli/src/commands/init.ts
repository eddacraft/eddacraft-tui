import { Command } from 'commander';
import inquirer from 'inquirer';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { EnvironmentDetector } from '../services/environment-detector.js';
import {
  TemplateGenerator,
  type PlanningFormat,
  type ConfigTemplate,
  type InitOptions,
} from '../services/template-generator.js';
import {
  analyseProjectArchitecture,
  formatEntryPoints,
  formatEntryPointsSummary,
  formatLayerDiagram,
  generateArchitectureExplanation,
  formatArchitectureExplanation,
  saveArchitectureBaseline,
  hasExistingBaseline,
  type ArchitectureSummary,
} from '../services/architecture-service.js';
import { success, error, info } from '../utils/output.js';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUI } from '../tui/utils/renderer.js';
import { InitWizard, type WizardState, type WizardContext } from '../tui/commands/init/index.js';
import { ProjectDetector } from '../services/project-detector.js';
import { SampleAnalyzer } from '../services/sample-analyzer.js';
import { HistoricalAnalyzer } from '../services/historical-analyser.js';
import { RepoScanner } from '../services/repo-scanner.js';
import { InitResults } from '../tui/commands/init/InitResults.js';
import type { InitAnalysisResults } from '../tui/components/ResultsDashboard.js';
import {
  PolicyConfigManager,
  selectStarterProfile,
  type PolicyEntry,
} from '../services/policy-config.js';
import { HookInstaller } from '../services/hook-installer.js';

export function createInitCommand(): Command {
  const command = new Command('init');

  command
    .description('Initialise Anvil in the current project')
    .option('--force', 'Overwrite existing .anvilrc if present')
    .option('--non-interactive', 'Skip interactive prompts and use defaults')
    .option('--no-tui', 'Use classic CLI prompts instead of TUI wizard')
    .option('--tui', 'Force TUI wizard mode')
    .option('--no-analysis', 'Skip automatic project analysis')
    .option('--quick', 'Use quick analysis (skip full codebase scan)')
    .option('--org <name>', 'Link to an org policy source (implies --non-interactive)')
    .action(
      async (options: {
        force?: boolean;
        nonInteractive?: boolean;
        // Commander.js --no-tui sets options.tui = false (not options.noTui = true)
        tui?: boolean;
        // Commander.js --no-analysis sets options.analysis = false
        analysis?: boolean;
        quick?: boolean;
        org?: string;
      }) => {
        try {
          const projectRoot = process.cwd();

          // --org implies non-interactive detect-don't-ask flow
          if (options.org) {
            await runDetectAndApplyInit(projectRoot, options.org, options.force);
            return;
          }

          console.log(chalk.bold('\n🔨 Initialising Anvil in current project...\n'));

          // Check if .anvilrc already exists
          const anvilrcPath = join(projectRoot, '.anvilrc');
          if (existsSync(anvilrcPath) && !options.force) {
            error('.anvilrc already exists. Use --force to overwrite.');
            console.log(
              chalk.dim('\nTip: Run `anvil gate:config --list` to view current configuration')
            );
            process.exit(1);
          }

          // Detect environment
          const detector = new EnvironmentDetector(projectRoot);
          const env = detector.detect();

          console.log(chalk.cyan('Detected environment:'));
          console.log(chalk.dim(`  Project: ${env.projectName || '(no package.json)'}`));
          console.log(chalk.dim(`  Package Manager: ${env.packageManager}`));
          console.log(chalk.dim(`  Git: ${env.hasGit ? '✓' : '✗'}`));
          console.log(chalk.dim(`  TypeScript: ${env.hasTypeScript ? '✓' : '✗'}`));
          console.log(chalk.dim(`  ESLint: ${env.hasEslint ? '✓' : '✗'}`));
          console.log(
            chalk.dim(`  Testing: ${env.hasVitest ? 'Vitest' : env.hasJest ? 'Jest' : '✗'}`)
          );
          console.log('');

          // Analyse project architecture
          const archSpinner = ora('Analysing project structure...').start();
          let archSummary: ArchitectureSummary | null = null;
          let shouldCreateBaseline = false;

          try {
            archSummary = await analyseProjectArchitecture(projectRoot);
            archSpinner.succeed(
              chalk.green(
                `Found ${archSummary.moduleCount} modules, ${archSummary.entryPoints.length} entry points`
              )
            );

            // Display architecture summary
            if (archSummary.entryPoints.length > 0) {
              console.log(chalk.cyan('\n' + formatEntryPointsSummary(archSummary.entryPoints)));
              formatEntryPoints(archSummary.entryPoints).forEach((line) =>
                console.log(chalk.dim(line))
              );
              console.log(chalk.dim('\n  Run `anvil status --entry-points` for full details'));
            }

            // Display layer diagram
            console.log(chalk.cyan('\nDetected layer structure:'));
            formatLayerDiagram(archSummary.layers, archSummary.layerAssignments).forEach((line) =>
              console.log(chalk.dim(line))
            );
            console.log('');

            // Display architecture explanation
            const explanation = generateArchitectureExplanation(archSummary);
            console.log(''); // Blank line before architecture analysis
            formatArchitectureExplanation(explanation).forEach((line) => {
              // Use cyan for the header line, white for section headers, dim for content
              if (line.startsWith('Architecture Analysis:')) {
                console.log(chalk.cyan(line));
              } else if (
                line.startsWith('  Recommended Template:') ||
                line.startsWith('  Insights:') ||
                line.startsWith('  Next Steps:')
              ) {
                console.log(chalk.white(line));
              } else {
                console.log(chalk.dim(line));
              }
            });
            console.log('');

            // Ask for confirmation (unless non-interactive)
            if (!options.nonInteractive) {
              const existingBaseline = hasExistingBaseline(projectRoot);
              const confirmAnswer = await inquirer.prompt([
                {
                  type: 'list' as const,
                  name: 'archAction' as const,
                  message: existingBaseline
                    ? 'Architecture baseline exists. What would you like to do?'
                    : 'Does this architecture look correct?',
                  choices: existingBaseline
                    ? [
                        { name: 'Keep existing baseline', value: 'keep' },
                        { name: 'Update with new analysis', value: 'update' },
                        { name: 'Skip architecture setup', value: 'skip' },
                      ]
                    : [
                        { name: 'Yes, save as baseline', value: 'save' },
                        { name: 'Skip architecture setup for now', value: 'skip' },
                      ],
                  default: existingBaseline ? 'keep' : 'save',
                },
              ]);

              shouldCreateBaseline =
                confirmAnswer.archAction === 'save' || confirmAnswer.archAction === 'update';
            } else {
              // Non-interactive: create baseline if none exists
              shouldCreateBaseline = !hasExistingBaseline(projectRoot);
            }
          } catch (archError) {
            archSpinner.warn(
              chalk.yellow('Could not analyse architecture (will skip baseline creation)')
            );
            if (archError instanceof Error) {
              console.log(chalk.dim(`  Reason: ${archError.message}`));
            }
          }

          let initOptions: InitOptions;

          if (options.nonInteractive) {
            initOptions = {
              projectRoot,
              planningDir: 'docs/plans',
              format: 'generic',
              createExample: true,
              configTemplate: 'basic',
              enabledChecks: detector.getRecommendedChecks(env),
              coverageThreshold: 80,
            };
          } else if (isTUIAvailable({ tui: options.tui })) {
            initOptions = await runTUIWizard(projectRoot, env, detector);
          } else {
            initOptions = await runInteractiveSetup(env, detector);
          }

          // Generate configuration and files
          const spinner = ora('Setting up Anvil...').start();

          try {
            const generator = new TemplateGenerator(initOptions);

            // Create .anvil directory
            generator.createAnvilDirectory();
            spinner.text = 'Created .anvil/ directory';

            // Create planning directory
            generator.createPlanningDirectory();
            spinner.text = `Created ${initOptions.planningDir}/ directory`;

            // Generate .anvilrc
            generator.generateAnvilrc();
            spinner.text = 'Generated .anvilrc configuration';

            // Update .gitignore
            if (env.hasGit) {
              generator.updateGitignore();
              spinner.text = 'Updated .gitignore';
            }

            // Generate example plans
            const exampleFiles = generator.generateExamplePlan(env);

            // Create architecture baseline if requested
            if (shouldCreateBaseline && archSummary) {
              spinner.text = 'Creating architecture baseline...';
              saveArchitectureBaseline(projectRoot, archSummary);
            }

            spinner.succeed(chalk.green('Anvil initialised successfully!'));

            // Show summary
            console.log('\n' + chalk.bold('Created files:'));
            console.log(chalk.dim('  ✓ .anvilrc'));
            console.log(chalk.dim('  ✓ .anvil/'));
            if (shouldCreateBaseline && archSummary) {
              console.log(chalk.dim('  ✓ .anvil/architecture.json'));
            }
            console.log(chalk.dim(`  ✓ ${initOptions.planningDir}/`));
            if (env.hasGit) {
              console.log(chalk.dim('  ✓ .gitignore (updated)'));
            }
            if (exampleFiles.length > 0) {
              console.log(chalk.dim('\n' + chalk.bold('Example files:')));
              exampleFiles.forEach((file) => {
                const relPath = file.replace(projectRoot + '/', '');
                console.log(chalk.dim(`  ✓ ${relPath}`));
              });
            }

            // Run intelligent first-run analysis (unless skipped via --no-analysis)
            let dashboardShown = false;
            if (options.analysis !== false) {
              console.log('');
              const analysisSpinner = ora(
                options.quick ? 'Analysing project...' : 'Scanning repository...'
              ).start();

              try {
                const analysisResults = await runIntelligentAnalysis(projectRoot, anvilrcPath, {
                  fullScan: !options.quick,
                });

                analysisSpinner.succeed(
                  chalk.green(
                    `Analysis complete: ${analysisResults.sampleFiles?.analyzed ?? 0} files analyzed`
                  )
                );

                // Show results dashboard if TUI is available
                if (isTUIAvailable({ tui: options.tui })) {
                  console.log('');

                  // Prepare stdin for Ink after inquirer prompts
                  await prepareStdinForInk();

                  await new Promise<void>((resolve, reject) => {
                    const result = renderTUI(InitResults, {
                      results: analysisResults,
                      onComplete: () => resolve(),
                      onQuit: () => resolve(),
                    });

                    if (!result) {
                      reject(new Error('Could not render results dashboard'));
                      return;
                    }

                    result.waitUntilExit().catch(reject);
                  });

                  // Dashboard was successfully shown
                  dashboardShown = true;
                } else {
                  // Fallback: Show text-based summary
                  console.log('\n' + chalk.bold('Project Analysis:'));
                  console.log(chalk.dim(`  Framework: ${analysisResults.project.framework}`));
                  console.log(chalk.dim(`  Project Size: ${analysisResults.project.size}`));

                  if (analysisResults.historical && analysisResults.historical.totalCommits > 0) {
                    console.log('\n' + chalk.bold('Historical Insights:'));
                    console.log(
                      chalk.dim(
                        `  Would have caught ${analysisResults.historical.totalViolations} issues in ${analysisResults.historical.totalCommits} commits`
                      )
                    );
                  }
                }
              } catch (analysisError) {
                analysisSpinner.warn(chalk.yellow('Analysis skipped - see next steps below'));
                if (analysisError instanceof Error) {
                  console.log(chalk.dim(`  Reason: ${analysisError.message}`));
                }
              }
            } else {
              console.log('\n' + chalk.dim('Skipping automatic analysis (--no-analysis flag)'));
            }

            // Show next steps (only if dashboard was not shown)
            if (!dashboardShown) {
              console.log('\n' + chalk.bold('Next steps:'));
              console.log(chalk.cyan('  1. Review configuration:'));
              console.log(chalk.dim('     anvil gate:config --list'));

              if (exampleFiles.length > 0) {
                const firstExample = exampleFiles[0].replace(projectRoot + '/', '');
                console.log(chalk.cyan('  2. Validate example plan:'));
                console.log(chalk.dim(`     anvil validate ${firstExample}`));
                console.log(chalk.cyan('  3. Run quality gates:'));
                console.log(chalk.dim(`     anvil gate ${firstExample}`));
              } else {
                console.log(chalk.cyan('  2. Create a planning document in:'));
                console.log(chalk.dim(`     ${initOptions.planningDir}/`));
                console.log(chalk.cyan('  3. Validate your plan:'));
                console.log(chalk.dim('     anvil validate <plan-file>'));
              }

              console.log('');
            }

            success('Anvil is ready to use!');
          } catch (err) {
            spinner.fail('Initialisation failed');
            throw err;
          }
        } catch (err) {
          error(`Initialisation failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
          process.exit(1);
        }
      }
    );

  return command;
}

/** Answer type for initial setup prompts */
interface SetupAnswers {
  planningDir: string;
  format: PlanningFormat | 'skip';
  createExample?: boolean;
  configTemplate: ConfigTemplate;
}

/** Answer type for gate check configuration */
interface GateCheckAnswers {
  eslint?: boolean;
  test?: boolean;
  coverage?: boolean;
  coverageThreshold?: number;
  secret?: boolean;
}

async function runInteractiveSetup(
  env: ReturnType<EnvironmentDetector['detect']>,
  detector: EnvironmentDetector
): Promise<InitOptions> {
  const answers = (await inquirer.prompt([
    {
      type: 'input' as const,
      name: 'planningDir' as const,
      message: 'Where should planning documents be stored?',
      default: 'docs/plans',
      validate: (input: string) => {
        if (!input || input.trim().length === 0) {
          return 'Directory path is required';
        }
        return true;
      },
    },
    {
      type: 'list' as const,
      name: 'format' as const,
      message: 'Which planning format do you use?',
      choices: [
        { name: 'SpecKit (GitHub spec-kit format)', value: 'speckit' },
        { name: 'BMAD (PRD/Architecture format)', value: 'bmad' },
        { name: 'Generic Markdown', value: 'generic' },
        { name: 'Skip example generation', value: 'skip' },
      ],
      default: 'generic',
    },
    {
      type: 'confirm' as const,
      name: 'createExample' as const,
      message: 'Create example planning document?',
      default: true,
      when: (currentAnswers: Record<string, unknown>) => currentAnswers['format'] !== 'skip',
    },
    {
      type: 'list' as const,
      name: 'configTemplate' as const,
      message: 'Configuration template:',
      choices: [
        { name: 'Basic (80% thresholds, recommended)', value: 'basic' },
        { name: 'Strict (90% thresholds, production-ready)', value: 'strict' },
        { name: 'CI-optimised (minimal checks, fast)', value: 'ci' },
      ],
      default: 'basic',
    },
  ])) as SetupAnswers;

  // Gate checks configuration
  const recommendedChecks = detector.getRecommendedChecks(env);
  const checkAnswers = (await inquirer.prompt([
    {
      type: 'confirm' as const,
      name: 'eslint' as const,
      message: `Enable ESLint gate?${env.hasEslint ? ' (detected)' : ''}`,
      default: env.hasEslint,
      when: recommendedChecks.includes('eslint') || env.hasEslint,
    },
    {
      type: 'confirm' as const,
      name: 'test' as const,
      message: `Enable test gate?${env.hasVitest || env.hasJest ? ' (detected)' : ''}`,
      default: env.hasVitest || env.hasJest,
      when: recommendedChecks.includes('test') || env.hasVitest || env.hasJest,
    },
    {
      type: 'confirm' as const,
      name: 'coverage' as const,
      message: 'Enable coverage gate?',
      default: true,
      when: (currentAnswers: Record<string, unknown>) => currentAnswers['test'] !== false,
    },
    {
      type: 'number' as const,
      name: 'coverageThreshold' as const,
      message: 'Coverage threshold (0-100):',
      default: answers.configTemplate === 'strict' ? 90 : 80,
      validate: (input: number) => (input >= 0 && input <= 100) || 'Must be between 0 and 100',
      when: (currentAnswers: Record<string, unknown>) => currentAnswers['coverage'] === true,
    },
    {
      type: 'confirm' as const,
      name: 'secret' as const,
      message: 'Enable secret scanning?',
      default: true,
    },
  ])) as GateCheckAnswers;

  // Build enabled checks array
  const enabledChecks: string[] = [];
  if (checkAnswers.eslint) enabledChecks.push('eslint');
  if (checkAnswers.test) enabledChecks.push('test');
  if (checkAnswers.coverage) enabledChecks.push('coverage');
  if (checkAnswers.secret) enabledChecks.push('secret');

  return {
    projectRoot: process.cwd(),
    planningDir: answers.planningDir,
    format: answers.format as PlanningFormat,
    createExample: answers.createExample !== false,
    configTemplate: answers.configTemplate,
    enabledChecks,
    coverageThreshold: checkAnswers.coverageThreshold || 80,
  };
}

/**
 * Prepares stdin for Ink TUI after inquirer prompts.
 *
 * Inquirer uses readline which:
 * 1. Pauses stdin when prompts complete
 * 2. May leave buffered keystrokes that Ink could misinterpret
 *
 * This function resets stdin to a clean state for Ink's useInput hook.
 *
 * IMPORTANT: Do NOT call setRawMode(true) here. Ink manages raw mode internally
 * via its useInput hook. Pre-setting raw mode interferes with Ink's internal
 * rawModeEnabledCount and can cause the TUI to exit immediately.
 */
async function prepareStdinForInk(): Promise<void> {
  // Resume stdin if paused by inquirer
  if (process.stdin.isPaused()) {
    process.stdin.resume();
  }

  // Drain any buffered input that might be misinterpreted by Ink.
  // This prevents leftover keystrokes from inquirer causing immediate exits.
  // We need to drain in a loop since there may be multiple buffered chunks.
  while (process.stdin.readable && process.stdin.readableLength > 0) {
    process.stdin.read();
  }

  // Give the event loop a tick to process any pending I/O operations.
  // This ensures stdin is in a stable state before Ink starts.
  await new Promise((resolve) => setImmediate(resolve));
}

async function runTUIWizard(
  projectRoot: string,
  env: ReturnType<EnvironmentDetector['detect']>,
  detector: EnvironmentDetector
): Promise<InitOptions> {
  // Prepare stdin for Ink after inquirer prompts
  await prepareStdinForInk();

  return new Promise((resolve, reject) => {
    const context: WizardContext = {
      projectRoot,
      environment: env,
      recommendedChecks: detector.getRecommendedChecks(env),
    };

    const handleComplete = (state: WizardState) => {
      resolve({
        projectRoot,
        planningDir: state.planningDir,
        format: state.format,
        createExample: state.createExample,
        configTemplate: state.configTemplate,
        enabledChecks: state.enabledChecks,
        coverageThreshold: state.coverageThreshold,
      });
    };

    const handleCancel = () => {
      reject(new Error('Setup cancelled by user'));
    };

    const result = renderTUI(InitWizard, {
      context,
      onComplete: handleComplete,
      onCancel: handleCancel,
    });

    // If TUI couldn't render, reject immediately
    if (!result) {
      reject(new Error('Could not start TUI wizard'));
      return;
    }

    // Wait for TUI to exit, then ensure Promise resolves/rejects
    result.waitUntilExit().catch(reject);
  });
}

/**
 * Run intelligent first-run analysis
 *
 * Orchestrates:
 * 1. Project context detection
 * 2. Full codebase analysis (architecture + anti-pattern checks)
 * 3. Historical git analysis
 *
 * Uses RepoScanner for comprehensive first-run analysis.
 */
async function runIntelligentAnalysis(
  projectRoot: string,
  configPath: string,
  options: { fullScan?: boolean } = {}
): Promise<InitAnalysisResults> {
  const { fullScan = true } = options;

  if (fullScan) {
    // Use RepoScanner for comprehensive analysis
    const scanner = new RepoScanner(projectRoot);
    const scanResult = await scanner.scan({
      historicalDaysBack: 30,
      historicalMaxCommits: 100,
      useCache: true,
      maxFiles: 500, // Limit for init to keep it reasonable
    });

    // Convert scan result to InitAnalysisResults format
    // Calculate per-check pass/fail based on which checks produced blocking warnings
    const blockingWarnings = scanResult.currentIssues.rawResult.warnings.warnings.filter(
      (w) => w.severity === 'error' && !w.suppressed
    );
    const checksWithBlockingWarnings = new Set(blockingWarnings.map((w) => w.category));
    const totalChecks = scanResult.currentIssues.checksRun.length;
    const passedChecks = totalChecks - checksWithBlockingWarnings.size;

    const results: InitAnalysisResults = {
      project: scanResult.project,
      configPath,
      sampleFiles: {
        analyzed: scanResult.currentIssues.filesScanned,
        total: scanResult.project.fileCount,
      },
      analysis: {
        totalChecks,
        passedChecks,
        warnings: scanResult.currentIssues.bySeverity.warnings,
        errors: scanResult.currentIssues.bySeverity.errors,
        suppressions: scanResult.currentIssues.rawResult.warnings.summary.suppressed,
      },
      historical: scanResult.historical,
    };

    return results;
  }

  // Fallback to lightweight analysis (project detection + history only)
  const projectDetector = new ProjectDetector(projectRoot);
  const projectContext = projectDetector.detect();

  const sampleAnalyzer = new SampleAnalyzer(projectRoot);
  const sampleSelection = await sampleAnalyzer.selectFiles({ maxFiles: 50 });

  const historicalAnalyzer = new HistoricalAnalyzer(projectRoot);
  const historicalAnalysis = await historicalAnalyzer.analyse({
    daysBack: 30,
    maxCommits: 100,
  });

  const results: InitAnalysisResults = {
    project: projectContext,
    configPath,
    sampleFiles: {
      analyzed: sampleSelection.files.length,
      total: sampleSelection.totalFound,
    },
    historical: historicalAnalysis,
  };

  return results;
}

// ---------------------------------------------------------------------------
// Detect-don't-ask init flow (--org or future default)
// ---------------------------------------------------------------------------

/**
 * Opinionated, silent init — detects project, applies starter profile,
 * writes .anvil/config.yml with org source, installs hooks. No questions.
 */
async function runDetectAndApplyInit(
  projectRoot: string,
  orgName: string,
  force?: boolean
): Promise<void> {
  const anvilrcPath = join(projectRoot, '.anvilrc');
  const configMgr = new PolicyConfigManager(projectRoot);

  if (existsSync(anvilrcPath) && !force) {
    error('.anvilrc already exists. Use --force to overwrite.');
    process.exit(1);
  }

  // Step 1: Detect project
  const detector = new EnvironmentDetector(projectRoot);
  const env = detector.detect();

  const projectDetector = new ProjectDetector(projectRoot);
  const projectContext = projectDetector.detect();

  const framework = projectContext.framework ?? 'unknown';
  const monorepo = projectContext.monorepo ?? 'none';

  // Step 2: Select starter profile based on detection
  const profile = selectStarterProfile(framework, monorepo);

  // Step 3: Build config.yml
  const orgSource = `git@github.com:${orgName}/anvil-policies.git`;
  const teamPolicies: PolicyEntry[] = profile.policies;

  configMgr.save({
    policies: {
      org: { source: orgSource },
      team: teamPolicies,
      starter_profile: profile.name,
    },
  });

  // Step 4: Generate .anvilrc with recommended checks
  const enabledChecks = detector.getRecommendedChecks(env);
  const initOptions: InitOptions = {
    projectRoot,
    planningDir: 'docs/plans',
    format: 'generic',
    createExample: false,
    configTemplate: 'basic',
    enabledChecks,
    coverageThreshold: 80,
  };

  const generator = new TemplateGenerator(initOptions);
  generator.createAnvilDirectory();
  generator.createPlanningDirectory();
  generator.generateAnvilrc();

  if (env.hasGit) {
    generator.updateGitignore();
  }

  // Step 5: Install hooks
  if (env.hasGit) {
    try {
      const hookInstaller = new HookInstaller();
      hookInstaller.installHook(projectRoot, 'pre-commit', '.git/hooks');
      hookInstaller.installHook(projectRoot, 'pre-push', '.git/hooks');
    } catch {
      // Non-fatal: hooks are nice-to-have
    }
  }

  // Output — concise, opinionated, done
  console.log('');
  success(
    `Detected: ${env.hasTypeScript ? 'TypeScript' : 'JavaScript'}${framework !== 'unknown' ? `, ${framework}` : ''}${env.hasVitest ? ', Vitest' : env.hasJest ? ', Jest' : ''}${env.hasEslint ? ', ESLint' : ''}`
  );
  success(`Applied starter profile: ${profile.name}`);
  success(`${teamPolicies.length} policies active (${teamPolicies.map((p) => p.name).join(', ')})`);
  if (env.hasGit) {
    success('Hooks installed');
  }
  info(`Org source: ${orgSource}`);

  console.log('');
  console.log(chalk.dim("Run `anvil policy list` to see what's active."));
  console.log(chalk.dim("Run `anvil policy tune` when you're ready to customise."));
  console.log('');
}
