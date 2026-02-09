import * as vscode from 'vscode';
import * as path from 'node:path';
import type { AnvilService, GateResults, GateResult } from '../services/anvilService.js';

type TreeItem =
  | FileCategoryItem
  | GateTreeItem
  | GateDetailItem
  | ViolationCategoryItem
  | ArchViolationItem
  | PolicyViolationItem;

interface ArchitectureViolation {
  from: string;
  to: string;
  rule: string;
  severity: 'error' | 'warn' | 'info' | 'ignore';
  cycle?: string[];
}

interface PolicyViolation {
  rule: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  path?: string;
  policy?: string;
}

interface ExtendedGateResult extends GateResult {
  violations?: ArchitectureViolation[];
  violationsByType?: Record<string, number>;
  violationsByPolicy?: Record<string, PolicyViolation[]>;
}

export class GateResultsProvider implements vscode.TreeDataProvider<TreeItem>, vscode.Disposable {
  private _onDidChangeTreeData: vscode.EventEmitter<TreeItem | undefined | null | void> =
    new vscode.EventEmitter<TreeItem | undefined | null | void>();
  readonly onDidChangeTreeData: vscode.Event<TreeItem | undefined | null | void> =
    this._onDidChangeTreeData.event;

  private resultsByFile: Map<string, GateResults> = new Map();
  private workspaceRoot: string;

  constructor(_anvilService: AnvilService) {
    this.workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';
  }

  dispose(): void {
    this._onDidChangeTreeData.dispose();
    this.resultsByFile.clear();
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
      return Promise.resolve(this.getFileCategories());
    }

    if (element instanceof FileCategoryItem) {
      return Promise.resolve(this.getGatesForFile(element.filePath));
    }

    if (element instanceof GateTreeItem) {
      return Promise.resolve(this.getGateChildren(element));
    }

    if (element instanceof ViolationCategoryItem) {
      return Promise.resolve(element.getChildren());
    }

