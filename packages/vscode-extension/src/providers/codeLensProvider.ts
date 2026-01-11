import * as vscode from 'vscode';
import type { AnvilService } from '../services/anvilService.js';

export class PlanCodeLensProvider implements vscode.CodeLensProvider, vscode.Disposable {
  private _onDidChangeCodeLenses: vscode.EventEmitter<void> = new vscode.EventEmitter<void>();
  readonly onDidChangeCodeLenses: vscode.Event<void> = this._onDidChangeCodeLenses.event;

  private anvilService: AnvilService;

  constructor(anvilService: AnvilService) {
    this.anvilService = anvilService;
  }

  dispose(): void {
    this._onDidChangeCodeLenses.dispose();
  }

  refresh(): void {
    this._onDidChangeCodeLenses.fire();
  }

  provideCodeLenses(
    document: vscode.TextDocument,
    _token: vscode.CancellationToken
  ): vscode.ProviderResult<vscode.CodeLens[]> {
    if (!this.isPlanFile(document)) {
      return [];
    }

    const codeLenses: vscode.CodeLens[] = [];
    const topOfFile = new vscode.Range(0, 0, 0, 0);

    // Validate CodeLens
    codeLenses.push(
      new vscode.CodeLens(topOfFile, {
        title: '$(check) Validate',
        command: 'anvil.validateCurrentFile',
        tooltip: 'Validate this plan file',
      })
    );

    // Gate CodeLens
    codeLenses.push(
      new vscode.CodeLens(topOfFile, {
        title: '$(shield) Run Gates',
        command: 'anvil.gateCurrentFile',
        tooltip: 'Run quality gates on this plan',
      })
    );

    // Export CodeLens
    codeLenses.push(
      new vscode.CodeLens(topOfFile, {
        title: '$(export) Export',
        command: 'anvil.export',
        tooltip: 'Export this plan to another format',
        arguments: [document.uri],
      })
    );

    // Add last validation status if available
    const lastResult = this.anvilService.getLastValidationResult(document.uri.fsPath);
    if (lastResult) {
      const statusIcon = lastResult.success ? '$(pass)' : '$(error)';
      const statusText = lastResult.success ? 'Valid' : `${lastResult.errors.length} error(s)`;

      codeLenses.push(
        new vscode.CodeLens(topOfFile, {
          title: `${statusIcon} ${statusText}`,
          command: 'anvil.showOutput',
          tooltip: lastResult.success
            ? `Plan is valid (ID: ${lastResult.planId || 'N/A'})`
            : `Validation failed: ${lastResult.errors.map((e) => e.message).join(', ')}`,
        })
      );
    }

    // Add gate results if available
    const lastGateResults = this.anvilService.getLastGateResults(document.uri.fsPath);
    if (lastGateResults) {
      const passed = lastGateResults.gates.filter((g) => g.status === 'passed').length;
      const total = lastGateResults.gates.length;
      const statusIcon = lastGateResults.success ? '$(pass)' : '$(error)';

      codeLenses.push(
        new vscode.CodeLens(topOfFile, {
          title: `${statusIcon} Gates: ${passed}/${total}`,
          command: 'anvil.showOutput',
          tooltip: `Gate results: ${lastGateResults.gates.map((g) => `${g.name}: ${g.status}`).join(', ')}`,
        })
      );
    }

    // Find task sections and add per-task CodeLens
    const taskLenses = this.findTaskCodeLenses(document);
    codeLenses.push(...taskLenses);

    return codeLenses;
  }

  private findTaskCodeLenses(document: vscode.TextDocument): vscode.CodeLens[] {
    const codeLenses: vscode.CodeLens[] = [];
    const text = document.getText();
    const lines = text.split('\n');

    // Look for task patterns in markdown
    // ## Task: xxx or ### Task xxx or - [ ] Task
    const taskPatterns = [
      /^##\s+Task\s*[:.]?\s*(.+)$/i,
      /^###\s+Task\s*[:.]?\s*(.+)$/i,
      /^##\s+(.+)$/,
      /^-\s*\[\s*[x ]?\s*\]\s*(.+)$/i,
    ];

    for (let lineNum = 0; lineNum < lines.length; lineNum++) {
      const line = lines[lineNum];

      for (const pattern of taskPatterns) {
        const match = line.match(pattern);
        if (match) {
          const range = new vscode.Range(lineNum, 0, lineNum, line.length);

          // Add a "Jump to implementation" CodeLens for task headers
          if (line.startsWith('##')) {
            codeLenses.push(
              new vscode.CodeLens(range, {
                title: '$(symbol-method) View Task',
                command: 'editor.action.peekDefinition',
                tooltip: 'View task details',
              })
            );
          }

          break; // Only match first pattern per line
        }
      }
    }

    return codeLenses;
  }

  private isPlanFile(document: vscode.TextDocument): boolean {
    const fileName = document.fileName.toLowerCase();
    const isPlanExtension =
      fileName.endsWith('.plan.md') ||
      fileName.endsWith('plan.md') ||
      fileName.endsWith('.spec.md') ||
      fileName.endsWith('spec.md') ||
      fileName.endsWith('.aps.json');

    if (isPlanExtension) {
      return true;
    }

    // Check for BMAD patterns
    if (
      fileName.endsWith('.md') &&
      (fileName.includes('prd') || fileName.includes('architecture'))
    ) {
      return true;
    }

    // Check document content for plan markers
    if (document.languageId === 'markdown') {
      const content = document.getText(new vscode.Range(0, 0, 20, 0)); // First 20 lines
      return (
        content.includes('## Tasks') ||
        content.includes('## Plan') ||
        content.includes('schema_version') ||
        content.includes('anvil:') ||
        content.includes('<!-- speckit') ||
        content.includes('<!-- bmad')
      );
    }

    return false;
  }
}
