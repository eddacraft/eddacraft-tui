import * as vscode from 'vscode';
import type { AnvilService, GateResults, GateResult } from '../services/anvilService.js';

type TreeItem = GateTreeItem | GateDetailItem | FileCategoryItem;

export class GateResultsProvider implements vscode.TreeDataProvider<TreeItem> {
  private _onDidChangeTreeData: vscode.EventEmitter<TreeItem | undefined | null | void> =
    new vscode.EventEmitter<TreeItem | undefined | null | void>();
  readonly onDidChangeTreeData: vscode.Event<TreeItem | undefined | null | void> =
    this._onDidChangeTreeData.event;

  private resultsByFile: Map<string, GateResults> = new Map();

  constructor(_anvilService: AnvilService) {
    // AnvilService stored for future use (e.g., auto-refresh results)
  }

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  updateResults(filePath: string, results: GateResults): void {
    this.resultsByFile.set(filePath, results);
    this.refresh();
  }

  clearResults(filePath?: string): void {
    if (filePath) {
      this.resultsByFile.delete(filePath);
    } else {
      this.resultsByFile.clear();
    }
    this.refresh();
  }

  getTreeItem(element: TreeItem): vscode.TreeItem {
    return element;
  }

  getChildren(element?: TreeItem): Thenable<TreeItem[]> {
    if (!element) {
      // Root level - show files with results
      return Promise.resolve(this.getFileCategories());
    }

    if (element instanceof FileCategoryItem) {
      // Show gates for this file
      return Promise.resolve(this.getGatesForFile(element.filePath));
    }

    if (element instanceof GateTreeItem) {
      // Show details for this gate
      return Promise.resolve(this.getGateDetails(element));
    }

    return Promise.resolve([]);
  }

  private getFileCategories(): FileCategoryItem[] {
    const items: FileCategoryItem[] = [];

    for (const [filePath, results] of this.resultsByFile) {
      const fileName = filePath.split('/').pop() || filePath;
      const passedCount = results.gates.filter((g) => g.status === 'passed').length;
      const totalCount = results.gates.length;
      const allPassed = results.success;

      items.push(
        new FileCategoryItem(filePath, fileName, `${passedCount}/${totalCount} passed`, allPassed)
      );
    }

    return items;
  }

  private getGatesForFile(filePath: string): GateTreeItem[] {
    const results = this.resultsByFile.get(filePath);
    if (!results) {
      return [];
    }

    return results.gates.map((gate) => new GateTreeItem(gate, filePath));
  }

  private getGateDetails(gateItem: GateTreeItem): GateDetailItem[] {
    const gate = gateItem.gate;

    if (!gate.details || gate.details.length === 0) {
      if (gate.message) {
        return [new GateDetailItem(gate.message, 'info')];
      }
      return [];
    }

    return gate.details.map(
      (detail) => new GateDetailItem(detail.message, detail.type, detail.file, detail.line)
    );
  }
}

class FileCategoryItem extends vscode.TreeItem {
  constructor(
    public readonly filePath: string,
    fileName: string,
    description: string,
    allPassed: boolean
  ) {
    super(fileName, vscode.TreeItemCollapsibleState.Expanded);

    this.description = description;
    this.tooltip = filePath;
    this.iconPath = allPassed
      ? new vscode.ThemeIcon('pass', new vscode.ThemeColor('testing.iconPassed'))
      : new vscode.ThemeIcon('error', new vscode.ThemeColor('testing.iconFailed'));

    this.contextValue = 'fileCategory';
  }
}

class GateTreeItem extends vscode.TreeItem {
  constructor(
    public readonly gate: GateResult,
    public readonly filePath: string
  ) {
    super(
      gate.name,
      gate.details && gate.details.length > 0
        ? vscode.TreeItemCollapsibleState.Collapsed
        : vscode.TreeItemCollapsibleState.None
    );

    this.description = gate.message || this.getStatusDescription(gate.status);
    this.tooltip = this.getTooltip();
    this.iconPath = this.getIcon();
    this.contextValue = 'gate';
  }

  private getStatusDescription(status: 'passed' | 'failed' | 'skipped' | 'error'): string {
    switch (status) {
      case 'passed':
        return 'Passed';
      case 'failed':
        return 'Failed';
      case 'skipped':
        return 'Skipped';
      case 'error':
        return 'Error';
    }
  }

  private getIcon(): vscode.ThemeIcon {
    switch (this.gate.status) {
      case 'passed':
        return new vscode.ThemeIcon('pass', new vscode.ThemeColor('testing.iconPassed'));
      case 'failed':
        return new vscode.ThemeIcon('error', new vscode.ThemeColor('testing.iconFailed'));
      case 'skipped':
        return new vscode.ThemeIcon(
          'debug-step-over',
          new vscode.ThemeColor('testing.iconSkipped')
        );
      case 'error':
        return new vscode.ThemeIcon('warning', new vscode.ThemeColor('testing.iconErrored'));
    }
  }

  private getTooltip(): string {
    let tooltip = `${this.gate.name}: ${this.gate.status}`;
    if (this.gate.duration) {
      tooltip += ` (${this.gate.duration}ms)`;
    }
    if (this.gate.message) {
      tooltip += `\n${this.gate.message}`;
    }
    return tooltip;
  }
}

class GateDetailItem extends vscode.TreeItem {
  constructor(message: string, type: 'error' | 'warning' | 'info', file?: string, line?: number) {
    super(message, vscode.TreeItemCollapsibleState.None);

    this.iconPath = this.getIcon(type);

    if (file && line) {
      this.description = `${file}:${line}`;
      this.command = {
        command: 'vscode.open',
        title: 'Open File',
        arguments: [
          vscode.Uri.file(file),
          {
            selection: new vscode.Range(
              new vscode.Position(line - 1, 0),
              new vscode.Position(line - 1, 0)
            ),
          },
        ],
      };
    }

    this.contextValue = 'gateDetail';
  }

  private getIcon(type: 'error' | 'warning' | 'info'): vscode.ThemeIcon {
    switch (type) {
      case 'error':
        return new vscode.ThemeIcon('error', new vscode.ThemeColor('testing.iconFailed'));
      case 'warning':
        return new vscode.ThemeIcon(
          'warning',
          new vscode.ThemeColor('problemsWarningIcon.foreground')
        );
      case 'info':
        return new vscode.ThemeIcon('info', new vscode.ThemeColor('problemsInfoIcon.foreground'));
    }
  }
}
