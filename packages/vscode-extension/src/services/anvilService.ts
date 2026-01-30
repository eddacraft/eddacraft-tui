import * as vscode from 'vscode';
import * as cp from 'node:child_process';
import * as path from 'node:path';

export interface ValidationResult {
  success: boolean;
  planId?: string;
  format?: string;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

export interface ValidationError {
  message: string;
  path?: string;
  line?: number;
  column?: number;
}

export interface ValidationWarning {
  message: string;
  path?: string;
  line?: number;
  column?: number;
}

export interface GateResult {
  name: string;
  status: 'passed' | 'failed' | 'skipped' | 'error';
  message?: string;
  duration?: number;
  details?: GateDetail[];
}

export interface GateDetail {
  type: 'error' | 'warning' | 'info';
  message: string;
  file?: string;
  line?: number;
  column?: number;
}

export interface GateResults {
  success: boolean;
  gates: GateResult[];
  timestamp: string;
  duration: number;
}

const DEFAULT_COMMAND_TIMEOUT_MS = 60000;
const GATE_COMMAND_TIMEOUT_MS = 120000;

export interface CommandOptions {
  timeout?: number;
  token?: vscode.CancellationToken;
}

export class AnvilService {
  private outputChannel: vscode.OutputChannel;
  private lastValidationResults: Map<string, ValidationResult> = new Map();
  private lastGateResults: Map<string, GateResults> = new Map();

  constructor(_context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel) {
    this.outputChannel = outputChannel;
  }

  getOutputChannel(): vscode.OutputChannel {
    return this.outputChannel;
  }

  getLastValidationResult(uri: string): ValidationResult | undefined {
    return this.lastValidationResults.get(uri);
  }

  getLastGateResults(uri: string): GateResults | undefined {
    return this.lastGateResults.get(uri);
  }

  clearCacheForUri(uri: string): void {
    this.lastValidationResults.delete(uri);
    this.lastGateResults.delete(uri);
  }

  async validate(filePath: string, token?: vscode.CancellationToken): Promise<ValidationResult> {
    this.outputChannel.appendLine(`\n[${new Date().toISOString()}] Validating: ${filePath}`);

    try {
      const { command, baseArgs } = this.getCliCommand();
      const workspaceFolder = this.getWorkspaceFolder(filePath);

      const result = await this.executeCommand(
        command,
        [...baseArgs, 'validate', filePath, '--json'],
        workspaceFolder,
        { timeout: DEFAULT_COMMAND_TIMEOUT_MS, token }
      );

      const validationResult = this.parseValidationResult(result);
      this.lastValidationResults.set(filePath, validationResult);

      if (validationResult.success) {
        this.outputChannel.appendLine(
          `  Validation passed (Plan ID: ${validationResult.planId || 'N/A'})`
        );
      } else {
        this.outputChannel.appendLine(
          `  Validation failed: ${validationResult.errors.length} error(s)`
        );
        validationResult.errors.forEach((err) => {
          this.outputChannel.appendLine(`    - ${err.message}`);
        });
      }

      return validationResult;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      this.outputChannel.appendLine(`  Error: ${errorMessage}`);

      const result: ValidationResult = {
        success: false,
        errors: [{ message: errorMessage }],
        warnings: [],
      };
      this.lastValidationResults.set(filePath, result);
      return result;
    }
  }

  async runGates(filePath: string, token?: vscode.CancellationToken): Promise<GateResults> {
    this.outputChannel.appendLine(`\n[${new Date().toISOString()}] Running gates: ${filePath}`);

    const startTime = Date.now();

    try {
      const { command, baseArgs } = this.getCliCommand();
      const workspaceFolder = this.getWorkspaceFolder(filePath);
      const config = vscode.workspace.getConfiguration('anvil');

      const args = [...baseArgs, 'gate', filePath, '--json'];

      const skipGates = config.get<string[]>('gates.skipInDevelopment', []);
      if (skipGates.length > 0) {
        args.push('--skip', skipGates.join(','));
      }

      const result = await this.executeCommand(command, args, workspaceFolder, {
        timeout: GATE_COMMAND_TIMEOUT_MS,
        token,
      });

      const gateResults = this.parseGateResults(result, startTime);
      this.lastGateResults.set(filePath, gateResults);

      this.outputChannel.appendLine(`  Gates completed in ${gateResults.duration}ms`);
      gateResults.gates.forEach((gate) => {
        const icon = gate.status === 'passed' ? '  ' : gate.status === 'failed' ? '  ' : '  ';
        this.outputChannel.appendLine(`    ${icon} ${gate.name}: ${gate.status}`);
      });

      return gateResults;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      this.outputChannel.appendLine(`  Error: ${errorMessage}`);

      const result: GateResults = {
        success: false,
        gates: [
          {
            name: 'execution',
            status: 'error',
            message: errorMessage,
          },
        ],
        timestamp: new Date().toISOString(),
        duration: Date.now() - startTime,
      };
      this.lastGateResults.set(filePath, result);
      return result;
    }
  }

