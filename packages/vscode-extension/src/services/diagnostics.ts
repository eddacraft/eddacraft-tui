import * as vscode from 'vscode';
import * as path from 'path';
import type { ValidationResult, GateResults, GateDetail } from './anvilService.js';
import type { AnalysisResult, AnalysisWarning } from './embeddedAnalysis.js';

export class DiagnosticsManager implements vscode.Disposable {
  public readonly diagnosticCollection: vscode.DiagnosticCollection;
  private gateDetailFiles: Set<string> = new Set();

  constructor() {
    this.diagnosticCollection = vscode.languages.createDiagnosticCollection('anvil');
  }

  private getWorkspaceRoot(): string {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';
  }

  private resolveFilePath(filePath: string): string {
    if (path.isAbsolute(filePath)) {
      return filePath;
    }
    return path.join(this.getWorkspaceRoot(), filePath);
  }

  updateFromValidation(uri: vscode.Uri, result: ValidationResult): void {
    const diagnostics: vscode.Diagnostic[] = [];

    // Add errors
    for (const error of result.errors) {
      const diagnostic = this.createDiagnostic(
        error.message,
        error.line,
        error.column,
        vscode.DiagnosticSeverity.Error,
        'validation'
      );
      diagnostics.push(diagnostic);
    }

    // Add warnings
    for (const warning of result.warnings) {
      const diagnostic = this.createDiagnostic(
        warning.message,
        warning.line,
        warning.column,
        vscode.DiagnosticSeverity.Warning,
        'validation'
      );
      diagnostics.push(diagnostic);
    }

    this.diagnosticCollection.set(uri, diagnostics);
  }

  updateFromGateResults(uri: vscode.Uri, results: GateResults): void {
    this.clearPreviousGateDetailFiles();

    const diagnostics: vscode.Diagnostic[] = [];
    const newGateDetailFiles: Map<string, vscode.Diagnostic[]> = new Map();

    for (const gate of results.gates) {
      if (gate.status === 'failed' || gate.status === 'error') {
        const diagnostic = this.createDiagnostic(
          `Gate "${gate.name}" ${gate.status}: ${gate.message || 'Check failed'}`,
          undefined,
          undefined,
          vscode.DiagnosticSeverity.Error,
          `gate:${gate.name}`
        );
        diagnostics.push(diagnostic);

        if (gate.details) {
          for (const detail of gate.details) {
            const severity = this.mapDetailSeverity(detail.type);
            const detailDiagnostic = this.createDiagnostic(
              detail.message,
              detail.line,
              detail.column,
              severity,
              `gate:${gate.name}`
            );

            if (detail.file) {
              const resolvedPath = this.resolveFilePath(detail.file);
              if (resolvedPath !== uri.fsPath) {
                const existing = newGateDetailFiles.get(resolvedPath) || [];
                existing.push(detailDiagnostic);
                newGateDetailFiles.set(resolvedPath, existing);
              } else {
                diagnostics.push(detailDiagnostic);
              }
            } else {
              diagnostics.push(detailDiagnostic);
            }
          }
        }
      }
    }

    for (const [filePath, fileDiagnostics] of newGateDetailFiles) {
      const detailUri = vscode.Uri.file(filePath);
      const existing = this.diagnosticCollection.get(detailUri) || [];
      const nonGateDiagnostics = existing.filter((d) => !d.source?.startsWith('anvil:gate'));
      this.diagnosticCollection.set(detailUri, [...nonGateDiagnostics, ...fileDiagnostics]);
      this.gateDetailFiles.add(filePath);
    }

    const existing = this.diagnosticCollection.get(uri) || [];
    const validationDiagnostics = existing.filter((d) => d.source === 'anvil:validation');

    this.diagnosticCollection.set(uri, [...validationDiagnostics, ...diagnostics]);
  }

