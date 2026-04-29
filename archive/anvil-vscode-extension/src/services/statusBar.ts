import * as vscode from 'vscode';

export type StatusBarState =
  | 'idle'
  | 'validating'
  | 'running-gates'
  | 'success'
  | 'error'
  | 'warning';

export class StatusBarManager implements vscode.Disposable {
  public readonly statusBarItem: vscode.StatusBarItem;
  private currentState: StatusBarState = 'idle';
  private successTimeout: NodeJS.Timeout | undefined;

  constructor() {
    this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    this.statusBarItem.command = 'anvil.showOutput';
    this.setIdle();
  }

  show(): void {
    const config = vscode.workspace.getConfiguration('anvil');
    if (config.get<boolean>('showStatusBar', true)) {
      this.statusBarItem.show();
    }
  }

  hide(): void {
    this.statusBarItem.hide();
  }

  setIdle(): void {
    this.clearTimeout();
    this.currentState = 'idle';
    this.statusBarItem.text = '$(shield) Anvil';
    this.statusBarItem.tooltip = 'Anvil - Click to show output';
    this.statusBarItem.backgroundColor = undefined;
  }

  setValidating(fileName?: string): void {
    this.clearTimeout();
    this.currentState = 'validating';
    this.statusBarItem.text = '$(loading~spin) Anvil: Validating...';
    this.statusBarItem.tooltip = fileName ? `Validating ${fileName}` : 'Validating plan...';
    this.statusBarItem.backgroundColor = undefined;
  }

  setRunningGates(fileName?: string): void {
    this.clearTimeout();
    this.currentState = 'running-gates';
    this.statusBarItem.text = '$(loading~spin) Anvil: Running gates...';
    this.statusBarItem.tooltip = fileName
      ? `Running quality gates on ${fileName}`
      : 'Running quality gates...';
    this.statusBarItem.backgroundColor = undefined;
  }

  setSuccess(message?: string): void {
    this.clearTimeout();
    this.currentState = 'success';
    this.statusBarItem.text = '$(check) Anvil: Passed';
    this.statusBarItem.tooltip = message || 'All checks passed';
    this.statusBarItem.backgroundColor = undefined;

    // Reset to idle after 5 seconds
    this.successTimeout = setTimeout(() => {
      this.setIdle();
    }, 5000);
  }

  setError(message?: string): void {
    this.clearTimeout();
    this.currentState = 'error';
    this.statusBarItem.text = '$(error) Anvil: Failed';
    this.statusBarItem.tooltip = message || 'Validation or gate check failed';
    this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
  }

  setWarning(message?: string): void {
    this.clearTimeout();
    this.currentState = 'warning';
    this.statusBarItem.text = '$(warning) Anvil: Warning';
    this.statusBarItem.tooltip = message || 'Validation passed with warnings';
    this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
  }

  getState(): StatusBarState {
    return this.currentState;
  }

  private clearTimeout(): void {
    if (this.successTimeout) {
      clearTimeout(this.successTimeout);
      this.successTimeout = undefined;
    }
  }

  dispose(): void {
    this.clearTimeout();
    this.statusBarItem.dispose();
  }
}