  async exportPlan(
    filePath: string,
    format: string,
    outputPath?: string,
    token?: vscode.CancellationToken
  ): Promise<{ success: boolean; outputPath?: string; error?: string }> {
    this.outputChannel.appendLine(
      `\n[${new Date().toISOString()}] Exporting: ${filePath} to ${format}`
    );

    try {
      const { command, baseArgs } = this.getCliCommand();
      const workspaceFolder = this.getWorkspaceFolder(filePath);

      const args = [...baseArgs, 'export', filePath, '--to', format];
      if (outputPath) {
        args.push('--output', outputPath);
      }

      await this.executeCommand(command, args, workspaceFolder, {
        timeout: DEFAULT_COMMAND_TIMEOUT_MS,
        token,
      });

      this.outputChannel.appendLine(`  Export successful`);

      return {
        success: true,
        outputPath: outputPath || this.inferOutputPath(filePath, format),
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      this.outputChannel.appendLine(`  Error: ${errorMessage}`);

      return {
        success: false,
        error: errorMessage,
      };
    }
  }

  async detectFormat(
    filePath: string,
    token?: vscode.CancellationToken
  ): Promise<string | undefined> {
    try {
      const { command, baseArgs } = this.getCliCommand();
      const workspaceFolder = this.getWorkspaceFolder(filePath);

      const result = await this.executeCommand(
        command,
        [...baseArgs, 'validate', filePath, '--json'],
        workspaceFolder,
        { timeout: DEFAULT_COMMAND_TIMEOUT_MS, token }
      );

      const parsed = JSON.parse(result);
      return parsed.format;
    } catch {
      return undefined;
    }
  }

  private getCliCommand(): { command: string; baseArgs: string[] } {
    const config = vscode.workspace.getConfiguration('anvil');
    const customPath = config.get<string>('cli.path', '');

    if (customPath) {
      if (!this.isValidCliPath(customPath)) {
        this.outputChannel.appendLine(
          `  Warning: Invalid CLI path "${customPath}", falling back to npx`
        );
        return { command: 'npx', baseArgs: ['anvil'] };
      }
      return { command: customPath, baseArgs: [] };
    }

    return { command: 'npx', baseArgs: ['anvil'] };
  }

  private isValidCliPath(cliPath: string): boolean {
    // Use dynamic imports to avoid bundling issues in VS Code extension
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const fs = require('node:fs') as typeof import('fs');
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const nodePath = require('node:path') as typeof import('path');

    if (!fs.existsSync(cliPath)) {
      return false;
    }

    const stats = fs.statSync(cliPath);
    if (!stats.isFile()) {
      return false;
    }

    const basename = nodePath.basename(cliPath).toLowerCase();
    const validNames = ['anvil', 'anvil.js', 'anvil.cmd', 'anvil.exe'];
    if (!validNames.some((name) => basename === name || basename.startsWith('anvil'))) {
      return false;
    }

    return true;
  }

  private getWorkspaceFolder(filePath: string): string {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(filePath));
    return workspaceFolder?.uri.fsPath || path.dirname(filePath);
  }

