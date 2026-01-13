import { Command } from 'commander';
import inquirer from 'inquirer';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync } from 'fs';
import { join } from 'path';
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
  formatLayerDiagram,
  generateArchitectureExplanation,
  formatArchitectureExplanation,
  saveArchitectureBaseline,
  hasExistingBaseline,
  type ArchitectureSummary,
} from '../services/architecture-service.js';
import { success, error } from '../utils/output.js';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUI } from '../tui/utils/renderer.js';
import { InitWizard, type WizardState, type WizardContext } from '../tui/commands/init/index.js';
import { ProjectDetector } from '../services/project-detector.js';
import { SampleAnalyzer } from '../services/sample-analyzer.js';
import { HistoricalAnalyzer } from '../services/historical-analyser.js';
import { InitResults } from '../tui/commands/init/InitResults.js';
import type { InitAnalysisResults } from '../tui/components/ResultsDashboard.js';

export function createInitCommand(): Command {
  const command = new Command('init');

  command
    .description('Initialise Anvil in the current project')
    .option('--force', 'Overwrite existing .anvilrc if present')
    .option('--non-interactive', 'Skip interactive prompts and use defaults')
    .option('--no-tui', 'Use classic CLI prompts instead of TUI wizard')
    .option('--tui', 'Force TUI wizard mode')
    .option('--no-analysis', 'Skip automatic project analysis')
    .action(
      async (options: {
        force?: boolean;
        nonInteractive?: boolean;
        tui?: boolean;
        noTui?: boolean;
        noAnalysis?: boolean;
      }) => {
        try {
          console.log(chalk.bold('\n🔨 Initialising Anvil in current project...\n'));

          const projectRoot = process.cwd();

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
              console.log(chalk.cyan('\nDetected entry points:'));
              formatEntryPoints(archSummary.entryPoints).forEach((line) =>
                console.log(chalk.dim(line))
              );
            }

            // Display layer diagram
            console.log(chalk.cyan('\nDetected layer structure:'));
            formatLayerDiagram(archSummary.layers, archSummary.layerAssignments).forEach((line) =>
              console.log(chalk.dim(line))
            );
            console.log('');

            // Display architecture explanation
            const explanation = generateArchitectureExplanation(archSummary);
            console.log(chalk.cyan('Architecture summary:'));
            formatArchitectureExplanation(explanation).forEach((line) =>
              console.log(chalk.dim(line))
            );
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
          } else if (isTUIAvailable({ tui: options.tui, noTui: options.noTui })) {
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

            // Run intelligent first-run analysis (unless skipped)
            let dashboardShown = false;
            if (!options.noAnalysis) {
              console.log('');
              const analysisSpinner = ora('Analysing project...').start();

              try {
                const analysisResults = await runIntelligentAnalysis(projectRoot, anvilrcPath);

                analysisSpinner.succeed(
                  chalk.green(
                    `Analysis complete: ${analysisResults.sampleFiles?.analyzed ?? 0} files analyzed`
                  )
                );

                // Show results dashboard if TUI is available
                if (isTUIAvailable({ tui: options.tui, noTui: options.noTui })) {
                  console.log('');

                  // Prepare stdin for Ink after inquirer prompts
                  prepareStdinForInk();

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

<<<<<<< HEAD:apps/anvil-cli/src/commands/init.ts
            // Show next steps (only if dashboard was not shown)
            if (!dashboardShown) {
=======
            // Show next steps (only if analysis was skipped or TUI not shown)
            if (options.noAnalysis || !isTUIAvailable({ tui: options.tui, noTui: options.noTui })) {
>>>>>>> 177f91e (style: Apply Prettier formatting to IFR files):cli/src/commands/init.ts
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
 * 1. Leaves stdin in "line" mode (not "raw" mode)
 * 2. May leave buffered keystrokes that Ink could misinterpret
 *
 * This function resets stdin to a clean state for Ink's useInput hook.
 */
function prepareStdinForInk(): void {
  // Resume stdin if paused by inquirer
  if (process.stdin.isPaused()) {
    process.stdin.resume();
  }

  // Drain any buffered input that might be misinterpreted by Ink
  // This prevents leftover keystrokes from inquirer causing immediate exits
  if (process.stdin.readable) {
    process.stdin.read();
  }

  // Set stdin to raw mode for Ink's useInput hook to work correctly.
  // Without this, keystrokes are buffered until Enter is pressed, causing
  // the TUI wizard to appear unresponsive and exit immediately.
  if (process.stdin.isTTY && process.stdin.setRawMode) {
    process.stdin.setRawMode(true);
  }
}

async function runTUIWizard(
  projectRoot: string,
  env: ReturnType<EnvironmentDetector['detect']>,
  detector: EnvironmentDetector
): Promise<InitOptions> {
  // Prepare stdin for Ink after inquirer prompts
  prepareStdinForInk();

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
 * 2. Sample file selection
 * 3. Quick wins identification
 * 4. Historical git analysis
 */
async function runIntelligentAnalysis(
  projectRoot: string,
  configPath: string
): Promise<InitAnalysisResults> {
  // Step 1: Detect project context
  const projectDetector = new ProjectDetector(projectRoot);
  const projectContext = projectDetector.detect();

  // Step 2: Select sample files for analysis
  const sampleAnalyzer = new SampleAnalyzer(projectRoot);
  const sampleSelection = await sampleAnalyzer.selectFiles({ maxFiles: 50 });

  // Step 3: Analyse git history (async, don't wait)
  const historicalAnalyzer = new HistoricalAnalyzer(projectRoot);
  const historicalAnalysis = await historicalAnalyzer.analyse({
    daysBack: 30,
    maxCommits: 100,
  });

  // Build results structure
  // Note: Full gate check integration is not included in this initial version
  // This provides the foundation for future integration
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