    return Promise.resolve([]);
  }

  private getFileCategories(): FileCategoryItem[] {
    const items: FileCategoryItem[] = [];

    for (const [filePath, results] of this.resultsByFile) {
      const fileName = path.basename(filePath);
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

    return results.gates.map((gate) => new GateTreeItem(gate, filePath, this.workspaceRoot));
  }

  private getGateChildren(gateItem: GateTreeItem): TreeItem[] {
    const gate = gateItem.gate;

    // Check if this is an extended gate result with additional properties
    if (isExtendedGateResult(gate)) {
      if (gate.name === 'architecture' && gate.violations) {
        return this.getArchitectureChildren(gate, gateItem.filePath);
      }

      if (gate.name === 'policy' && gate.violationsByPolicy) {
        return this.getPolicyChildren(gate);
      }
    }

    return this.getGenericGateDetails(gateItem);
  }

  private getArchitectureChildren(gate: ExtendedGateResult, _filePath: string): TreeItem[] {
    const children: TreeItem[] = [];
    const violations = gate.violations || [];
    const byType = gate.violationsByType || {};
    const hasTypeCounts = Object.keys(byType).length > 0;

    if (hasTypeCounts) {
      if (byType.circular && byType.circular > 0) {
        const circularViolations = violations.filter((v) => v.cycle && v.cycle.length > 0);
        children.push(
          new ViolationCategoryItem(
            'Circular Dependencies',
            'ARCH-001',
            byType.circular,
            circularViolations.map(
              (v) =>
                new ArchViolationItem(
                  v,
                  this.workspaceRoot,
                  'Break the cycle by extracting shared code'
                )
            )
          )
        );
      }

      if (byType.layer && byType.layer > 0) {
        const layerViolations = violations.filter(
          (v) => v.rule.includes('layer') || v.rule.includes('boundary')
        );
        children.push(
          new ViolationCategoryItem(
            'Layer Violations',
            'ARCH-003',
            byType.layer,
            layerViolations.map(
              (v) =>
                new ArchViolationItem(
                  v,
                  this.workspaceRoot,
                  'Move import to appropriate layer or adjust boundary'
                )
            )
          )
        );
      }

      if (byType.orphan && byType.orphan > 0) {
        const orphanViolations = violations.filter((v) => v.rule.includes('orphan'));
        children.push(
          new ViolationCategoryItem(
            'Orphaned Modules',
            'ARCH-002',
            byType.orphan,
            orphanViolations.map(
              (v) =>
                new ArchViolationItem(v, this.workspaceRoot, 'Connect module or remove if unused')
            )
          )
        );
      }

      if (byType.other && byType.other > 0) {
        const otherViolations = violations.filter(
          (v) => !v.cycle?.length && !v.rule.includes('layer') && !v.rule.includes('orphan')
        );
        children.push(
          new ViolationCategoryItem(
            'Other Violations',
            'ARCH-004',
            byType.other,
            otherViolations.map(
              (v) => new ArchViolationItem(v, this.workspaceRoot, 'Review and fix the violation')
            )
          )
        );
      }
    } else if (violations.length > 0) {
      const grouped = this.groupViolationsByRule(violations);
      for (const [rule, ruleViolations] of grouped) {
        children.push(
          new ViolationCategoryItem(
            rule,
            'ARCH',
            ruleViolations.length,
            ruleViolations.map(
              (v) => new ArchViolationItem(v, this.workspaceRoot, 'Review and fix the violation')
            )
          )
        );
      }
    }

    if (children.length === 0 && gate.message) {
      children.push(new GateDetailItem(gate.message, 'info'));
    }

    return children;
  }

  private groupViolationsByRule(
    violations: ArchitectureViolation[]
  ): Map<string, ArchitectureViolation[]> {
    const grouped = new Map<string, ArchitectureViolation[]>();
    for (const v of violations) {
      const key = v.rule || 'Unknown';
      const existing = grouped.get(key) || [];
      existing.push(v);
      grouped.set(key, existing);
    }
    return grouped;
  }

  private getPolicyChildren(gate: ExtendedGateResult): TreeItem[] {
    const children: TreeItem[] = [];
    const byPolicy = gate.violationsByPolicy || {};

    for (const [policyName, violations] of Object.entries(byPolicy)) {
      children.push(
        new ViolationCategoryItem(
          policyName,
          'POLICY',
          violations.length,
          violations.map((v) => new PolicyViolationItem(v, this.workspaceRoot))
        )
      );
    }

    if (children.length === 0 && gate.message) {
      children.push(new GateDetailItem(gate.message, 'info'));
    }

    return children;
  }

  private getGenericGateDetails(gateItem: GateTreeItem): GateDetailItem[] {
    const gate = gateItem.gate;

    if (!gate.details || gate.details.length === 0) {
      if (gate.message) {
        return [new GateDetailItem(gate.message, 'info')];
      }
      return [];
    }

    return gate.details.map(
      (detail) =>
        new GateDetailItem(
          detail.message,
          detail.type,
          detail.file ? this.resolveFilePath(detail.file) : undefined,
          detail.line
        )
    );
  }

  private resolveFilePath(filePath: string): string {
    if (path.isAbsolute(filePath)) {
      // Validate absolute paths stay within workspace
      if (
        this.workspaceRoot &&
        !filePath.startsWith(this.workspaceRoot + path.sep) &&
        filePath !== this.workspaceRoot
      ) {
        return filePath; // Return as-is, VS Code will handle security
      }
      return filePath;
    }
    const resolved = path.resolve(this.workspaceRoot, filePath);
    // Validate resolved path stays within workspace
    if (
      this.workspaceRoot &&
      !resolved.startsWith(this.workspaceRoot + path.sep) &&
      resolved !== this.workspaceRoot
    ) {
      return path.join(this.workspaceRoot, path.basename(filePath));
    }
    return resolved;
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
    public readonly filePath: string,
    private readonly _workspaceRoot: string
  ) {
    const hasChildren = GateTreeItem.hasExpandableContent(gate);
    super(
      gate.name,
      hasChildren ? vscode.TreeItemCollapsibleState.Collapsed : vscode.TreeItemCollapsibleState.None
    );

    this.description = gate.message || this.getStatusDescription(gate.status);
    this.tooltip = this.getTooltip();
    this.iconPath = this.getIcon();
    this.contextValue = 'gate';
  }

  private static hasExpandableContent(gate: GateResult): boolean {
    const extended = gate as ExtendedGateResult;
    if (extended.violations && extended.violations.length > 0) return true;
    if (extended.violationsByPolicy && Object.keys(extended.violationsByPolicy).length > 0)
      return true;
    if (gate.details && gate.details.length > 0) return true;
    return false;
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

class ViolationCategoryItem extends vscode.TreeItem {
  private children: TreeItem[];

  constructor(label: string, code: string, count: number, children: TreeItem[]) {
    super(label, vscode.TreeItemCollapsibleState.Collapsed);
    this.children = children;
    this.description = `${count} violation${count !== 1 ? 's' : ''}`;
    this.tooltip = `${label} (${code}): ${count} violation${count !== 1 ? 's' : ''}`;
    this.iconPath = new vscode.ThemeIcon(
      'symbol-folder',
      new vscode.ThemeColor('testing.iconFailed')
    );
    this.contextValue = 'violationCategory';
  }

  getChildren(): TreeItem[] {
    return this.children;
  }
}

class ArchViolationItem extends vscode.TreeItem {
  constructor(violation: ArchitectureViolation, workspaceRoot: string, suggestion: string) {
    const label = violation.cycle
      ? `Cycle: ${violation.cycle.slice(0, 3).join(' → ')}${violation.cycle.length > 3 ? '...' : ''}`
      : `${path.basename(violation.from)} → ${path.basename(violation.to)}`;

    super(label, vscode.TreeItemCollapsibleState.None);

    this.description = violation.rule;
    this.tooltip = this.buildTooltip(violation, suggestion);
    this.iconPath = this.getIcon(violation.severity);
    this.contextValue = 'archViolation';

    const absolutePath = path.isAbsolute(violation.from)
      ? violation.from
      : path.join(workspaceRoot, violation.from);

    this.command = {
      command: 'vscode.open',
      title: 'Open File',
      arguments: [vscode.Uri.file(absolutePath)],
    };
  }

  private buildTooltip(violation: ArchitectureViolation, suggestion: string): string {
    const lines = [`Rule: ${violation.rule}`, `From: ${violation.from}`, `To: ${violation.to}`];

    if (violation.cycle) {
      lines.push(`Cycle: ${violation.cycle.join(' → ')}`);
    }

    lines.push('', `Suggestion: ${suggestion}`);

    return lines.join('\n');
  }

  private getIcon(severity: string): vscode.ThemeIcon {
    if (severity === 'error') {
      return new vscode.ThemeIcon('error', new vscode.ThemeColor('testing.iconFailed'));
    }
    if (severity === 'warn' || severity === 'warning') {
      return new vscode.ThemeIcon(
        'warning',
        new vscode.ThemeColor('problemsWarningIcon.foreground')
      );
    }
    return new vscode.ThemeIcon('info', new vscode.ThemeColor('problemsInfoIcon.foreground'));
  }
}

class PolicyViolationItem extends vscode.TreeItem {
  constructor(violation: PolicyViolation, workspaceRoot: string) {
    super(violation.message, vscode.TreeItemCollapsibleState.None);

    this.description = violation.rule;
    this.tooltip = this.buildTooltip(violation);
    this.iconPath = this.getIcon(violation.severity);
    this.contextValue = 'policyViolation';

    if (violation.path) {
      const absolutePath = path.isAbsolute(violation.path)
        ? violation.path
        : path.join(workspaceRoot, violation.path);

      this.command = {
        command: 'vscode.open',
        title: 'Open File',
        arguments: [vscode.Uri.file(absolutePath)],
      };
    }
  }

  private buildTooltip(violation: PolicyViolation): string {
    const lines = [
      `Rule: ${violation.rule}`,
      `Severity: ${violation.severity}`,
      `Message: ${violation.message}`,
    ];

    if (violation.policy) {
      lines.push(`Policy: ${violation.policy}`);
    }

    if (violation.path) {
      lines.push(`File: ${violation.path}`);
    }

    return lines.join('\n');
  }

  private getIcon(severity: 'error' | 'warning' | 'info'): vscode.ThemeIcon {
    switch (severity) {
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

class GateDetailItem extends vscode.TreeItem {
  constructor(message: string, type: 'error' | 'warning' | 'info', file?: string, line?: number) {
    super(message, vscode.TreeItemCollapsibleState.None);

    this.iconPath = this.getIcon(type);

    if (file && line) {
      this.description = `${path.basename(file)}:${line}`;
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
    } else if (file) {
      this.description = path.basename(file);
      this.command = {
        command: 'vscode.open',
        title: 'Open File',
        arguments: [vscode.Uri.file(file)],
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
