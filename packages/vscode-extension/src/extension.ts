import * as vscode from 'vscode';
import { registerCommands } from './commands/index.js';
import { StatusBarManager } from './services/statusBar.js';
import { DiagnosticsManager } from './services/diagnostics.js';
import { GateResultsProvider } from './providers/gateResultsProvider.js';
import { PlanCodeLensProvider } from './providers/codeLensProvider.js';
import { PlanWatcher } from './services/planWatcher.js';
import { AnvilService } from './services/anvilService.js';

let statusBarManager: StatusBarManager;
let diagnosticsManager: DiagnosticsManager;
let gateResultsProvider: GateResultsProvider;
let planWatcher: PlanWatcher;
let anvilService: AnvilService;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  console.log('Anvil extension is activating...');

  // Initialise core services
  anvilService = new AnvilService(context);
  diagnosticsManager = new DiagnosticsManager();
  statusBarManager = new StatusBarManager();
  gateResultsProvider = new GateResultsProvider(anvilService);
  planWatcher = new PlanWatcher(anvilService, diagnosticsManager);

  // Register tree view
  const treeView = vscode.window.createTreeView('anvilGateResults', {
    treeDataProvider: gateResultsProvider,
    showCollapseAll: true,
  });
  context.subscriptions.push(treeView);

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
    context.subscriptions.push(codeLensDisposable);
  }

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

  // Start watching for plan files
  await planWatcher.start();
  context.subscriptions.push(planWatcher);

  // Set context for when we have plan files
  await updatePlanContext();

  // Watch for file changes to update context
  const fileWatcher = vscode.workspace.createFileSystemWatcher('**/*.{plan.md,spec.md,aps.json}');
  fileWatcher.onDidCreate(() => updatePlanContext());
  fileWatcher.onDidDelete(() => updatePlanContext());
  context.subscriptions.push(fileWatcher);

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

  console.log('Anvil extension activated successfully');
}

export function deactivate(): void {
  console.log('Anvil extension deactivating...');

  if (statusBarManager) {
    statusBarManager.dispose();
  }
  if (diagnosticsManager) {
    diagnosticsManager.dispose();
  }
  if (planWatcher) {
    planWatcher.dispose();
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