  private executeCommand(
    command: string,
    args: string[],
    cwd: string,
    options: CommandOptions = {}
  ): Promise<string> {
    const { timeout = DEFAULT_COMMAND_TIMEOUT_MS, token } = options;

    return new Promise((resolve, reject) => {
      this.outputChannel.appendLine(`  > ${command} ${args.join(' ')}`);

      const child = cp.spawn(command, args, {
        cwd,
        stdio: ['pipe', 'pipe', 'pipe'],
        shell: false,
      });

      let stdout = '';
      let stderr = '';
      let killed = false;

      const timeoutId = setTimeout(() => {
        killed = true;
        child.kill('SIGTERM');
        reject(new Error(`Command timed out after ${timeout}ms`));
      }, timeout);

      const cancellationListener = token?.onCancellationRequested(() => {
        killed = true;
        child.kill('SIGTERM');
        reject(new Error('Operation cancelled'));
      });

      const cleanup = () => {
        clearTimeout(timeoutId);
        cancellationListener?.dispose();
      };

      child.stdout.on('data', (data: Buffer) => {
        stdout += data.toString('utf8');
      });

      child.stderr.on('data', (data: Buffer) => {
        stderr += data.toString('utf8');
      });

      child.on('error', (error: Error) => {
        cleanup();
        reject(new Error(`Failed to spawn command: ${error.message}`));
      });

      child.on('close', (code: number | null) => {
        cleanup();

        if (killed) {
          return;
        }

        if (code !== 0) {
          if (stderr) {
            try {
              const errorJson = JSON.parse(stderr);
              if (errorJson.error) {
                reject(new Error(errorJson.error));
                return;
              }
            } catch {
              // Not JSON, use raw error
            }
          }
          reject(new Error(stderr || `Command exited with code ${code}`));
          return;
        }

        resolve(stdout);
      });
    });
  }

  private parseValidationResult(output: string): ValidationResult {
    try {
      const parsed = JSON.parse(output);

      return {
        success: parsed.success ?? parsed.valid ?? false,
        planId: parsed.planId ?? parsed.plan_id,
        format: parsed.format,
        errors: (parsed.errors || []).map(
          (err: { message?: string; path?: string; line?: number; column?: number }) => ({
            message: err.message || String(err),
            path: err.path,
            line: err.line,
            column: err.column,
          })
        ),
        warnings: (parsed.warnings || []).map(
          (warn: { message?: string; path?: string; line?: number; column?: number }) => ({
            message: warn.message || String(warn),
            path: warn.path,
            line: warn.line,
            column: warn.column,
          })
        ),
      };
    } catch {
      // If not JSON, treat as plain text result
      const success =
        output.toLowerCase().includes('valid') && !output.toLowerCase().includes('invalid');

      return {
        success,
        errors: success ? [] : [{ message: output.trim() }],
        warnings: [],
      };
    }
  }

  private parseGateResults(output: string, startTime: number): GateResults {
    try {
      const parsed = JSON.parse(output);

      return {
        success: parsed.success ?? false,
        gates: (parsed.gates || parsed.results || []).map((gate: Record<string, unknown>) => ({
          ...gate,
          name: (gate.name as string) || 'unknown',
          status: this.normalizeGateStatus(gate.status as string | undefined),
          message: gate.message as string | undefined,
          duration: gate.duration as number | undefined,
          details: gate.details as GateDetail[] | undefined,
        })),
        timestamp: parsed.timestamp || new Date().toISOString(),
        duration: parsed.duration || Date.now() - startTime,
      };
    } catch {
      // If not JSON, treat as plain text result
      const success =
        output.toLowerCase().includes('passed') || output.toLowerCase().includes('success');

      return {
        success,
        gates: [
          {
            name: 'gate',
            status: success ? 'passed' : 'failed',
            message: output.trim(),
          },
        ],
        timestamp: new Date().toISOString(),
        duration: Date.now() - startTime,
      };
    }
  }

  private normalizeGateStatus(
    status: string | undefined
  ): 'passed' | 'failed' | 'skipped' | 'error' {
    if (!status) return 'error';

    const normalised = status.toLowerCase();
    if (normalised === 'pass' || normalised === 'passed' || normalised === 'success') {
      return 'passed';
    }
    if (normalised === 'fail' || normalised === 'failed' || normalised === 'failure') {
      return 'failed';
    }
    if (normalised === 'skip' || normalised === 'skipped') {
      return 'skipped';
    }
    return 'error';
  }

  private inferOutputPath(inputPath: string, format: string): string {
    const dir = path.dirname(inputPath);
    const baseName = path.basename(inputPath, path.extname(inputPath));

    const extensions: Record<string, string> = {
      aps: '.aps.json',
      speckit: '.plan.md',
      bmad: '.prd.md',
      generic: '.md',
      json: '.json',
    };

    const ext = extensions[format] || `.${format}`;
    return path.join(dir, `${baseName}${ext}`);
  }
}
