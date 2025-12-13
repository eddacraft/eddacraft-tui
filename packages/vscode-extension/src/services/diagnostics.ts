import * as vscode from 'vscode';
import type { ValidationResult, GateResults, GateDetail } from './anvilService.js';

export class DiagnosticsManager implements vscode.Disposable {
  public readonly diagnosticCollection: vscode.DiagnosticCollection;

  constructor() {
    this.diagnosticCollection = vscode.languages.createDiagnosticCollection('anvil');
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
    const diagnostics: vscode.Diagnostic[] = [];

    for (const gate of results.gates) {
      if (gate.status === 'failed' || gate.status === 'error') {
        // Add main gate failure
        const diagnostic = this.createDiagnostic(
          `Gate "${gate.name}" ${gate.status}: ${gate.message || 'Check failed'}`,
          undefined,
          undefined,
          vscode.DiagnosticSeverity.Error,
          `gate:${gate.name}`
        );
        diagnostics.push(diagnostic);

        // Add detailed issues if available
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

            // If detail has a different file, we need to handle it separately
            if (detail.file && detail.file !== uri.fsPath) {
              const detailUri = vscode.Uri.file(detail.file);
              const existing = this.diagnosticCollection.get(detailUri) || [];
              this.diagnosticCollection.set(detailUri, [...existing, detailDiagnostic]);
            } else {
              diagnostics.push(detailDiagnostic);
            }
          }
        }
      }
    }

    // Merge with existing diagnostics (preserve validation errors)
    const existing = this.diagnosticCollection.get(uri) || [];
    const validationDiagnostics = existing.filter((d) => d.source === 'anvil:validation');

    this.diagnosticCollection.set(uri, [...validationDiagnostics, ...diagnostics]);
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
