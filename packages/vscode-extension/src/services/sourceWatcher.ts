import * as vscode from 'vscode';
import { getEmbeddedAnalysisService } from './embeddedAnalysis.js';
import type { DiagnosticsManager } from './diagnostics.js';
import { DEFAULT_ANALYSABLE_EXTENSIONS } from '@eddacraft/anvil-platform-config';

const ANALYSABLE_EXTENSIONS = [...DEFAULT_ANALYSABLE_EXTENSIONS, '.mts', '.cts'];
const DEBOUNCE_MS = 300;

export class SourceWatcher implements vscode.Disposable {
  private diagnosticsManager: DiagnosticsManager;
  private debounceTimers: Map<string, NodeJS.Timeout> = new Map();
  private disposables: vscode.Disposable[] = [];
  private outputChannel: vscode.OutputChannel;

  constructor(diagnosticsManager: DiagnosticsManager, outputChannel: vscode.OutputChannel) {
    this.diagnosticsManager = diagnosticsManager;
    this.outputChannel = outputChannel;
  }

  async start(): Promise<void> {
    const config = vscode.workspace.getConfiguration('anvil');

    if (!config.get<boolean>('autoValidate', true)) {
      return;
    }

    this.disposables.push(
      vscode.workspace.onDidSaveTextDocument((document) => {
        if (this.shouldAnalyse(document)) {
          this.debouncedAnalyse(document);
        }
      })
    );

    this.disposables.push(
      vscode.workspace.onDidOpenTextDocument((document) => {
        if (config.get<boolean>('validateOnOpen', true) && this.shouldAnalyse(document)) {
          this.analyseDocument(document);
        }
      })
    );

    await this.analyseOpenSourceFiles();
  }

  private async analyseOpenSourceFiles(): Promise<void> {
    const config = vscode.workspace.getConfiguration('anvil');
    if (!config.get<boolean>('validateOnOpen', true)) {
      return;
    }

    for (const document of vscode.workspace.textDocuments) {
      if (this.shouldAnalyse(document)) {
        await this.analyseDocument(document);
      }
    }
  }

  private shouldAnalyse(document: vscode.TextDocument): boolean {
    if (document.uri.scheme !== 'file') {
      return false;
    }

    const ext = document.fileName.substring(document.fileName.lastIndexOf('.'));
    if (!ANALYSABLE_EXTENSIONS.includes(ext.toLowerCase())) {
      return false;
    }

    if (document.fileName.includes('node_modules')) {
      return false;
    }

    return true;
  }

  private debouncedAnalyse(document: vscode.TextDocument): void {
    const key = document.uri.toString();

    const existingTimer = this.debounceTimers.get(key);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    const timer = setTimeout(() => {
      this.analyseDocument(document);
      this.debounceTimers.delete(key);
    }, DEBOUNCE_MS);

    this.debounceTimers.set(key, timer);
  }

  private async analyseDocument(document: vscode.TextDocument): Promise<void> {
    try {
      const service = getEmbeddedAnalysisService();
      const content = document.getText();
      const filePath = document.uri.fsPath;

      const result = service.analyseFile(filePath, content);

      this.diagnosticsManager.updateFromAnalysis(document.uri, result);

      if (result.warnings.length > 0) {
        this.outputChannel.appendLine(
          `[${new Date().toISOString()}] Anti-pattern analysis: ${filePath} - ${result.warnings.length} warning(s) (${result.duration}ms)`
        );
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.outputChannel.appendLine(
        `[${new Date().toISOString()}] Anti-pattern analysis error: ${message}`
      );
    }
  }

  dispose(): void {
    for (const timer of this.debounceTimers.values()) {
      clearTimeout(timer);
    }
    this.debounceTimers.clear();

    for (const disposable of this.disposables) {
      disposable.dispose();
    }
    this.disposables = [];
  }
}
