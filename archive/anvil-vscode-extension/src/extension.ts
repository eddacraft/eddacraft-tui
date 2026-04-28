import * as vscode from 'vscode';
import { registerCommands } from './commands/index.js';
import { StatusBarManager } from './services/statusBar.js';
import { DiagnosticsManager } from './services/diagnostics.js';
import { GateResultsProvider } from './providers/gateResultsProvider.js';
import { PlanCodeLensProvider } from './providers/codeLensProvider.js';
import { NudgeCodeActionProvider } from './providers/nudgeCodeActionProvider.js';
import { PlanWatcher } from './services/planWatcher.js';
import { SourceWatcher } from './services/sourceWatcher.js';
import { AnvilService } from './services/anvilService.js';

let statusBarManager: StatusBarManager;
let diagnosticsManager: DiagnosticsManager;
let gateResultsProvider: GateResultsProvider;
let planWatcher: PlanWatcher;
let sourceWatcher: SourceWatcher;
let anvilService: AnvilService;
let outputChannel: vscode.OutputChannel;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  outputChannel = vscode.window.createOutputChannel('Anvil');
  context.subscriptions.push(outputChannel);
  outputChannel.appendLine('Anvil extension is activating...');

  // Initialise core services
  anvilService = new AnvilService(context, outputChannel);
  diagnosticsManager = new DiagnosticsManager();
  statusBarManager = new StatusBarManager();
  gateResultsProvider = new GateResultsProvider(anvilService);
  planWatcher = new PlanWatcher(anvilService, diagnosticsManager);

  // Register tree view
  const treeView = vscode.window.createTreeView('anvilGateResults', {
    treeDataProvider: gateResultsProvider,
    showCollapseAll: true,
  });
  context.subscriptions.push(treeView, gateResultsProvider);

  // Register CodeLens provider for plan files
  const config = vscode.workspace.getConfiguration('anvil');
  if (config.get<boolean>('showCodeLens', true)) {
    const codeLensProvider = new PlanCodeLensProvider(anvilService);
    const codeLensDisposable = vscode.languages.registerCodeLensProvider(
      [
        { pattern: '**/*.plan.md' },
        { pattern: '**/plan.md' },
        { pattern: '**/*.spec.md' },
        { pattern: '**/spec.md' },
        { pattern: '**/*.aps.json' },
        { language: 'markdown' },
      ],
      codeLensProvider
    );
    context.subscriptions.push(codeLensDisposable, codeLensProvider);
  }

  // Register CodeAction provider for nudge coaching
  const nudgeProvider = new NudgeCodeActionProvider();
  const nudgeLanguages = [
    { language: 'typescript' },
    { language: 'typescriptreact' },
    { language: 'javascript' },
    { language: 'javascriptreact' },
    { language: 'html' },
    { language: 'css' },
    { language: 'scss' },
    { language: 'less' },
  ];
  const nudgeDisposable = vscode.languages.registerCodeActionsProvider(
    nudgeLanguages,
    nudgeProvider,
    { providedCodeActionKinds: NudgeCodeActionProvider.providedCodeActionKinds }
  );
  context.subscriptions.push(nudgeDisposable);

  // Register nudge display command
  context.subscriptions.push(
    vscode.commands.registerCommand('anvil.showNudge', (nudgeText: string, patternId: string) => {
      vscode.window.showInformationMessage(`[${patternId}] ${nudgeText}`);
    })
  );

  // Register commands
  registerCommands(
    context,
    anvilService,
    diagnosticsManager,
    statusBarManager,
    gateResultsProvider
  );

  // Register diagnostics collection
  context.subscriptions.push(diagnosticsManager.diagnosticCollection);

  // Register status bar
  context.subscriptions.push(statusBarManager.statusBarItem);

  await planWatcher.start();
  context.subscriptions.push(planWatcher);

  sourceWatcher = new SourceWatcher(diagnosticsManager, anvilService.getOutputChannel());
  await sourceWatcher.start();
  context.subscriptions.push(sourceWatcher);

  // Set context for when we have plan files
  await updatePlanContext();

  // Watch for file changes to update context
  const fileWatcher = vscode.workspace.createFileSystemWatcher('**/*.{plan.md,spec.md,aps.json}');
  context.subscriptions.push(
    fileWatcher,
    fileWatcher.onDidCreate(() => updatePlanContext()),
    fileWatcher.onDidDelete(() => updatePlanContext())
  );

  // Watch for active editor changes
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor) {
        updateIsPlanFileContext(editor.document);
      }
    })
  );

  // Set initial context for current editor
  if (vscode.window.activeTextEditor) {
    updateIsPlanFileContext(vscode.window.activeTextEditor.document);
  }

  // Show status bar
  statusBarManager.show();
  statusBarManager.setIdle();

  outputChannel.appendLine('Anvil extension activated successfully');
}

export function deactivate(): void {
  outputChannel?.appendLine('Anvil extension deactivating...');

  if (statusBarManager) {
    statusBarManager.dispose();
  }
  if (diagnosticsManager) {
    diagnosticsManager.dispose();
  }
  if (planWatcher) {
    planWatcher.dispose();
  }
  if (sourceWatcher) {
    sourceWatcher.dispose();
  }
}

async function updatePlanContext(): Promise<void> {
  const planFiles = await vscode.workspace.findFiles(
    '**/*.{plan.md,spec.md,aps.json}',
    '**/node_modules/**',
    1
  );
  await vscode.commands.executeCommand('setContext', 'anvil.hasPlans', planFiles.length > 0);
}

function updateIsPlanFileContext(document: vscode.TextDocument): void {
  const isPlanFile = isPlanDocument(document);
  vscode.commands.executeCommand('setContext', 'anvil.isPlanFile', isPlanFile);
}

function isPlanDocument(document: vscode.TextDocument): boolean {
  const fileName = document.fileName.toLowerCase();
  return (
    fileName.endsWith('.plan.md') ||
    fileName.endsWith('plan.md') ||
    fileName.endsWith('.spec.md') ||
    fileName.endsWith('spec.md') ||
    fileName.endsWith('.aps.json') ||
    // Check for BMAD patterns
    fileName.includes('prd') ||
    fileName.includes('architecture')
  );
}
