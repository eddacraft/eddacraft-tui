import * as vscode from 'vscode';
import type { AnvilService } from '../services/anvilService.js';
import type { DiagnosticsManager } from '../services/diagnostics.js';
import type { StatusBarManager } from '../services/statusBar.js';
import type { GateResultsProvider } from '../providers/gateResultsProvider.js';

export function registerCommands(
  context: vscode.ExtensionContext,
  anvilService: AnvilService,
  diagnosticsManager: DiagnosticsManager,
  statusBarManager: StatusBarManager,
  gateResultsProvider: GateResultsProvider
): void {
  // Validate command
  context.subscriptions.push(
    vscode.commands.registerCommand('anvil.validate', async () => {
      const fileUri = await selectPlanFile();
      if (fileUri) {
        await runValidation(fileUri, anvilService, diagnosticsManager, statusBarManager);
      }
    })
  );

  // Validate current file command
  context.subscriptions.push(
    vscode.commands.registerCommand('anvil.validateCurrentFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showWarningMessage('No file is currently open');
        return;
      }

      await runValidation(editor.document.uri, anvilService, diagnosticsManager, statusBarManager);
    })
  );

  // Gate command
  context.subscriptions.push(
    vscode.commands.registerCommand('anvil.gate', async () => {
      const fileUri = await selectPlanFile();
      if (fileUri) {
        await runGates(
          fileUri,
          anvilService,
          diagnosticsManager,
          statusBarManager,
          gateResultsProvider
        );
      }
    })
  );

  // Gate current file command
  context.subscriptions.push(
    vscode.commands.registerCommand('anvil.gateCurrentFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showWarningMessage('No file is currently open');
        return;
      }

      await runGates(
        editor.document.uri,
        anvilService,
        diagnosticsManager,
        statusBarManager,
        gateResultsProvider
      );
    })
  );

  // Export command
  context.subscriptions.push(
    vscode.commands.registerCommand('anvil.export', async (uri?: vscode.Uri) => {
      const fileUri = uri || (await selectPlanFile());
      if (!fileUri) {
        return;
      }

      const format = await vscode.window.showQuickPick(
        [
          { label: 'APS JSON', value: 'aps', description: 'Anvil Plan Specification format' },
          { label: 'SpecKit', value: 'speckit', description: 'GitHub spec-kit format' },
          { label: 'BMAD', value: 'bmad', description: 'BMAD PRD format' },
          { label: 'Generic Markdown', value: 'generic', description: 'Generic markdown format' },
        ],
        {
          placeHolder: 'Select export format',
          title: 'Export Plan',
        }
      );

      if (!format) {
        return;
      }

      statusBarManager.setValidating('Exporting...');

      try {
        const result = await anvilService.exportPlan(fileUri.fsPath, format.value);

        if (result.success) {
          statusBarManager.setSuccess(`Exported to ${format.label}`);
          vscode.window.showInformationMessage(`Plan exported to ${result.outputPath}`);

          // Open the exported file
          if (result.outputPath) {
            const doc = await vscode.workspace.openTextDocument(result.outputPath);
            await vscode.window.showTextDocument(doc);
          }
        } else {
          statusBarManager.setError('Export failed');
          vscode.window.showErrorMessage(`Export failed: ${result.error}`);
        }
      } catch (error) {
        statusBarManager.setError('Export failed');
        const message = error instanceof Error ? error.message : String(error);
        vscode.window.showErrorMessage(`Export failed: ${message}`);
      }
    })
  );

  // Refresh command
  context.subscriptions.push(
    vscode.commands.registerCommand('anvil.refresh', () => {
      gateResultsProvider.refresh();
      vscode.window.showInformationMessage('Anvil results refreshed');
    })
  );

  // Show output command
  context.subscriptions.push(
    vscode.commands.registerCommand('anvil.showOutput', () => {
      anvilService.getOutputChannel().show();
    })
  );
}

async function selectPlanFile(): Promise<vscode.Uri | undefined> {
  const planFiles = await vscode.workspace.findFiles(
    '**/*.{plan.md,spec.md,aps.json}',
    '**/node_modules/**',
    50
  );

  if (planFiles.length === 0) {
    vscode.window.showWarningMessage('No plan files found in workspace');
    return undefined;
  }

  if (planFiles.length === 1) {
    return planFiles[0];
  }

  const items = planFiles.map((uri) => ({
    label: vscode.workspace.asRelativePath(uri),
    uri,
  }));

  const selected = await vscode.window.showQuickPick(items, {
    placeHolder: 'Select a plan file',
    title: 'Anvil',
  });

  return selected?.uri;
}

async function runValidation(
  uri: vscode.Uri,
  anvilService: AnvilService,
  diagnosticsManager: DiagnosticsManager,
  statusBarManager: StatusBarManager
): Promise<void> {
  const fileName = vscode.workspace.asRelativePath(uri);
  statusBarManager.setValidating(fileName);

  try {
    const result = await anvilService.validate(uri.fsPath);
    diagnosticsManager.updateFromValidation(uri, result);

    if (result.success) {
      statusBarManager.setSuccess(`Valid (${result.format || 'unknown format'})`);

      if (result.warnings.length > 0) {
        statusBarManager.setWarning(`Valid with ${result.warnings.length} warning(s)`);
      }

      vscode.window.showInformationMessage(`Plan is valid (ID: ${result.planId || 'N/A'})`);
    } else {
      statusBarManager.setError(`${result.errors.length} validation error(s)`);
      vscode.window.showErrorMessage(
        `Validation failed: ${result.errors[0]?.message || 'Unknown error'}`
      );
    }
  } catch (error) {
    statusBarManager.setError('Validation failed');
    const message = error instanceof Error ? error.message : String(error);
    vscode.window.showErrorMessage(`Validation failed: ${message}`);
  }
}

async function runGates(
  uri: vscode.Uri,
  anvilService: AnvilService,
  diagnosticsManager: DiagnosticsManager,
  statusBarManager: StatusBarManager,
  gateResultsProvider: GateResultsProvider
): Promise<void> {
  const fileName = vscode.workspace.asRelativePath(uri);
  statusBarManager.setRunningGates(fileName);

  try {
    const results = await anvilService.runGates(uri.fsPath);

    diagnosticsManager.updateFromGateResults(uri, results);
    gateResultsProvider.updateResults(uri.fsPath, results);

    const passed = results.gates.filter((g) => g.status === 'passed').length;
    const total = results.gates.length;

    if (results.success) {
      statusBarManager.setSuccess(`All gates passed (${passed}/${total})`);
      vscode.window.showInformationMessage(`All quality gates passed (${passed}/${total})`);
    } else {
      const failed = results.gates.filter((g) => g.status === 'failed');
      statusBarManager.setError(`${failed.length} gate(s) failed`);
      vscode.window.showErrorMessage(
        `Quality gates failed: ${failed.map((g) => g.name).join(', ')}`
      );
    }
  } catch (error) {
    statusBarManager.setError('Gates failed');
    const message = error instanceof Error ? error.message : String(error);
    vscode.window.showErrorMessage(`Gates failed: ${message}`);
  }
}