  private clearPreviousGateDetailFiles(): void {
    for (const filePath of this.gateDetailFiles) {
      const detailUri = vscode.Uri.file(filePath);
      const existing = this.diagnosticCollection.get(detailUri) || [];
      const nonGateDiagnostics = existing.filter((d) => !d.source?.startsWith('anvil:gate'));
      if (nonGateDiagnostics.length > 0) {
        this.diagnosticCollection.set(detailUri, nonGateDiagnostics);
      } else {
        this.diagnosticCollection.delete(detailUri);
      }
    }
    this.gateDetailFiles.clear();
  }

  updateFromAnalysis(uri: vscode.Uri, result: AnalysisResult): void {
    const diagnostics: vscode.Diagnostic[] = result.warnings.map((warning) =>
      this.createDiagnosticFromWarning(warning)
    );

    const existing = this.diagnosticCollection.get(uri) || [];
    const nonAnalysisDiagnostics = existing.filter(
      (d) => !d.source?.startsWith('anvil:antipattern')
    );

    this.diagnosticCollection.set(uri, [...nonAnalysisDiagnostics, ...diagnostics]);
  }

  private createDiagnosticFromWarning(warning: AnalysisWarning): vscode.Diagnostic {
    const startLine = Math.max(0, warning.location.line - 1);
    const startCol = warning.location.column;
    const endLine = warning.location.endLine ? warning.location.endLine - 1 : startLine;
    const endCol = warning.location.endColumn ?? startCol + 20;

    const range = new vscode.Range(
      new vscode.Position(startLine, startCol),
      new vscode.Position(endLine, endCol)
    );

    const severity = this.mapWarningSeverity(warning.severity);
    const diagnostic = new vscode.Diagnostic(range, warning.message, severity);

    diagnostic.source = `anvil:antipattern`;
    diagnostic.code = {
      value: warning.id,
      target: warning.documentationUrl
        ? vscode.Uri.parse(warning.documentationUrl)
        : vscode.Uri.parse(`https://github.com/EddaCraft/anvil-001#${warning.id.toLowerCase()}`),
    };

    diagnostic.relatedInformation = [
      new vscode.DiagnosticRelatedInformation(
        new vscode.Location(vscode.Uri.file(warning.location.file), range),
        `${warning.explanation}\n\nSuggestion: ${warning.suggestion}`
      ),
    ];

    return diagnostic;
  }

  private mapWarningSeverity(severity: 'error' | 'warning' | 'info'): vscode.DiagnosticSeverity {
    switch (severity) {
      case 'error':
        return vscode.DiagnosticSeverity.Error;
      case 'warning':
        return vscode.DiagnosticSeverity.Warning;
      case 'info':
        return vscode.DiagnosticSeverity.Information;
      default:
        return vscode.DiagnosticSeverity.Warning;
    }
  }

  clearForUri(uri: vscode.Uri): void {
    this.diagnosticCollection.delete(uri);
  }

  clearAll(): void {
    this.diagnosticCollection.clear();
  }

  private createDiagnostic(
    message: string,
    line: number | undefined,
    column: number | undefined,
    severity: vscode.DiagnosticSeverity,
    source: string
  ): vscode.Diagnostic {
    // Default to first line if no position provided
    const startLine = line !== undefined ? Math.max(0, line - 1) : 0;
    const startCol = column !== undefined ? Math.max(0, column - 1) : 0;

    const range = new vscode.Range(
      new vscode.Position(startLine, startCol),
      new vscode.Position(startLine, startCol + 100) // Highlight the line
    );

    const diagnostic = new vscode.Diagnostic(range, message, severity);
    diagnostic.source = `anvil:${source}`;

    return diagnostic;
  }

  private mapDetailSeverity(type: GateDetail['type']): vscode.DiagnosticSeverity {
    switch (type) {
      case 'error':
        return vscode.DiagnosticSeverity.Error;
      case 'warning':
        return vscode.DiagnosticSeverity.Warning;
      case 'info':
        return vscode.DiagnosticSeverity.Information;
      default:
        return vscode.DiagnosticSeverity.Error;
    }
  }

  dispose(): void {
    this.diagnosticCollection.dispose();
  }
}
