import * as vscode from 'vscode';
import type { AnvilService } from './anvilService.js';
import type { DiagnosticsManager } from './diagnostics.js';

export class PlanWatcher implements vscode.Disposable {
  private anvilService: AnvilService;
  private diagnosticsManager: DiagnosticsManager;
  private fileWatcher: vscode.FileSystemWatcher | undefined;
  private documentWatcher: vscode.Disposable | undefined;
  private debounceTimers: Map<string, NodeJS.Timeout> = new Map();
  private readonly debounceMs = 500;

  constructor(anvilService: AnvilService, diagnosticsManager: DiagnosticsManager) {
    this.anvilService = anvilService;
    this.diagnosticsManager = diagnosticsManager;
  }

  async start(): Promise<void> {
    const config = vscode.workspace.getConfiguration('anvil');

    // Watch for file saves
    if (config.get<boolean>('autoValidate', true)) {
      this.documentWatcher = vscode.workspace.onDidSaveTextDocument((document) => {
        if (this.isPlanFile(document)) {
          this.debouncedValidate(document.uri);
        }
      });
    }

    // Watch for file opens
    if (config.get<boolean>('validateOnOpen', true)) {
      vscode.workspace.onDidOpenTextDocument((document) => {
        if (this.isPlanFile(document)) {
          this.debouncedValidate(document.uri);
        }
      });
    }

    // Watch for file changes in workspace
    this.fileWatcher = vscode.workspace.createFileSystemWatcher('**/*.{plan.md,spec.md,aps.json}');

    this.fileWatcher.onDidChange((uri) => {
      // Only validate if file is open
      const openDoc = vscode.workspace.textDocuments.find(
        (doc) => doc.uri.toString() === uri.toString()
      );
      if (openDoc) {
        this.debouncedValidate(uri);
      }
    });

    this.fileWatcher.onDidDelete((uri) => {
      this.diagnosticsManager.clearForUri(uri);
    });

    // Validate currently open plan files
    await this.validateOpenPlanFiles();
  }

  private async validateOpenPlanFiles(): Promise<void> {
    const config = vscode.workspace.getConfiguration('anvil');
    if (!config.get<boolean>('validateOnOpen', true)) {
      return;
    }

    for (const document of vscode.workspace.textDocuments) {
      if (this.isPlanFile(document)) {
        await this.validateDocument(document.uri);
      }
    }
  }

  private debouncedValidate(uri: vscode.Uri): void {
    const key = uri.toString();

    // Clear existing timer
    const existingTimer = this.debounceTimers.get(key);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    // Set new timer
    const timer = setTimeout(() => {
      this.validateDocument(uri);
      this.debounceTimers.delete(key);
    }, this.debounceMs);

    this.debounceTimers.set(key, timer);
  }

  private async validateDocument(uri: vscode.Uri): Promise<void> {
    try {
      const result = await this.anvilService.validate(uri.fsPath);
      this.diagnosticsManager.updateFromValidation(uri, result);
    } catch (error) {
      console.error(`Failed to validate ${uri.fsPath}:`, error);
    }
  }

  private isPlanFile(document: vscode.TextDocument): boolean {
    const fileName = document.fileName.toLowerCase();
    return (
      fileName.endsWith('.plan.md') ||
      fileName.endsWith('plan.md') ||
      fileName.endsWith('.spec.md') ||
      fileName.endsWith('spec.md') ||
      fileName.endsWith('.aps.json') ||
      // BMAD patterns
      (fileName.includes('prd') && fileName.endsWith('.md')) ||
      (fileName.includes('architecture') && fileName.endsWith('.md'))
    );
  }

  dispose(): void {
    // Clear all debounce timers
    for (const timer of this.debounceTimers.values()) {
      clearTimeout(timer);
    }
    this.debounceTimers.clear();

    if (this.fileWatcher) {
      this.fileWatcher.dispose();
    }

    if (this.documentWatcher) {
      this.documentWatcher.dispose();
    }
  }
}
