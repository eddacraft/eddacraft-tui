/**
 * Nudge CodeAction Provider
 *
 * Surfaces coaching nudges as "Rethink" hint actions in the VS Code
 * lightbulb menu for anti-pattern diagnostics. For patterns with
 * deterministic fixes (AP-004: @ts-ignore → @ts-expect-error), also
 * provides a "Fix" quick-fix action.
 *
 * @module providers/nudgeCodeActionProvider
 */

import * as vscode from 'vscode';
import { getEmbeddedAnalysisService } from '../services/embeddedAnalysis.js';

/**
 * Provides CodeActions for Anvil anti-pattern diagnostics:
 * - "Anvil: Rethink" — shows the nudge coaching text (hint)
 * - "Anvil: Fix" — applies deterministic fixes where available (quickfix)
 */
export class NudgeCodeActionProvider implements vscode.CodeActionProvider {
  static readonly providedCodeActionKinds = [vscode.CodeActionKind.QuickFix];

  provideCodeActions(
    document: vscode.TextDocument,
    range: vscode.Range | vscode.Selection,
    context: vscode.CodeActionContext
  ): vscode.CodeAction[] {
    const actions: vscode.CodeAction[] = [];

    for (const diagnostic of context.diagnostics) {
      if (diagnostic.source !== 'anvil:antipattern') continue;

      const patternId = this.getPatternId(diagnostic);
      if (!patternId) continue;

      // Look up nudge text from the pattern
      const service = getEmbeddedAnalysisService();
      const pattern = service.getPatternInfo(patternId);
      if (!pattern?.nudge) continue;

      // "Rethink" hint action — shows nudge as a notification
      const rethinkAction = new vscode.CodeAction(
        `Anvil: Rethink — ${this.truncate(pattern.nudge, 80)}`,
        vscode.CodeActionKind.QuickFix
      );
      rethinkAction.diagnostics = [diagnostic];
      rethinkAction.isPreferred = false;
      rethinkAction.command = {
        command: 'anvil.showNudge',
        title: 'Show Anvil Nudge',
        arguments: [pattern.nudge, patternId],
      };
      actions.push(rethinkAction);

      // Deterministic fix actions
      const fix = this.getDeterministicFix(patternId, document, diagnostic);
      if (fix) {
        actions.push(fix);
      }
    }

    return actions;
  }

  private getPatternId(diagnostic: vscode.Diagnostic): string | undefined {
    if (!diagnostic.code) return undefined;
    if (typeof diagnostic.code === 'object' && 'value' in diagnostic.code) {
      return String(diagnostic.code.value);
    }
    if (typeof diagnostic.code === 'string') {
      return diagnostic.code;
    }
    return undefined;
  }

  /**
   * Returns a deterministic fix CodeAction for patterns that have one.
   *
   * Currently supports:
   * - AP-004: Replace @ts-ignore with @ts-expect-error
   */
  private getDeterministicFix(
    patternId: string,
    document: vscode.TextDocument,
    diagnostic: vscode.Diagnostic
  ): vscode.CodeAction | undefined {
    if (patternId === 'AP-004') {
      return this.createTsIgnoreFix(document, diagnostic);
    }
    return undefined;
  }

  private createTsIgnoreFix(
    document: vscode.TextDocument,
    diagnostic: vscode.Diagnostic
  ): vscode.CodeAction {
    const action = new vscode.CodeAction(
      'Anvil: Fix — replace @ts-ignore with @ts-expect-error',
      vscode.CodeActionKind.QuickFix
    );
    action.diagnostics = [diagnostic];
    action.isPreferred = true;

    const lineText = document.getText(
      new vscode.Range(
        new vscode.Position(diagnostic.range.start.line, 0),
        new vscode.Position(diagnostic.range.start.line, 1000)
      )
    );

    const edit = new vscode.WorkspaceEdit();
    const tsIgnoreIndex = lineText.indexOf('@ts-ignore');
    if (tsIgnoreIndex >= 0) {
      const replaceRange = new vscode.Range(
        new vscode.Position(diagnostic.range.start.line, tsIgnoreIndex),
        new vscode.Position(diagnostic.range.start.line, tsIgnoreIndex + '@ts-ignore'.length)
      );
      edit.replace(document.uri, replaceRange, '@ts-expect-error');
    }
    action.edit = edit;

    return action;
  }

  private truncate(text: string, maxLength: number): string {
    if (text.length <= maxLength) return text;
    return text.substring(0, maxLength - 1) + '…';
  }
}
